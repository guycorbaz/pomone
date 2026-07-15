//! Pomone application crate: services, use cases, application state.
//!
//! Application code (the `pomone-ui` and `pomone-cli` binaries) depends on
//! this crate and through it on the `Repository` abstraction in
//! `pomone-db`. The repository implementation is selected by [`AppConfig`]
//! at runtime.

pub mod agenda_view;
pub mod app;
pub mod backup;
pub mod bed_usage_view;
pub mod calendar_view;
pub mod config;
pub mod crop_map_view;
pub mod cultures_view;
pub mod demo;
pub mod error;
pub mod facts;
pub mod families_view;
pub mod harvest_view;
pub mod i18n;
pub mod locations_view;
pub mod migration;
pub mod plan_view;
pub mod planting_detail_view;
pub mod plantings_view;
pub mod printdoc;
pub mod services;
pub mod strata_view;
pub mod task_autogen;
pub mod task_calendar_view;
pub mod task_types_view;
pub mod tasks_view;
pub mod treatments_view;
pub mod unified_calendar_view;
pub mod units;

#[cfg(test)]
mod test_helpers;

pub use agenda_view::{list_agenda, mark_task_done, reopen_task, skip_task, AgendaRow};
pub use app::{test_backend, App};
pub use backup::{
    backup_backend, backup_path_for, backup_sqlite, backup_stamp_now, restore_sqlite,
};
pub use bed_usage_view::{bed_usage_series, BedUsage, BedUsagePoint};
pub use calendar_view::{list_events_in_range, CalendarEvent, CalendarEventKind};
pub use config::{default_config_path, AppConfig, BackendConfig, WindowGeometry};
pub use crop_map_view::{
    list_crop_map_lanes, move_planting_to_location, split_planting, CropMapBar, CropMapLane,
    SplitPart,
};
pub use cultures_view::{
    create_crop, create_variety, delete_crop, delete_variety, get_crop_for_edit,
    get_variety_for_edit, list_crops, list_family_options, list_strata_options,
    list_varieties_for_crop, update_crop, update_variety, CropEditForm, CropInput, CropRow,
    FamilyOption, LifespanKind, StrataOption, VarietyEditForm, VarietyInput, VarietyProfileKind,
    VarietyRow,
};
pub use demo::{seed_demo_data, DemoSummary};
pub use error::{AppError, AppResult};
pub use families_view::{
    create_family, delete_family, get_family_for_edit, list_families_admin, update_family,
    FamilyAdminRow, FamilyEditForm,
};
pub use harvest_view::{list_yearly_harvests_for_planting, YearlyHarvestRow};
pub use i18n::{I18n, Lang};
pub use locations_view::{
    create_location, delete_location, get_location_for_edit, list_location_kind_options,
    list_locations_tree, list_parent_options, update_location, LocationEditForm, LocationInput,
    LocationKindOption, LocationListItem, ParentLocationOption,
};
pub use migration::{copy_all, MigrationReport};
pub use plan_view::{delete_plan_line, list_plan_rows, save_plan_line, PlanLineInput, PlanRow};
pub use planting_detail_view::{
    get_planting_detail, list_planting_tasks, DetailLine, PlantingDetail, PlantingTaskRow,
};
pub use plantings_view::{
    list_location_options, list_plantings, list_variety_options, parse_id, parse_iso_date,
    planting_status_key, CycleDates, LocationOption, PlantingRow, VarietyOption,
};
pub use strata_view::{create_strata, delete_strata, list_strata_rows, StrataInput, StrataRow};
pub use task_autogen::generate_tasks_for_planting;
pub use task_calendar_view::{
    list_task_calendar_rows, reschedule_task, toggle_task_completion, TaskCalendarRow,
};
pub use task_types_view::{
    create_task_type, delete_task_type, get_task_type_for_edit, list_task_category_options,
    list_task_types_admin, update_task_type, TaskCategoryOption, TaskTypeAdminRow,
    TaskTypeEditForm,
};
pub use tasks_view::{
    create_recurring_task, create_task, delete_task, extend_series_if_needed, get_task_for_edit,
    list_planting_choices, list_task_type_options, recurrence_unit_str, update_task,
    PlantingChoice, TaskEditForm, TaskTypeOption,
};
pub use treatments_view::{delete_treatment, list_treatments_for_planting, TreatmentRow};
pub use unified_calendar_view::{list_calendar_entries, CalendarEntry, CalendarEntryKind};
pub use units::{AreaUnit, MassUnit};
