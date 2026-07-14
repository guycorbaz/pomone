//! Home screen (counts + user-manual button) wiring — extracted from `main.rs` (story 0.4). Shared
//! helpers stay reachable through `crate::…` re-exports.

use std::cell::RefCell;
use std::rc::Rc;

use slint::{ComponentHandle, SharedString};

use std::path::PathBuf;

use crate::{refresh_bed_usage, UiState};

use crate::generated::MainWindow;

/// Register the home callbacks on the window. Called once
/// from `main()`; standard wiring shape — see `wiring/mod.rs`.
pub(crate) fn wire_home(window: &MainWindow, state: &Rc<RefCell<UiState>>) {
    // --- Home navigation (sidebar) — refresh counts on entry ---
    {
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_navigate_home(move || {
            let Some(window) = weak.upgrade() else {
                return;
            };
            let s = state.borrow();
            refresh_bed_usage(&window, &s.app, &s.runtime);
        });
    }
    {
        // "Imprimer ma semaine (brut)" (story 1.4): export the rough plain-text
        // weekly print of this week's tasks next to the database, then open it
        // with the system viewer. The clock lives here at the UI layer.
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_print_week(move || {
            let Some(window) = weak.upgrade() else {
                return;
            };
            let s = state.borrow();
            let today = chrono::Local::now().date_naive();
            let dir = week_print_dir(s.app.config());
            let result = s.runtime.block_on(async {
                pomone_app::printdoc::export_week_sheet(s.app.repo(), s.app.i18n(), today, &dir)
                    .await
            });
            let (key, is_err) = match result {
                Ok(path) => {
                    if let Err(e) = open::that_detached(&path) {
                        tracing::warn!(error = %e, path = %path.display(), "failed to open weekly print");
                        ("status-week-print-failed", true)
                    } else {
                        tracing::info!(path = %path.display(), "wrote + opened weekly print");
                        ("status-week-print-written", false)
                    }
                }
                Err(e) => {
                    tracing::error!(error = %e, "failed to export weekly print");
                    ("status-week-print-failed", true)
                }
            };
            window.set_status_text(SharedString::from(s.app.i18n().t(key)));
            window.set_status_is_error(is_err);
        });
    }
    {
        // Open the bundled user manual PDF. find_manual_path tries the standard
        // install locations + a dev-mode fallback; the outcome is surfaced in
        // the global status banner so a missing PDF isn't a silent no-op (#66).
        let state = Rc::clone(state);
        let weak = window.as_weak();
        window.on_open_manual(move || {
            let Some(window) = weak.upgrade() else {
                return;
            };
            let s = state.borrow();
            let i18n = s.app.i18n();
            let (key, is_err) = if let Some(path) = find_manual_path() {
                if let Err(e) = open::that_detached(&path) {
                    tracing::warn!(error = %e, path = %path.display(), "failed to open manual");
                    ("status-manual-open-failed", true)
                } else {
                    tracing::info!(path = %path.display(), "opened user manual");
                    ("status-manual-opened", false)
                }
            } else {
                tracing::warn!("user manual PDF not found in any standard location");
                ("status-manual-not-found", true)
            };
            window.set_status_text(SharedString::from(i18n.t(key)));
            window.set_status_is_error(is_err);
        });
    }
}

/// The directory the weekly print is written into — next to the SQLite
/// database (the data dir). Falls back to the current directory when the
/// backend is MariaDB or has no parent.
fn week_print_dir(config: &pomone_app::AppConfig) -> PathBuf {
    match &config.backend {
        pomone_app::BackendConfig::Sqlite { path } => path
            .parent()
            .map_or_else(|| PathBuf::from("."), std::path::Path::to_path_buf),
        pomone_app::BackendConfig::Mariadb { .. } => PathBuf::from("."),
    }
}

/// Locate the bundled user manual PDF at runtime. Returns the first
/// candidate that exists; `None` if none of them do.
///
/// Layout per package format:
/// - Linux `.deb`: `/usr/bin/pomone` + manual at `/usr/share/doc/pomone/manuel.pdf`
///   → reachable as `<exe_dir>/../share/doc/pomone/manuel.pdf`.
/// - Linux AppImage: `$APPDIR/usr/share/doc/pomone/manuel.pdf`.
/// - macOS `.app`: `Contents/MacOS/pomone` + `Contents/Resources/manuel.pdf`
///   → `<exe_dir>/../Resources/manuel.pdf`.
/// - Windows: PDF placed next to the binary.
/// - Dev (`cargo run`): the workspace's `docs/manual/manuel.pdf`, resolved
///   through `CARGO_MANIFEST_DIR` at compile time.
fn find_manual_path() -> Option<PathBuf> {
    if let Ok(appdir) = std::env::var("APPDIR") {
        let p = PathBuf::from(appdir).join("usr/share/doc/pomone/manuel.pdf");
        if p.exists() {
            return Some(p);
        }
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            for rel in [
                "../share/doc/pomone/manuel.pdf", // Linux .deb / AppImage layout
                "../Resources/manuel.pdf",        // macOS .app
                "manuel.pdf",                     // Windows / portable
            ] {
                let candidate = exe_dir.join(rel);
                if candidate.exists() {
                    return Some(candidate);
                }
            }
        }
    }

    // Dev fallback (cargo run): workspace_root/docs/manual/manuel.pdf
    let dev = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/manual/manuel.pdf");
    if dev.exists() {
        return Some(dev);
    }

    None
}
