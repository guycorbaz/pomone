//! Fluent catalogue application — every static label/tooltip set at once.
//! Extracted from `main.rs` (story 0.4); re-exported from the crate root so
//! `crate::…` paths keep working everywhere.

use crate::{usize_to_i32, AXIS_DAYS};
use chrono::{Datelike, Local};
use fluent::FluentArgs;
use pomone_app::{planting_status_key, App, AreaUnit, MassUnit};
use pomone_domain::{HolidayRegion, PlantingStatus};
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};

use crate::generated::{MainWindow, TooltipCatalog};

/// Refresh every string the UI displays based on the active language.
// Five panes' worth of labels in one place keeps the flow easy to follow;
// clippy's 100-line cap is too tight for a UI translation broadcast.
#[allow(clippy::too_many_lines)]
pub(crate) fn apply_translations(window: &MainWindow, app: &App) {
    let i18n = app.i18n();
    window.set_title_text(SharedString::from("Pomone"));
    window.set_welcome_text(SharedString::from(i18n.t("welcome-summary")));
    window.set_print_week_text(SharedString::from(i18n.t("home-print-week")));
    window.set_version_text(SharedString::from(format!(
        "v{}",
        env!("CARGO_PKG_VERSION")
    )));
    window.set_bed_usage_legend_open(SharedString::from(i18n.t("bed-usage-legend-open")));
    window.set_bed_usage_legend_sheltered(SharedString::from(i18n.t("bed-usage-legend-sheltered")));
    window.set_section_season_text(SharedString::from(i18n.t("section-season")));
    window.set_empty_season_text(SharedString::from(i18n.t("empty-season")));
    window.set_section_gantt_text(SharedString::from(i18n.t("section-gantt")));
    window.set_language_button_text(SharedString::from(i18n.t("button-switch-language")));
    window.set_current_language_tag(SharedString::from(i18n.lang().tag()));

    // Localized 12 month abbreviations for the Gantt header. Index 0 = January.
    // Uses `gantt-month-N` (short form, e.g. "Janv.") rather than the full
    // `month-N` so the 80px-wide column doesn't overflow on long names.
    let month_labels: Vec<SharedString> = (1..=12)
        .map(|m| SharedString::from(i18n.t(format!("gantt-month-{m}").as_str())))
        .collect();
    window.set_gantt_month_labels(ModelRc::new(VecModel::from(month_labels)));

    // Today's day-of-year for the Gantt's vertical "today" line. Refreshed
    // here so a language toggle (rare but possible mid-session) also
    // re-snaps it; an app left open across midnight would still need a
    // separate timer, but that's a v1.x problem.
    // Clamp to the 365-wide axis so a leap-year Dec 31 (ordinal 366) lands on
    // the last column instead of overshooting (#67).
    let today_doy = usize_to_i32(Local::now().date_naive().ordinal() as usize).min(AXIS_DAYS);
    window.set_gantt_today_day(today_doy);

    // Sidebar nav
    window.set_nav_group_planning_text(SharedString::from(i18n.t("nav-group-planning")));
    window.set_nav_group_catalog_text(SharedString::from(i18n.t("nav-group-catalog")));
    window.set_nav_group_system_text(SharedString::from(i18n.t("nav-group-system")));
    window.set_nav_home_text(SharedString::from(i18n.t("nav-home")));
    window.set_nav_plantings_text(SharedString::from(i18n.t("nav-plantings")));
    window.set_nav_cultures_text(SharedString::from(i18n.t("nav-cultures")));
    window.set_nav_locations_text(SharedString::from(i18n.t("nav-locations")));
    window.set_nav_strata_text(SharedString::from(i18n.t("nav-strata")));
    window.set_nav_families_text(SharedString::from(i18n.t("nav-families")));
    window.set_nav_crop_map_text(SharedString::from(i18n.t("nav-crop-map")));
    window.set_nav_help_text(SharedString::from(i18n.t("nav-help")));

    // Contextual-help tooltips (#39). The texts live in a Slint global
    // (`TooltipCatalog`) so pages read them directly — no per-string
    // window → page property forwarding.
    let tips = window.global::<TooltipCatalog>();
    let t = |key: &str| SharedString::from(i18n.t(key));
    tips.set_nav_home(t("tooltip-nav-home"));
    tips.set_nav_plantings(t("tooltip-nav-plantings"));
    tips.set_nav_tasks(t("tooltip-nav-tasks"));
    tips.set_nav_agenda(t("tooltip-nav-agenda"));
    tips.set_nav_crop_map(t("tooltip-nav-crop-map"));
    tips.set_nav_cultures(t("tooltip-nav-cultures"));
    tips.set_nav_locations(t("tooltip-nav-locations"));
    tips.set_nav_strata(t("tooltip-nav-strata"));
    tips.set_nav_families(t("tooltip-nav-families"));
    tips.set_nav_settings(t("tooltip-nav-settings"));
    tips.set_nav_help(t("tooltip-nav-help"));
    tips.set_nav_language(t("tooltip-nav-language"));
    tips.set_planting_variety(t("tooltip-planting-variety"));
    tips.set_planting_location(t("tooltip-planting-location"));
    tips.set_planting_strata(t("tooltip-planting-strata"));
    tips.set_planting_method(t("tooltip-planting-method"));
    tips.set_planting_sown_on(t("tooltip-planting-sown-on"));
    tips.set_planting_established_on(t("tooltip-planting-established-on"));
    tips.set_planting_removal_on(t("tooltip-planting-removal-on"));
    // tooltip-planting-area is set by `apply_unit_labels` (issue #29).
    tips.set_planting_count(t("tooltip-planting-count"));
    tips.set_planting_create(t("tooltip-planting-create"));
    tips.set_task_type(t("tooltip-task-type"));
    tips.set_task_planting(t("tooltip-task-planting"));
    tips.set_task_planned_on(t("tooltip-task-planned-on"));
    tips.set_task_completed(t("tooltip-task-completed"));
    tips.set_task_notes(t("tooltip-task-notes"));
    tips.set_task_recurring(t("tooltip-task-recurring"));
    tips.set_task_recurrence_interval(t("tooltip-task-recurrence-interval"));
    tips.set_task_recurrence_unit(t("tooltip-task-recurrence-unit"));
    tips.set_task_recurrence_end_on(t("tooltip-task-recurrence-end-on"));
    tips.set_calendar_prev(t("tooltip-calendar-prev"));
    tips.set_calendar_next(t("tooltip-calendar-next"));
    tips.set_calendar_today(t("tooltip-calendar-today"));
    tips.set_calendar_new_task(t("tooltip-calendar-new-task"));
    tips.set_calendar_manage_types(t("tooltip-calendar-manage-types"));
    tips.set_calendar_filter_chip(t("tooltip-calendar-filter-chip"));
    tips.set_calendar_filter_all(t("tooltip-calendar-filter-all"));
    tips.set_calendar_milestones(t("tooltip-calendar-milestones"));

    // Crop Map — static labels; lanes / pickers come from refresh_crop_map.
    window.set_crop_map_title_text(SharedString::from(i18n.t("title-crop-map")));
    window.set_crop_map_hint_text(SharedString::from(i18n.t("crop-map-hint")));
    window.set_crop_map_empty_text(SharedString::from(i18n.t("crop-map-empty")));
    window.set_crop_map_btn_move_text(SharedString::from(i18n.t("btn-crop-map-move")));
    window.set_crop_map_btn_split_text(SharedString::from(i18n.t("btn-crop-map-split")));
    window.set_crop_map_btn_deselect_text(SharedString::from(i18n.t("btn-crop-map-deselect")));
    window.set_crop_map_picker_title(SharedString::from(i18n.t("crop-map-picker-title")));
    window.set_crop_map_picker_cancel_text(SharedString::from(i18n.t("crop-map-picker-cancel")));
    window.set_crop_map_split_title(SharedString::from(i18n.t("crop-map-split-title")));
    window.set_crop_map_split_hint(SharedString::from(i18n.t("crop-map-split-hint")));
    window.set_crop_map_split_part_a_label(SharedString::from(i18n.t("crop-map-split-part-a")));
    window.set_crop_map_split_part_b_label(SharedString::from(i18n.t("crop-map-split-part-b")));
    window.set_crop_map_split_location_label(SharedString::from(i18n.t("crop-map-split-location")));
    // crop-map-split-area is set by `apply_unit_labels` (issue #29).
    window.set_crop_map_split_count_label(SharedString::from(i18n.t("crop-map-split-count")));
    window.set_crop_map_split_placeholder_area(SharedString::from(
        i18n.t("crop-map-split-placeholder-area"),
    ));
    window.set_crop_map_split_placeholder_count(SharedString::from(
        i18n.t("crop-map-split-placeholder-count"),
    ));
    window.set_crop_map_split_confirm_text(SharedString::from(i18n.t("crop-map-split-confirm")));
    window.set_crop_map_split_cancel_text(SharedString::from(i18n.t("crop-map-split-cancel")));

    // Strata page — static labels; the list and status come from refresh_strata.
    window.set_strata_title_text(SharedString::from(i18n.t("title-strata")));
    window.set_strata_list_title(SharedString::from(i18n.t("strata-list-title")));
    window.set_strata_empty_text(SharedString::from(i18n.t("empty-strata")));
    window.set_strata_delete_text(SharedString::from(i18n.t("button-delete")));
    window.set_strata_in_use_text(SharedString::from(i18n.t("strata-in-use")));
    window.set_strata_form_section(SharedString::from(i18n.t("section-new-strata")));
    window.set_strata_label_name(SharedString::from(i18n.t("label-strata-name")));
    window.set_strata_placeholder_name(SharedString::from(i18n.t("placeholder-strata-name")));
    window.set_strata_label_description(SharedString::from(i18n.t("label-strata-description")));
    window.set_strata_placeholder_description(SharedString::from(
        i18n.t("placeholder-strata-description"),
    ));
    window.set_strata_label_height_min(SharedString::from(i18n.t("label-strata-height-min")));
    window.set_strata_placeholder_height_min(SharedString::from(
        i18n.t("placeholder-strata-height-min"),
    ));
    window.set_strata_label_height_max(SharedString::from(i18n.t("label-strata-height-max")));
    window.set_strata_placeholder_height_max(SharedString::from(
        i18n.t("placeholder-strata-height-max"),
    ));
    window.set_strata_label_sort_order(SharedString::from(i18n.t("label-strata-sort-order")));
    window.set_strata_placeholder_sort_order(SharedString::from(
        i18n.t("placeholder-strata-sort-order"),
    ));
    window.set_strata_create_button_text(SharedString::from(i18n.t("button-create-strata")));

    // Settings page — static labels; the current-backend display is
    // refreshed by `refresh_settings`.
    window.set_nav_settings_text(SharedString::from(i18n.t("nav-settings")));
    window.set_settings_title_text(SharedString::from(i18n.t("title-settings")));
    window.set_settings_current_section(SharedString::from(i18n.t("settings-current-section")));
    window.set_settings_current_label(SharedString::from(i18n.t("settings-current-label")));
    window.set_settings_edit_section(SharedString::from(i18n.t("settings-edit-section")));
    window
        .set_settings_backend_kind_label(SharedString::from(i18n.t("settings-backend-kind-label")));
    let backend_kind_labels: Vec<SharedString> = [
        i18n.t("settings-backend-sqlite"),
        i18n.t("settings-backend-mariadb"),
    ]
    .into_iter()
    .map(SharedString::from)
    .collect();
    window.set_settings_backend_kind_labels(ModelRc::new(VecModel::from(backend_kind_labels)));
    window.set_settings_sqlite_path_label(SharedString::from(i18n.t("settings-sqlite-path-label")));
    window.set_settings_sqlite_path_placeholder(SharedString::from(
        i18n.t("settings-sqlite-path-placeholder"),
    ));
    window
        .set_settings_mariadb_host_label(SharedString::from(i18n.t("settings-mariadb-host-label")));
    window.set_settings_mariadb_host_placeholder(SharedString::from(
        i18n.t("settings-mariadb-host-placeholder"),
    ));
    window
        .set_settings_mariadb_port_label(SharedString::from(i18n.t("settings-mariadb-port-label")));
    window.set_settings_mariadb_port_placeholder(SharedString::from(
        i18n.t("settings-mariadb-port-placeholder"),
    ));
    window
        .set_settings_mariadb_user_label(SharedString::from(i18n.t("settings-mariadb-user-label")));
    window.set_settings_mariadb_user_placeholder(SharedString::from(
        i18n.t("settings-mariadb-user-placeholder"),
    ));
    window.set_settings_mariadb_password_label(SharedString::from(
        i18n.t("settings-mariadb-password-label"),
    ));
    window.set_settings_mariadb_password_placeholder(SharedString::from(
        i18n.t("settings-mariadb-password-placeholder"),
    ));
    window.set_settings_mariadb_database_label(SharedString::from(
        i18n.t("settings-mariadb-database-label"),
    ));
    window.set_settings_mariadb_database_placeholder(SharedString::from(
        i18n.t("settings-mariadb-database-placeholder"),
    ));
    window.set_settings_test_button(SharedString::from(i18n.t("settings-button-test")));
    window.set_settings_save_button(SharedString::from(i18n.t("settings-button-save")));
    window.set_settings_save_migrate_button(SharedString::from(
        i18n.t("settings-button-save-migrate"),
    ));
    window.set_settings_migrate_warning(SharedString::from(i18n.t("settings-migrate-warning")));
    window.set_settings_backup_section(SharedString::from(i18n.t("section-backup")));
    window.set_settings_backup_explain(SharedString::from(i18n.t("settings-backup-explain")));
    window.set_settings_backup_button(SharedString::from(i18n.t("button-backup-now")));

    // Public-holiday region picker (issue #35). Index 0 = "none"; the rest
    // mirrors `HolidayRegion::ALL` order.
    window.set_settings_holiday_section(SharedString::from(i18n.t("settings-holiday-section")));
    window.set_settings_holiday_region_label(SharedString::from(
        i18n.t("settings-holiday-region-label"),
    ));
    window.set_settings_holiday_explain(SharedString::from(i18n.t("settings-holiday-explain")));
    let mut region_labels: Vec<SharedString> =
        vec![SharedString::from(i18n.t("holiday-region-none"))];
    region_labels.extend(
        HolidayRegion::ALL
            .iter()
            .map(|r| SharedString::from(i18n.t(&format!("holiday-region-{}", r.key())))),
    );
    window.set_settings_holiday_region_labels(ModelRc::new(VecModel::from(region_labels)));
    window.set_settings_holiday_region_index(holiday_region_index(&app.config().holiday_region));

    // Display-unit pickers (issue #29). Combo order mirrors `AreaUnit::ALL`
    // / `MassUnit::ALL`; the suffixes themselves ("m²", "ha", "kg", "t")
    // are not translated.
    window.set_settings_units_section(SharedString::from(i18n.t("settings-units-section")));
    window.set_settings_units_explain(SharedString::from(i18n.t("settings-units-explain")));
    window.set_settings_area_unit_label(SharedString::from(i18n.t("settings-area-unit-label")));
    window.set_settings_mass_unit_label(SharedString::from(i18n.t("settings-mass-unit-label")));
    let area_unit_labels: Vec<SharedString> = AreaUnit::ALL
        .iter()
        .map(|u| SharedString::from(u.suffix()))
        .collect();
    window.set_settings_area_unit_labels(ModelRc::new(VecModel::from(area_unit_labels)));
    let mass_unit_labels: Vec<SharedString> = MassUnit::ALL
        .iter()
        .map(|u| SharedString::from(u.suffix()))
        .collect();
    window.set_settings_mass_unit_labels(ModelRc::new(VecModel::from(mass_unit_labels)));
    window.set_settings_area_unit_index(area_unit_index(app.area_unit()));
    window.set_settings_mass_unit_index(mass_unit_index(app.mass_unit()));
    apply_unit_labels(window, app);

    // Weekday header labels, shared by the unified calendar grid below.
    let weekday_labels: Vec<SharedString> = [
        i18n.t("weekday-mon-short"),
        i18n.t("weekday-tue-short"),
        i18n.t("weekday-wed-short"),
        i18n.t("weekday-thu-short"),
        i18n.t("weekday-fri-short"),
        i18n.t("weekday-sat-short"),
        i18n.t("weekday-sun-short"),
    ]
    .into_iter()
    .map(SharedString::from)
    .collect();

    // Unified calendar — sidebar + page chrome strings; the day grid is built
    // on every refresh by `refresh_task_calendar`.
    window.set_nav_tasks_text(SharedString::from(i18n.t("nav-tasks")));

    // Agenda page — static labels; the three row lists are pushed by
    // `refresh_agenda` on navigation and after any task edit.
    window.set_nav_agenda_text(SharedString::from(i18n.t("nav-agenda")));
    window.set_agenda_title_text(SharedString::from(i18n.t("title-agenda")));
    window.set_agenda_empty_text(SharedString::from(i18n.t("agenda-empty")));
    // Plan grid (Epic 2, story 2.3).
    window.set_nav_plan_text(SharedString::from(i18n.t("nav-plan")));
    window.set_plan_title_text(SharedString::from(i18n.t("title-plan")));
    window.set_plan_hint_text(SharedString::from(i18n.t("plan-hint")));
    window.set_plan_empty_text(SharedString::from(i18n.t("plan-empty")));
    window.set_plan_add_line_text(SharedString::from(i18n.t("plan-add-line")));
    window.set_plan_derived_placeholder(SharedString::from(i18n.t("plan-derived-placeholder")));
    window.set_plan_generate_label(SharedString::from(i18n.t("plan-generate-label")));
    window.set_plan_col_variety(SharedString::from(i18n.t("plan-col-variety")));
    window.set_plan_col_series(SharedString::from(i18n.t("plan-col-series")));
    window.set_plan_col_bed_meters(SharedString::from(i18n.t("plan-col-bed-meters")));
    window.set_plan_col_stagger(SharedString::from(i18n.t("plan-col-stagger")));
    window.set_plan_col_first_on(SharedString::from(i18n.t("plan-col-first-on")));
    window.set_plan_col_derived(SharedString::from(i18n.t("plan-col-derived")));
    window.set_plan_col_needs(SharedString::from(i18n.t("plan-col-needs")));
    window.set_plan_col_draft(SharedString::from(i18n.t("plan-col-draft")));
    window.set_plan_col_notes(SharedString::from(i18n.t("plan-col-notes")));
    window.set_plan_draft_badge(SharedString::from(i18n.t("plan-draft-badge")));

    // Needs list «Besoins» (story 2.7).
    window.set_nav_needs_text(SharedString::from(i18n.t("nav-needs")));
    window.set_needs_title_text(SharedString::from(i18n.t("title-needs")));
    window.set_needs_hint_text(SharedString::from(i18n.t("needs-hint")));
    window.set_needs_empty_text(SharedString::from(i18n.t("needs-empty")));
    window.set_needs_col_variety(SharedString::from(i18n.t("needs-col-variety")));
    window.set_needs_col_quantity(SharedString::from(i18n.t("needs-col-quantity")));
    window.set_needs_col_buy_by(SharedString::from(i18n.t("needs-col-buy-by")));
    window.set_needs_col_lines(SharedString::from(i18n.t("needs-col-lines")));
    window.set_needs_buy_by_none(SharedString::from(i18n.t("needs-buy-by-none")));
    window.set_needs_print_text(SharedString::from(i18n.t("needs-print")));
    window.set_needs_print_disabled_tip(SharedString::from(i18n.t("needs-print-disabled")));

    window.set_agenda_overdue_label(SharedString::from(i18n.t("agenda-overdue-title")));
    window.set_agenda_today_label(SharedString::from(i18n.t("agenda-today-title")));
    // Settle menu + skipped badge (story 1.5).
    window.set_agenda_skipped_label(SharedString::from(i18n.t("agenda-skipped-title")));
    window.set_agenda_menu_done_label(SharedString::from(i18n.t("agenda-menu-done")));
    window.set_agenda_menu_skip_label(SharedString::from(i18n.t("agenda-menu-skip")));
    window.set_agenda_menu_correct_label(SharedString::from(i18n.t("agenda-menu-correct")));
    // Skip-reason dialog chrome (story 1.5).
    window.set_skip_dialog_title(SharedString::from(i18n.t("skip-dialog-title")));
    window.set_skip_dialog_note_placeholder(SharedString::from(
        i18n.t("skip-dialog-note-placeholder"),
    ));
    window.set_skip_dialog_ok_text(SharedString::from(i18n.t("skip-dialog-ok")));
    window.set_skip_dialog_cancel_text(SharedString::from(i18n.t("skip-dialog-cancel")));

    // Shared confirmation dialog (issue #61) — static chrome.
    window.set_confirm_title(SharedString::from(i18n.t("confirm-delete-title")));
    window.set_confirm_ok_text(SharedString::from(i18n.t("confirm-ok")));
    window.set_confirm_cancel_text(SharedString::from(i18n.t("confirm-cancel")));

    window.set_task_calendar_title_text(SharedString::from(i18n.t("title-task-calendar")));
    window.set_task_calendar_prev_button_text(SharedString::from(i18n.t("calendar-prev")));
    window.set_task_calendar_next_button_text(SharedString::from(i18n.t("calendar-next")));
    window.set_task_calendar_today_button_text(SharedString::from(i18n.t("calendar-today")));
    window.set_task_calendar_empty_state_text(SharedString::from(i18n.t("task-calendar-empty")));
    window.set_task_calendar_hint_text(SharedString::from(i18n.t("task-calendar-hint")));
    window.set_task_calendar_new_task_button_text(SharedString::from(
        i18n.t("task-calendar-new-task"),
    ));
    window.set_task_calendar_weekday_labels(ModelRc::new(VecModel::from(weekday_labels)));

    // Task form (create / edit)
    window.set_task_form_title_new_text(SharedString::from(i18n.t("task-form-title-new")));
    window.set_task_form_title_edit_text(SharedString::from(i18n.t("task-form-title-edit")));
    window.set_task_form_label_task_type(SharedString::from(i18n.t("label-task-type")));
    window.set_task_form_label_planting(SharedString::from(i18n.t("label-task-planting")));
    window.set_task_form_label_planned_on(SharedString::from(i18n.t("label-task-planned-on")));
    window.set_task_form_label_notes(SharedString::from(i18n.t("label-task-notes")));
    window.set_task_form_placeholder_date(SharedString::from(i18n.t("placeholder-date")));
    window.set_task_form_placeholder_notes(SharedString::from(i18n.t("placeholder-task-notes")));
    window.set_task_form_label_completed(SharedString::from(i18n.t("label-task-completed")));
    window.set_task_form_btn_save_text(SharedString::from(i18n.t("btn-task-save")));
    window.set_task_form_btn_cancel_text(SharedString::from(i18n.t("btn-task-cancel")));
    window.set_task_form_btn_delete_text(SharedString::from(i18n.t("btn-task-delete")));
    window.set_task_form_label_recurring(SharedString::from(i18n.t("label-task-recurring")));
    window.set_task_form_label_recurrence_interval(SharedString::from(
        i18n.t("label-task-recurrence-interval"),
    ));
    window.set_task_form_placeholder_recurrence_interval(SharedString::from(
        i18n.t("placeholder-task-recurrence-interval"),
    ));
    window.set_task_form_label_recurrence_unit(SharedString::from(
        i18n.t("label-task-recurrence-unit"),
    ));
    window.set_task_form_label_recurrence_end_on(SharedString::from(
        i18n.t("label-task-recurrence-end-on"),
    ));
    window.set_task_form_placeholder_recurrence_end_on(SharedString::from(
        i18n.t("placeholder-task-recurrence-end-on"),
    ));
    window.set_task_form_hint_recurrence_end_on(SharedString::from(
        i18n.t("hint-task-recurrence-end-on"),
    ));
    window.set_task_form_recurring_series_badge_text(SharedString::from(
        i18n.t("task-form-series-badge"),
    ));

    // Task Calendar — "Manage types" button
    window.set_task_calendar_manage_types_button_text(SharedString::from(
        i18n.t("task-types-button"),
    ));
    window.set_task_calendar_filter_hint_text(SharedString::from(
        i18n.t("task-calendar-filter-hint"),
    ));
    window.set_task_calendar_filter_all_button_text(SharedString::from(
        i18n.t("task-calendar-filter-all"),
    ));
    window.set_task_calendar_milestone_filter_label(SharedString::from(
        i18n.t("task-calendar-filter-milestones"),
    ));
    window.set_task_calendar_legend_task_label(SharedString::from(
        i18n.t("task-calendar-legend-task"),
    ));
    window.set_task_calendar_legend_milestone_label(SharedString::from(
        i18n.t("task-calendar-legend-milestone"),
    ));

    // Task Types catalog
    window.set_task_types_title_text(SharedString::from(i18n.t("title-task-types")));
    window.set_task_types_list_title(SharedString::from(i18n.t("task-types-list-title")));
    window.set_task_types_empty_text(SharedString::from(i18n.t("task-types-empty")));
    window.set_task_types_form_section_create(SharedString::from(
        i18n.t("task-types-form-section-create"),
    ));
    window.set_task_types_form_section_edit(SharedString::from(
        i18n.t("task-types-form-section-edit"),
    ));
    window.set_task_types_label_name(SharedString::from(i18n.t("label-task-type-name")));
    window
        .set_task_types_placeholder_name(SharedString::from(i18n.t("placeholder-task-type-name")));
    window.set_task_types_label_category(SharedString::from(i18n.t("label-task-type-category")));
    window.set_task_types_label_color(SharedString::from(i18n.t("label-task-type-color")));
    window.set_task_types_placeholder_color(SharedString::from(
        i18n.t("placeholder-task-type-color"),
    ));
    window.set_task_types_hint_color(SharedString::from(i18n.t("hint-task-type-color")));
    window.set_task_types_btn_save_text(SharedString::from(i18n.t("btn-task-type-save")));
    window.set_task_types_btn_cancel_text(SharedString::from(i18n.t("btn-task-type-cancel")));
    window.set_task_types_btn_back_text(SharedString::from(i18n.t("btn-task-type-back")));
    window.set_task_types_edit_text(SharedString::from(i18n.t("btn-task-type-edit")));
    window.set_task_types_delete_text(SharedString::from(i18n.t("btn-task-type-delete")));
    window.set_task_types_in_use_text(SharedString::from(i18n.t("task-type-in-use")));

    // Families catalog
    window.set_families_title_text(SharedString::from(i18n.t("title-families")));
    window.set_families_list_title(SharedString::from(i18n.t("families-list-title")));
    window.set_families_empty_text(SharedString::from(i18n.t("families-empty")));
    window.set_families_form_section_create(SharedString::from(
        i18n.t("families-form-section-create"),
    ));
    window.set_families_form_section_edit(SharedString::from(i18n.t("families-form-section-edit")));
    window.set_families_label_name(SharedString::from(i18n.t("label-family-name")));
    window.set_families_placeholder_name(SharedString::from(i18n.t("placeholder-family-name")));
    window.set_families_label_latin(SharedString::from(i18n.t("label-family-latin")));
    window.set_families_placeholder_latin(SharedString::from(i18n.t("placeholder-family-latin")));
    window.set_families_label_description(SharedString::from(i18n.t("label-family-description")));
    window.set_families_placeholder_description(SharedString::from(
        i18n.t("placeholder-family-description"),
    ));
    window.set_families_label_color(SharedString::from(i18n.t("label-family-color")));
    window.set_families_placeholder_color(SharedString::from(i18n.t("placeholder-family-color")));
    window.set_families_hint_color(SharedString::from(i18n.t("hint-family-color")));
    window.set_families_btn_save_text(SharedString::from(i18n.t("btn-family-save")));
    window.set_families_btn_cancel_text(SharedString::from(i18n.t("btn-family-cancel")));
    window.set_families_edit_text(SharedString::from(i18n.t("btn-family-edit")));
    window.set_families_delete_text(SharedString::from(i18n.t("btn-family-delete")));
    window.set_families_in_use_text(SharedString::from(i18n.t("family-in-use")));

    // Plantings page
    window.set_plantings_col_variety(SharedString::from(i18n.t("plantings-col-variety")));
    window.set_plantings_col_location(SharedString::from(i18n.t("plantings-col-location")));
    window.set_plantings_col_strata(SharedString::from(i18n.t("label-crop-strata")));
    window.set_plantings_col_schedule(SharedString::from(i18n.t("plantings-col-schedule")));
    window.set_plantings_col_area(SharedString::from(i18n.t("plantings-col-area")));
    window.set_plantings_col_plants(SharedString::from(i18n.t("plantings-col-plants")));
    window.set_plantings_col_status(SharedString::from(i18n.t("plantings-col-status")));
    window.set_plantings_title_text(SharedString::from(i18n.t("title-plantings")));
    window.set_empty_state_text(SharedString::from(i18n.t("empty-plantings")));
    window.set_section_new_text(SharedString::from(i18n.t("section-new-planting")));
    window.set_label_variety(SharedString::from(i18n.t("label-variety")));
    window.set_label_location(SharedString::from(i18n.t("label-location")));
    window.set_label_sown_on(SharedString::from(i18n.t("label-sown-on")));
    window.set_label_planting_method(SharedString::from(i18n.t("label-planting-method")));
    window.set_label_planting_date(SharedString::from(i18n.t("label-planting-date")));
    // Order must match `establishment_method_from_index`: 0 direct, 1 raised, 2 bought.
    let method_labels: Vec<SharedString> = [
        "method-direct-sow",
        "method-raised-transplant",
        "method-bought-plants",
    ]
    .into_iter()
    .map(|k| SharedString::from(i18n.t(k)))
    .collect();
    window.set_planting_method_labels(ModelRc::new(VecModel::from(method_labels)));
    window.set_label_established_on(SharedString::from(i18n.t("label-established-on")));
    window.set_label_removal_on(SharedString::from(i18n.t("label-removal-on")));
    window.set_placeholder_removal_date(SharedString::from(i18n.t("placeholder-removal-date")));
    // label-area is set by `apply_unit_labels` (issue #29).
    window.set_label_count(SharedString::from(i18n.t("label-plants-count")));
    window.set_placeholder_date(SharedString::from(i18n.t("placeholder-date")));
    window.set_placeholder_area(SharedString::from(i18n.t("placeholder-area")));
    window.set_placeholder_count(SharedString::from(i18n.t("placeholder-count")));
    window.set_create_button_text(SharedString::from(i18n.t("button-create-planting")));
    window.set_plants_suffix(SharedString::from(i18n.t("plants-suffix")));

    // Cultures page
    window.set_cultures_title_text(SharedString::from(i18n.t("title-cultures")));
    window.set_crops_title(SharedString::from(i18n.t("crops-title")));
    window.set_empty_crops_text(SharedString::from(i18n.t("empty-crops")));
    window.set_varieties_title(SharedString::from(i18n.t("varieties-title")));
    window.set_empty_varieties_text(SharedString::from(i18n.t("empty-varieties")));
    window.set_no_crop_selected_text(SharedString::from(i18n.t("no-crop-selected")));
    // ITK editor (story 2.5).
    window.set_itk_title(SharedString::from(i18n.t("itk-title")));
    window.set_itk_empty_text(SharedString::from(i18n.t("itk-empty")));
    window.set_itk_offset_placeholder(SharedString::from(i18n.t("itk-offset-placeholder")));
    window.set_itk_label_placeholder(SharedString::from(i18n.t("itk-label-placeholder")));
    window.set_itk_add_label(SharedString::from(i18n.t("itk-add")));
    window.set_itk_save_label(SharedString::from(i18n.t("itk-save")));
    window.set_itk_cancel_label(SharedString::from(i18n.t("itk-cancel")));
    window.set_itk_method_label(SharedString::from(i18n.t("itk-method")));
    window.set_itk_implement_label(SharedString::from(i18n.t("itk-implement")));
    window.set_itk_edit_tip(SharedString::from(i18n.t("itk-edit-tip")));
    window.set_itk_delete_tip(SharedString::from(i18n.t("itk-delete-tip")));
    window.set_new_crop_section(SharedString::from(i18n.t("new-crop-section")));
    window.set_new_variety_section(SharedString::from(i18n.t("new-variety-section")));
    window.set_new_variety_section_pluriannual(SharedString::from(
        i18n.t("new-variety-section-pluriannual"),
    ));
    window.set_label_crop_name(SharedString::from(i18n.t("label-crop-name")));
    window.set_placeholder_crop_name(SharedString::from(i18n.t("placeholder-crop-name")));
    window.set_label_crop_latin(SharedString::from(i18n.t("label-crop-latin")));
    window.set_placeholder_crop_latin(SharedString::from(i18n.t("placeholder-crop-latin")));
    window.set_label_crop_family(SharedString::from(i18n.t("label-crop-family")));
    // Strata now lives on the planting (issue #86): the label feeds the
    // Plantings form, reusing the existing "Strate" string.
    window.set_label_planting_strata(SharedString::from(i18n.t("label-crop-strata")));
    window.set_label_lifespan(SharedString::from(i18n.t("label-lifespan")));
    window.set_label_lifespan_years(SharedString::from(i18n.t("label-lifespan-years")));
    window.set_placeholder_lifespan_years(SharedString::from(i18n.t("placeholder-lifespan-years")));
    window.set_label_years_to_first_yield(SharedString::from(i18n.t("label-years-to-first-yield")));
    window.set_placeholder_years_to_first_yield(SharedString::from(
        i18n.t("placeholder-years-to-first-yield"),
    ));
    window.set_label_pruning(SharedString::from(i18n.t("label-pruning")));
    let lifespan_labels: Vec<SharedString> = [
        i18n.t("lifespan-annual"),
        i18n.t("lifespan-pluriannual-single"),
        i18n.t("lifespan-pluriannual-recurring"),
    ]
    .into_iter()
    .map(SharedString::from)
    .collect();
    window.set_lifespan_labels(ModelRc::new(VecModel::from(lifespan_labels)));
    let pruning_labels: Vec<SharedString> = [
        i18n.t("pruning-none-label"),
        i18n.t("pruning-winter-label"),
        i18n.t("pruning-summer-label"),
        i18n.t("pruning-both-label"),
    ]
    .into_iter()
    .map(SharedString::from)
    .collect();
    window.set_pruning_labels(ModelRc::new(VecModel::from(pruning_labels)));
    window.set_label_bud_break_doy(SharedString::from(i18n.t("label-bud-break-doy")));
    window.set_placeholder_bud_break_doy(SharedString::from(i18n.t("placeholder-bud-break-doy")));
    window.set_label_flowering_doy(SharedString::from(i18n.t("label-flowering-doy")));
    window.set_placeholder_flowering_doy(SharedString::from(i18n.t("placeholder-flowering-doy")));
    window.set_label_harvest_start_doy(SharedString::from(i18n.t("label-harvest-start-doy")));
    window.set_placeholder_harvest_start_doy(SharedString::from(
        i18n.t("placeholder-harvest-start-doy"),
    ));
    window.set_label_harvest_end_doy(SharedString::from(i18n.t("label-harvest-end-doy")));
    window
        .set_placeholder_harvest_end_doy(SharedString::from(i18n.t("placeholder-harvest-end-doy")));
    window.set_label_yield_kg(SharedString::from(i18n.t("label-yield-kg")));
    window.set_placeholder_yield_kg(SharedString::from(i18n.t("placeholder-yield-kg")));
    window.set_label_variety_name(SharedString::from(i18n.t("label-variety-name")));
    window.set_placeholder_variety_name(SharedString::from(i18n.t("placeholder-variety-name")));
    window.set_label_variety_description(SharedString::from(i18n.t("label-variety-description")));
    window.set_placeholder_variety_description(SharedString::from(
        i18n.t("placeholder-variety-description"),
    ));
    window.set_label_dtt(SharedString::from(i18n.t("label-dtt")));
    window.set_label_dtm(SharedString::from(i18n.t("label-dtm")));
    window.set_label_window(SharedString::from(i18n.t("label-window")));
    window.set_placeholder_dtt(SharedString::from(i18n.t("placeholder-dtt")));
    window.set_placeholder_dtm(SharedString::from(i18n.t("placeholder-dtm")));
    window.set_placeholder_window(SharedString::from(i18n.t("placeholder-window")));
    window.set_create_crop_button_text(SharedString::from(i18n.t("button-create-crop")));
    window.set_crop_edit_text(SharedString::from(i18n.t("button-edit")));
    window.set_crop_delete_text(SharedString::from(i18n.t("button-delete")));
    window.set_crop_in_use_text(SharedString::from(i18n.t("crop-in-use")));
    window.set_crop_form_section_edit(SharedString::from(i18n.t("crop-form-section-edit")));
    window.set_crop_cancel_text(SharedString::from(i18n.t("button-cancel-crop-edit")));
    window.set_crop_save_text(SharedString::from(i18n.t("button-save-crop")));
    window.set_create_variety_button_text(SharedString::from(i18n.t("button-create-variety")));
    window.set_variety_edit_text(SharedString::from(i18n.t("button-edit")));
    window.set_variety_form_section_edit(SharedString::from(i18n.t("variety-form-section-edit")));
    window.set_variety_save_text(SharedString::from(i18n.t("button-save-variety")));
    window.set_variety_cancel_text(SharedString::from(i18n.t("button-cancel-variety-edit")));

    // Planting detail page — static labels only; per-planting data is
    // refreshed by `refresh_planting_detail` whenever a row/event is clicked.
    window.set_detail_title_text(SharedString::from(i18n.t("title-planting-detail")));
    window.set_detail_back_button_text(SharedString::from(i18n.t("button-back")));
    window.set_detail_section_schedule_text(SharedString::from(i18n.t("section-schedule")));
    window.set_detail_section_summary_text(SharedString::from(i18n.t("section-summary")));
    window.set_detail_name_label(SharedString::from(i18n.t("label-planting-name")));
    window.set_detail_notes_label(SharedString::from(i18n.t("label-planting-notes")));
    window.set_detail_empty_state_text(SharedString::from(i18n.t("empty-planting-detail")));

    // Tasks section labels — content rows come from refresh_planting_detail.
    window.set_detail_tasks_section_text(SharedString::from(i18n.t("section-planting-tasks")));
    window.set_detail_tasks_empty_text(SharedString::from(i18n.t("empty-planting-tasks")));
    window.set_detail_tasks_overdue_badge(SharedString::from(i18n.t("task-badge-overdue")));
    window.set_detail_tasks_done_badge(SharedString::from(i18n.t("task-badge-done")));
    window.set_detail_tasks_skipped_badge(SharedString::from(i18n.t("agenda-skipped-title")));

    // Life-cycle status & delete (issue #63). The options model is parallel to
    // `status_from_index` / `status_to_index`: active, completed, failed,
    // abandoned. The current value/index is set per-planting in refresh.
    window.set_detail_status_section_text(SharedString::from(i18n.t("section-planting-lifecycle")));
    window.set_detail_status_field_label(SharedString::from(i18n.t("label-status")));
    window.set_detail_change_status_text(SharedString::from(i18n.t("button-change-status")));
    window.set_detail_delete_button_text(SharedString::from(i18n.t("button-delete-planting")));
    let status_options: Vec<SharedString> = [
        PlantingStatus::Active,
        PlantingStatus::Completed,
        PlantingStatus::Failed,
        PlantingStatus::Abandoned,
    ]
    .into_iter()
    .map(|st| SharedString::from(i18n.t(planting_status_key(st))))
    .collect();
    window.set_detail_status_options(ModelRc::new(VecModel::from(status_options)));

    // Yearly-harvest section labels — content rows come from refresh_planting_detail.
    window.set_harvest_section_title(SharedString::from(i18n.t("section-yearly-harvest")));
    window.set_harvest_empty_text(SharedString::from(i18n.t("empty-yearly-harvest")));
    window.set_harvest_header_year(SharedString::from(i18n.t("harvest-header-year")));
    window.set_harvest_header_expected(SharedString::from(i18n.t("harvest-header-expected")));
    window.set_harvest_header_actual(SharedString::from(i18n.t("harvest-header-actual")));
    window.set_harvest_header_variance(SharedString::from(i18n.t("harvest-header-variance")));
    window.set_harvest_header_notes(SharedString::from(i18n.t("harvest-header-notes")));
    window.set_harvest_form_section(SharedString::from(i18n.t("section-record-harvest")));
    window.set_harvest_label_year(SharedString::from(i18n.t("label-harvest-year")));
    // label-harvest-expected/actual are set by `apply_unit_labels` (issue #29).
    window.set_harvest_label_notes(SharedString::from(i18n.t("label-harvest-notes")));
    window.set_harvest_placeholder_year(SharedString::from(i18n.t("placeholder-harvest-year")));
    window.set_harvest_placeholder_kg(SharedString::from(i18n.t("placeholder-harvest-kg")));
    window.set_harvest_placeholder_notes(SharedString::from(i18n.t("placeholder-harvest-notes")));
    window.set_harvest_record_button(SharedString::from(i18n.t("button-record-harvest")));

    // Treatments section labels (issue #82) — rows come from refresh_planting_detail.
    window.set_treatments_section_title(SharedString::from(i18n.t("section-treatments")));
    window.set_treatments_empty_text(SharedString::from(i18n.t("empty-treatments")));
    window.set_treatment_header_date(SharedString::from(i18n.t("treatment-header-date")));
    window.set_treatment_header_substance(SharedString::from(i18n.t("treatment-header-substance")));
    window.set_treatment_header_product(SharedString::from(i18n.t("treatment-header-product")));
    window.set_treatment_header_dose(SharedString::from(i18n.t("treatment-header-dose")));
    window.set_treatment_header_notes(SharedString::from(i18n.t("treatment-header-notes")));
    window.set_treatment_form_section(SharedString::from(i18n.t("section-record-treatment")));
    window.set_treatment_label_date(SharedString::from(i18n.t("label-treatment-date")));
    window.set_treatment_label_substance(SharedString::from(i18n.t("label-treatment-substance")));
    window.set_treatment_label_product(SharedString::from(i18n.t("label-treatment-product")));
    window.set_treatment_label_dose(SharedString::from(i18n.t("label-treatment-dose")));
    window.set_treatment_label_unit(SharedString::from(i18n.t("label-treatment-unit")));
    window.set_treatment_label_notes(SharedString::from(i18n.t("label-treatment-notes")));
    window.set_treatment_placeholder_substance(SharedString::from(
        i18n.t("placeholder-treatment-substance"),
    ));
    window.set_treatment_placeholder_product(SharedString::from(
        i18n.t("placeholder-treatment-product"),
    ));
    window.set_treatment_placeholder_dose(SharedString::from(i18n.t("placeholder-treatment-dose")));
    window.set_treatment_placeholder_unit(SharedString::from(i18n.t("placeholder-treatment-unit")));
    window
        .set_treatment_placeholder_notes(SharedString::from(i18n.t("placeholder-treatment-notes")));
    window.set_treatment_record_button(SharedString::from(i18n.t("button-record-treatment")));
    window.set_treatment_delete_button(SharedString::from(i18n.t("button-delete-treatment")));

    // Locations page
    window.set_locations_title_text(SharedString::from(i18n.t("title-locations")));
    window.set_locations_list_title(SharedString::from(i18n.t("locations-list-title")));
    window.set_empty_locations_text(SharedString::from(i18n.t("empty-locations")));
    window.set_location_form_section(SharedString::from(i18n.t("new-location-section")));
    window.set_label_loc_name(SharedString::from(i18n.t("label-loc-name")));
    window.set_placeholder_loc_name(SharedString::from(i18n.t("placeholder-loc-name")));
    window.set_label_loc_kind(SharedString::from(i18n.t("label-loc-kind")));
    window.set_label_loc_length(SharedString::from(i18n.t("label-loc-length")));
    window.set_placeholder_loc_length(SharedString::from(i18n.t("placeholder-loc-length")));
    window.set_label_loc_width(SharedString::from(i18n.t("label-loc-width")));
    window.set_placeholder_loc_width(SharedString::from(i18n.t("placeholder-loc-width")));
    window.set_label_loc_parent(SharedString::from(i18n.t("label-loc-parent")));
    window.set_label_loc_notes(SharedString::from(i18n.t("label-loc-notes")));
    window.set_placeholder_loc_notes(SharedString::from(i18n.t("placeholder-loc-notes")));
    window.set_create_loc_button_text(SharedString::from(i18n.t("button-create-location")));
    window.set_loc_form_section_edit(SharedString::from(i18n.t("loc-form-section-edit")));
}

/// (Re)apply the form labels and tooltips that embed the current display
/// units (issue #29). Split out of `apply_translations` so a unit change
/// can refresh them without redoing the whole catalogue.
pub(crate) fn apply_unit_labels(window: &MainWindow, app: &App) {
    let i18n = app.i18n();
    let mut area_args = FluentArgs::new();
    area_args.set("unit", app.area_unit().suffix());
    let mut mass_args = FluentArgs::new();
    mass_args.set("unit", app.mass_unit().suffix());
    window.set_label_area(SharedString::from(i18n.t_args("label-area", &area_args)));
    window.set_crop_map_split_area_label(SharedString::from(
        i18n.t_args("crop-map-split-area", &area_args),
    ));
    window
        .global::<TooltipCatalog>()
        .set_planting_area(SharedString::from(
            i18n.t_args("tooltip-planting-area", &area_args),
        ));
    window.set_harvest_label_expected(SharedString::from(
        i18n.t_args("label-harvest-expected", &mass_args),
    ));
    window.set_harvest_label_actual(SharedString::from(
        i18n.t_args("label-harvest-actual", &mass_args),
    ));
}

/// Combo index of an area display unit — `AreaUnit::ALL` order. Inverse of
/// `wiring::settings::area_unit_from_index`.
pub(crate) fn area_unit_index(unit: AreaUnit) -> i32 {
    AreaUnit::ALL
        .iter()
        .position(|u| *u == unit)
        .and_then(|p| i32::try_from(p).ok())
        .unwrap_or(0)
}

/// Combo index of a mass display unit — `MassUnit::ALL` order. Inverse of
/// `wiring::settings::mass_unit_from_index`.
pub(crate) fn mass_unit_index(unit: MassUnit) -> i32 {
    MassUnit::ALL
        .iter()
        .position(|u| *u == unit)
        .and_then(|p| i32::try_from(p).ok())
        .unwrap_or(0)
}

/// Combo index of a persisted holiday-region code: 0 = none/unknown, then
/// `HolidayRegion::ALL` order shifted by one. Inverse of `wiring::settings::holiday_region_code`.
pub(crate) fn holiday_region_index(code: &str) -> i32 {
    HolidayRegion::parse(code).map_or(0, |r| {
        HolidayRegion::ALL
            .iter()
            .position(|x| *x == r)
            .and_then(|p| i32::try_from(p + 1).ok())
            .unwrap_or(0)
    })
}
