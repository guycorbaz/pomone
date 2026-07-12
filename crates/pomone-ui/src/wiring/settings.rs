//! Settings-family wiring: the Settings screen (backend test / save /
//! save-and-migrate, manual backup, public-holiday region, display units)
//! plus the language toggle. Extracted from `main.rs` (story 0.1); shared
//! helpers (`UiState`, refreshes, error rendering) stay in the crate root
//! and are reached through `crate::…`.

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use anyhow::Result;
use fluent::FluentArgs;
use pomone_app::{
    test_backend, AppError, AreaUnit, BackendConfig, Lang, MassUnit, MigrationReport,
};
use pomone_domain::HolidayRegion;
use slint::{ComponentHandle, SharedString};

use crate::generated::MainWindow;
use crate::{
    apply_translations, apply_unit_labels, localize_app_error, refresh_bed_usage, refresh_cultures,
    refresh_locations, refresh_planting_detail, refresh_plantings, refresh_strata,
    refresh_task_calendar, render_form_error, FormError, UiState,
};

/// Register every settings-family callback on the window. Called once from
/// `main()`; each block clones the `Rc` and a weak window handle, upgrading
/// the weak at call time (the standard wiring shape — see `wiring/mod.rs`).
#[allow(clippy::too_many_lines)]
pub(crate) fn wire_settings(window: &MainWindow, state: &Rc<RefCell<UiState>>) {
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_toggle_language(move || {
            let Some(window) = weak.upgrade() else {
                return;
            };
            let mut s = state.borrow_mut();
            let next = match s.app.i18n().lang() {
                Lang::Fr => Lang::En,
                Lang::En => Lang::Fr,
            };
            s.app.set_lang(next);
            apply_translations(&window, &s.app);
        });
    }
    // --- Settings navigation + test / save / save-and-migrate ---
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_navigate_settings(move || {
            let Some(window) = weak.upgrade() else {
                return;
            };
            refresh_settings(&window, &state.borrow());
            window.set_current_page(SharedString::from("settings"));
            window.set_settings_status_text(SharedString::from(""));
            window.set_settings_status_is_error(false);
        });
    }
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_settings_test_backend(
            move |kind, sqlite_path, host, port, user, password, db| {
                let Some(window) = weak.upgrade() else {
                    return;
                };
                let s = state.borrow();
                let form = SettingsFormValues {
                    kind,
                    sqlite_path: sqlite_path.into(),
                    host: host.into(),
                    port: port.into(),
                    user: user.into(),
                    password: password.into(),
                    database: db.into(),
                };
                tracing::info!(?form, "test backend invoked");
                let new_backend = match form.into_backend() {
                    Ok(b) => b,
                    Err(text) => {
                        window.set_settings_status_text(SharedString::from(text));
                        window.set_settings_status_is_error(true);
                        return;
                    }
                };
                match s.runtime.block_on(test_backend(&new_backend)) {
                    Ok(()) => {
                        window.set_settings_status_text(SharedString::from(
                            s.app.i18n().t("settings-test-ok"),
                        ));
                        window.set_settings_status_is_error(false);
                    }
                    Err(e) => {
                        let i18n = s.app.i18n();
                        let mut args = FluentArgs::new();
                        args.set("message", localize_app_error(i18n, &e));
                        window.set_settings_status_text(SharedString::from(
                            i18n.t_args("status-planting-failed", &args),
                        ));
                        window.set_settings_status_is_error(true);
                    }
                }
            },
        );
    }
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_settings_save_backend(
            move |kind, sqlite_path, host, port, user, password, db| {
                let Some(window) = weak.upgrade() else {
                    return;
                };
                let form = SettingsFormValues {
                    kind,
                    sqlite_path: sqlite_path.into(),
                    host: host.into(),
                    port: port.into(),
                    user: user.into(),
                    password: password.into(),
                    database: db.into(),
                };
                tracing::info!(?form, "save backend invoked");
                try_swap_backend(&window, state.clone(), form, false);
            },
        );
    }
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_settings_save_and_migrate(
            move |kind, sqlite_path, host, port, user, password, db| {
                let Some(window) = weak.upgrade() else {
                    return;
                };
                let form = SettingsFormValues {
                    kind,
                    sqlite_path: sqlite_path.into(),
                    host: host.into(),
                    port: port.into(),
                    user: user.into(),
                    password: password.into(),
                    database: db.into(),
                };
                tracing::info!(?form, "save+migrate backend invoked");
                try_swap_backend(&window, state.clone(), form, true);
            },
        );
    }

    // --- Manual backup from the settings page (issue #58) ---
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_settings_backup_now(move || {
            let Some(window) = weak.upgrade() else {
                return;
            };
            let s = state.borrow();
            let i18n = s.app.i18n();
            match s.app.backup_now() {
                Ok(path) => {
                    let mut args = FluentArgs::new();
                    args.set("path", path.display().to_string());
                    window.set_settings_backup_status_text(SharedString::from(
                        i18n.t_args("settings-backup-done", &args),
                    ));
                    window.set_settings_backup_status_is_error(false);
                }
                Err(AppError::Inconsistent(ref code)) if code == "backup_sqlite_only" => {
                    window.set_settings_backup_status_text(SharedString::from(
                        i18n.t("error-backup-sqlite-only"),
                    ));
                    window.set_settings_backup_status_is_error(true);
                }
                Err(e) => {
                    let mut args = FluentArgs::new();
                    args.set("message", localize_app_error(i18n, &e));
                    window.set_settings_backup_status_text(SharedString::from(
                        i18n.t_args("status-planting-failed", &args),
                    ));
                    window.set_settings_backup_status_is_error(true);
                }
            }
        });
    }

    // --- Public-holiday region picker (issue #35) ---
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_settings_holiday_region_changed(move |index| {
            let Some(window) = weak.upgrade() else {
                return;
            };
            let mut s = state.borrow_mut();
            let code = holiday_region_code(index);
            // The combo fires once at startup when apply_translations sets
            // the initial index — don't rewrite the config for a no-op.
            if s.app.config().holiday_region == code {
                return;
            }
            match s.app.set_holiday_region(&code) {
                Ok(()) => {
                    let msg = s.app.i18n().t("status-holiday-region-saved");
                    window.set_settings_status_text(SharedString::from(msg));
                    window.set_settings_status_is_error(false);
                    if let Err(e) = refresh_task_calendar(&window, &mut s) {
                        tracing::error!(error = %e, "failed to refresh calendar after region change");
                    }
                }
                Err(e) => {
                    let (text, is_err) = render_form_error(s.app.i18n(), FormError::Service(e));
                    window.set_settings_status_text(text);
                    window.set_settings_status_is_error(is_err);
                }
            }
        });
    }

    // --- Display-unit pickers (issue #29) ---
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_settings_area_unit_changed(move |index| {
            let Some(window) = weak.upgrade() else {
                return;
            };
            let mut s = state.borrow_mut();
            let unit = area_unit_from_index(index);
            // The combo fires once at startup when apply_translations sets
            // the initial index — don't rewrite the config for a no-op.
            if s.app.area_unit() == unit {
                return;
            }
            match s.app.set_area_unit(unit) {
                Ok(()) => on_units_saved(&window, &mut s),
                Err(e) => {
                    let (text, is_err) = render_form_error(s.app.i18n(), FormError::Service(e));
                    window.set_settings_status_text(text);
                    window.set_settings_status_is_error(is_err);
                }
            }
        });
    }
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_settings_mass_unit_changed(move |index| {
            let Some(window) = weak.upgrade() else {
                return;
            };
            let mut s = state.borrow_mut();
            let unit = mass_unit_from_index(index);
            if s.app.mass_unit() == unit {
                return;
            }
            match s.app.set_mass_unit(unit) {
                Ok(()) => on_units_saved(&window, &mut s),
                Err(e) => {
                    let (text, is_err) = render_form_error(s.app.i18n(), FormError::Service(e));
                    window.set_settings_status_text(text);
                    window.set_settings_status_is_error(is_err);
                }
            }
        });
    }
}

/// Push the active backend onto the Settings header and pre-fill the edit
/// form so the user can tweak it without retyping everything.
pub(crate) fn refresh_settings(window: &MainWindow, state: &UiState) {
    let cfg = state.app.config();
    let value = backend_display(&cfg.backend);
    window.set_settings_current_value(SharedString::from(value));

    match &cfg.backend {
        BackendConfig::Sqlite { path } => {
            window.set_settings_backend_kind_index(0);
            window.set_settings_sqlite_path(SharedString::from(path.display().to_string()));
        }
        BackendConfig::Mariadb { url } => {
            window.set_settings_backend_kind_index(1);
            // Best-effort split of the URL back into structured fields so
            // the user sees something usable. Falls back to leaving fields
            // empty if the URL doesn't match the canonical shape.
            let (host, port, user, password, db) = split_mariadb_url(url);
            window.set_settings_mariadb_host(SharedString::from(host));
            window.set_settings_mariadb_port(SharedString::from(port));
            window.set_settings_mariadb_user(SharedString::from(user));
            window.set_settings_mariadb_password(SharedString::from(password));
            window.set_settings_mariadb_database(SharedString::from(db));
        }
    }
}

/// Human-readable rendering of a backend for the Settings header.
fn backend_display(b: &BackendConfig) -> String {
    match b {
        BackendConfig::Sqlite { path } => format!("SQLite — {}", path.display()),
        BackendConfig::Mariadb { url } => format!("MariaDB — {}", redact_password(url)),
    }
}

/// Replace the password in `mysql://user:pass@host…` with `***` so the
/// banner doesn't leak credentials when the user takes screenshots.
fn redact_password(url: &str) -> String {
    if let Some(scheme_end) = url.find("://") {
        let (scheme, rest) = url.split_at(scheme_end + 3);
        if let Some(at_pos) = rest.find('@') {
            let (creds, tail) = rest.split_at(at_pos);
            if let Some(colon_pos) = creds.find(':') {
                let (user, _) = creds.split_at(colon_pos);
                return format!("{scheme}{user}:***{tail}");
            }
        }
    }
    url.to_owned()
}

/// Best-effort decomposition of a `mysql://user:pass@host:port/db` URL into
/// its five components. Returns empty strings for anything missing.
fn split_mariadb_url(url: &str) -> (String, String, String, String, String) {
    let mut port = "3306".to_owned();
    let mut user = String::new();
    let mut password = String::new();
    let rest = url.strip_prefix("mysql://").unwrap_or(url);
    let (creds, tail) = match rest.find('@') {
        Some(p) => (&rest[..p], &rest[p + 1..]),
        None => ("", rest),
    };
    if !creds.is_empty() {
        if let Some(colon) = creds.find(':') {
            creds[..colon].clone_into(&mut user);
            creds[colon + 1..].clone_into(&mut password);
        } else {
            creds.clone_into(&mut user);
        }
    }
    let (hostport, after) = match tail.find('/') {
        Some(p) => (&tail[..p], &tail[p + 1..]),
        None => (tail, ""),
    };
    let host = if let Some(colon) = hostport.find(':') {
        hostport[colon + 1..].clone_into(&mut port);
        hostport[..colon].to_owned()
    } else {
        hostport.to_owned()
    };
    let db = after.split('?').next().unwrap_or("").to_owned();
    (host, port, user, password, db)
}

/// Snapshot of the Settings form values, captured at the moment a button
/// is clicked. Going through callback args (rather than property reads)
/// dodges any propagation hiccup in the `<=>` chain between MainWindow
/// and the SettingsPage subcomponent.
#[derive(Debug, Clone)]
struct SettingsFormValues {
    kind: i32,
    sqlite_path: String,
    host: String,
    port: String,
    user: String,
    password: String,
    database: String,
}

impl SettingsFormValues {
    fn into_backend(self) -> Result<BackendConfig, String> {
        if self.kind == 0 {
            let trimmed = self.sqlite_path.trim();
            if trimmed.is_empty() {
                return Err("SQLite path is required".to_owned());
            }
            Ok(BackendConfig::Sqlite {
                path: PathBuf::from(trimmed),
            })
        } else {
            let host = self.host.trim().to_owned();
            let port = self.port.trim().to_owned();
            let user = self.user.trim().to_owned();
            let password = self.password;
            let db = self.database.trim().to_owned();
            if host.is_empty() || user.is_empty() || db.is_empty() {
                return Err("MariaDB host, user and database are required".to_owned());
            }
            let port = if port.is_empty() {
                "3306".to_owned()
            } else {
                port
            };
            // sqlx accepts `mysql://user:pass@host:port/db`. Password may
            // contain URL-reserved chars; for v1 we trust the user — a
            // proper percent-encoder is a follow-up if needed.
            let url = if password.is_empty() {
                format!("mysql://{user}@{host}:{port}/{db}")
            } else {
                format!("mysql://{user}:{password}@{host}:{port}/{db}")
            };
            Ok(BackendConfig::Mariadb { url })
        }
    }
}

/// Localized one-liner summarising a [`MigrationReport`].
fn format_migration_report(report: &MigrationReport, i18n: &pomone_app::I18n) -> String {
    fn n(v: usize) -> i64 {
        i64::try_from(v).unwrap_or(i64::MAX)
    }
    let mut args = FluentArgs::new();
    args.set("families", n(report.families));
    args.set("strata", n(report.strata));
    args.set("kinds", n(report.location_kinds));
    args.set("locations", n(report.locations));
    args.set("crops", n(report.crops));
    args.set("varieties", n(report.varieties));
    args.set("plantings", n(report.plantings));
    args.set("harvests", n(report.yearly_harvests));
    args.set("tasktypes", n(report.task_types));
    args.set("taskmethods", n(report.task_methods));
    args.set("taskimplements", n(report.task_implements));
    args.set("taskseries", n(report.task_series));
    args.set("tasks", n(report.tasks));
    args.set("treatments", n(report.treatments));
    i18n.t_args("settings-report", &args)
}

/// Wire the Save / Save+Migrate buttons. Validates the form, calls
/// `App::swap_backend`, refreshes every screen so the new data shows up,
/// and writes a localized status line.
fn try_swap_backend(
    window: &MainWindow,
    state: Rc<RefCell<UiState>>,
    form: SettingsFormValues,
    migrate: bool,
) {
    let new_backend = match form.into_backend() {
        Ok(b) => b,
        Err(text) => {
            window.set_settings_status_text(SharedString::from(text));
            window.set_settings_status_is_error(true);
            return;
        }
    };
    let mut s = state.borrow_mut();
    // Split-borrow: swap_backend needs `&mut app` but the runtime needs to
    // outlive that mutable borrow. Destructuring through reborrow gives the
    // compiler two independent slots from the same `RefMut`.
    let result: Result<MigrationReport, AppError> = {
        let UiState {
            ref runtime,
            ref mut app,
            ..
        } = *s;
        runtime.block_on(async { app.swap_backend(new_backend, migrate).await })
    };
    match result {
        Ok(report) => {
            let i18n = s.app.i18n();
            let backend_text = backend_display(&s.app.config().backend);
            let mut args = FluentArgs::new();
            args.set("backend", backend_text.clone());
            let mut msg = if migrate {
                let report_text = format_migration_report(&report, i18n);
                args.set("report", report_text);
                i18n.t_args("settings-migrate-ok", &args)
            } else {
                i18n.t_args("settings-save-ok", &args)
            };
            // Surface the pre-swap auto-backup path (issue #58) so the user
            // knows where the safety net lives.
            if let Some(backup) = &report.pre_swap_backup {
                let mut bargs = FluentArgs::new();
                bargs.set("path", backup.display().to_string());
                msg.push('\n');
                msg.push_str(&i18n.t_args("settings-backup-note", &bargs));
            }

            // Every list-based screen now points at a different repo; reload
            // them all. A failed reload is surfaced in the status (issue #69)
            // instead of leaving a silently empty screen.
            refresh_bed_usage(window, &s.app, &s.runtime);
            let mut failed_screens: Vec<&str> = Vec::new();
            // (nav key, result) — the key doubles as the localized screen
            // name in the warning below.
            let reloads: [(&str, Result<()>); 5] = [
                ("nav-plantings", refresh_plantings(window, &mut s)),
                ("nav-cultures", refresh_cultures(window, &mut s)),
                ("nav-locations", refresh_locations(window, &mut s)),
                ("nav-tasks", refresh_task_calendar(window, &mut s)),
                ("nav-strata", refresh_strata(window, &mut s)),
            ];
            for (screen, result) in reloads {
                if let Err(e) = result {
                    tracing::error!(error = %e, screen, "failed to refresh after backend swap");
                    failed_screens.push(screen);
                }
            }
            refresh_settings(window, &s);
            if !failed_screens.is_empty() {
                let i18n = s.app.i18n();
                let screens = failed_screens
                    .iter()
                    .map(|key| i18n.t(key))
                    .collect::<Vec<_>>()
                    .join(", ");
                let mut wargs = FluentArgs::new();
                wargs.set("screens", screens);
                msg.push('\n');
                msg.push_str(&i18n.t_args("settings-refresh-warning", &wargs));
            }
            window.set_settings_status_text(SharedString::from(msg));
            window.set_settings_status_is_error(false);
        }
        Err(e) => {
            let i18n = s.app.i18n();
            let mut args = FluentArgs::new();
            args.set("message", localize_app_error(i18n, &e));
            window.set_settings_status_text(SharedString::from(
                i18n.t_args("status-planting-failed", &args),
            ));
            window.set_settings_status_is_error(true);
        }
    }
}

/// After a display-unit change: confirm in the status line, refresh the
/// labels embedding the unit, and repaint every view that formats areas
/// or masses (plantings, locations, and the open planting detail).
fn on_units_saved(window: &MainWindow, state: &mut UiState) {
    let msg = state.app.i18n().t("status-units-saved");
    window.set_settings_status_text(SharedString::from(msg));
    window.set_settings_status_is_error(false);
    apply_unit_labels(window, &state.app);
    if let Err(e) = refresh_plantings(window, state) {
        tracing::error!(error = %e, "failed to refresh plantings after unit change");
    }
    if let Err(e) = refresh_locations(window, state) {
        tracing::error!(error = %e, "failed to refresh locations after unit change");
    }
    if !state.detail_planting_id.is_empty() {
        let pid = state.detail_planting_id.clone();
        if let Err(e) = refresh_planting_detail(window, state, &pid) {
            tracing::error!(error = %e, "failed to refresh planting detail after unit change");
        }
    }
}

/// Area unit for a combo index, defaulting to m² when out of range.
fn area_unit_from_index(index: i32) -> AreaUnit {
    usize::try_from(index)
        .ok()
        .and_then(|i| AreaUnit::ALL.get(i))
        .copied()
        .unwrap_or_default()
}

/// Mass unit for a combo index, defaulting to kg when out of range.
fn mass_unit_from_index(index: i32) -> MassUnit {
    usize::try_from(index)
        .ok()
        .and_then(|i| MassUnit::ALL.get(i))
        .copied()
        .unwrap_or_default()
}

/// Persisted code for a combo index ("" = none). Inverse of
/// `holiday_region_index` (in `main.rs`).
fn holiday_region_code(index: i32) -> String {
    usize::try_from(index)
        .ok()
        .and_then(|i| i.checked_sub(1))
        .and_then(|i| HolidayRegion::ALL.get(i))
        .map_or_else(String::new, |r| r.code().to_owned())
}
