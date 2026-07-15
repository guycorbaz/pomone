//! Mutable single-threaded UI state shared by every wiring module.
//! Extracted from `main.rs` (story 0.4); re-exported from the crate root so
//! `crate::…` paths keep working everywhere.

use pomone_app::App;

/// A destructive action awaiting the user's confirmation in the shared dialog
/// (issue #61). Stored on [`UiState`] between "delete requested" and the
/// dialog's accept/cancel.
#[derive(Clone)]
pub(crate) enum PendingDelete {
    Strata(String),
    /// Task id (the one open in the edit form).
    Task(String),
    TaskType(String),
    /// Planting id (the one open in the detail page). Issue #63.
    Planting(String),
    /// Crop id (a row in the Cultures catalog).
    Crop(String),
    /// Variety id (a row in the Cultures varieties panel).
    Variety(String),
    /// Location id (a row in the Lieux tree).
    Location(String),
    /// Family id (a row in the Familles catalog).
    Family(String),
    /// Treatment id (a row in the detail page's treatments table). Issue #82.
    Treatment(String),
}

/// Mutable, single-threaded UI state. Slint runs on the main thread and tokio
/// drives async DB calls via `Runtime::block_on` inside callbacks (SQLite
/// queries finish in microseconds — blocking the UI thread is fine here).
pub(crate) struct UiState {
    pub(crate) app: App,
    /// Set when a destructive action is waiting on the confirmation dialog.
    pub(crate) pending_delete: Option<PendingDelete>,
    pub(crate) runtime: tokio::runtime::Runtime,
    /// Stringified `VarietyId`s, parallel to the Plantings page `variety-labels`.
    pub(crate) variety_ids: Vec<String>,
    /// Parallel to `variety_ids`: tells the planting form which date fields
    /// to show (sowing for annuals, establishment + removal for perennials).
    pub(crate) variety_is_annuals_plantings: Vec<bool>,
    /// Active sort column for the Plantings table (`"variety"`, `"location"`,
    /// `"area"`, `"plants"`, `"status"`) and its direction.
    pub(crate) plantings_sort_column: String,
    pub(crate) plantings_sort_asc: bool,
    /// Stringified `LocationId`s, parallel to the Plantings page `location-labels`.
    pub(crate) location_ids: Vec<String>,
    /// Stringified `FamilyId`s, parallel to the Cultures page `family-labels`.
    pub(crate) family_ids: Vec<String>,
    /// Stringified `StrataId`s, parallel to the Plantings page `strata-labels`
    /// (strata moved from crop to planting — issue #86).
    pub(crate) strata_ids: Vec<String>,
    /// Stringified `CropId`s, parallel to the Cultures page `crops` model
    /// (so a row click index resolves to a typed `CropId`).
    pub(crate) crop_ids: Vec<String>,
    /// Parallel to `crop_ids`: tells the UI which variety form to show.
    pub(crate) crop_is_annuals: Vec<bool>,
    /// Stringified `LocationKindId`s, parallel to the Locations page
    /// `loc-kind-labels` model.
    pub(crate) location_kind_ids: Vec<String>,
    /// Stringified `LocationId`s, parallel to the Locations page
    /// `loc-parent-labels` model. The first entry is an empty string for the
    /// synthetic "(no parent)" option.
    pub(crate) parent_location_ids: Vec<String>,
    /// Year currently displayed by the Task Calendar screen.
    pub(crate) task_calendar_year: i32,
    /// Month (1..=12) currently displayed by the Task Calendar screen.
    pub(crate) task_calendar_month: u32,
    /// Page to return to when the Back button is pressed on the detail
    /// screen. Stored at the moment the user opens a planting so the
    /// detail view can route back to either the list or the calendar.
    pub(crate) detail_previous_page: String,
    /// Stringified `PlantingId` currently shown on the detail screen.
    /// Needed by the yearly-harvest "Record" callback, which doesn't get
    /// the id passed back through Slint.
    pub(crate) detail_planting_id: String,
    /// Stringified `TaskTypeId`s parallel to the task form's type dropdown.
    pub(crate) task_form_type_ids: Vec<String>,
    /// Stringified `PlantingId`s parallel to the task form's planting
    /// dropdown. Index 0 is the empty string for the "— Aucun —" entry.
    pub(crate) task_form_planting_ids: Vec<String>,
    /// Stringified `TaskId` currently being edited; empty in create mode.
    pub(crate) editing_task_id: String,
    /// Page to return to after the task form closes (typically "tasks").
    pub(crate) task_form_previous_page: String,
    /// Plan-grid rows currently displayed, parallel to the Slint grid — the
    /// commit callback indexes into this to recover a row's id (story 2.3).
    pub(crate) plan_rows: Vec<pomone_app::PlanRow>,
    /// Variety-option ids parallel to the grid's variety picker model, so a
    /// picked index resolves back to a `VarietyId` string.
    pub(crate) plan_variety_option_ids: Vec<String>,
    /// Id of the last plan line the grower touched (edited / added / duplicated)
    /// — the Ctrl+D duplication target and the row the grid re-focuses when the
    /// plan screen is reopened (story 2.4 session resume).
    pub(crate) plan_last_edited_id: String,
    /// Stringified `TaskId` targeted by the open skip-reason dialog; empty when
    /// the dialog is closed (story 1.5).
    pub(crate) agenda_skip_target: String,
    /// Closed-set skip-reason keys (`"weather"`, `"pest-disease"`, …) parallel
    /// to the skip dialog's reason ComboBox model.
    pub(crate) agenda_skip_reason_keys: Vec<String>,
    /// Stringified `TaskTypeId`s parallel to the Task Types admin list,
    /// so callbacks emitting just a row id can be routed back to typed IDs.
    pub(crate) task_type_admin_ids: Vec<String>,
    /// Stable category keys (`"sow"`, `"transplant"`, …) parallel to the
    /// `task-types-category-labels` ComboBox model. Index 0 must always
    /// be the first key returned by `list_task_category_options`.
    pub(crate) task_type_category_keys: Vec<String>,
    /// Stringified `TaskTypeId` currently being edited in the catalog
    /// form; empty in create mode.
    pub(crate) editing_task_type_id: String,
    /// Stringified `FamilyId` currently being edited in the Familles form;
    /// empty in create mode.
    pub(crate) editing_family_id: String,
    /// Stringified `CropId` currently edited in the Cultures crop form; empty
    /// in create mode.
    pub(crate) editing_crop_id: String,
    /// Stringified `VarietyId` currently edited in the Cultures variety form;
    /// empty in create mode.
    pub(crate) editing_variety_id: String,
    /// Stringified `LocationId` currently edited in the Lieux form; empty in
    /// create mode.
    pub(crate) editing_location_id: String,
    /// Active categories on the Task Calendar's per-category filter row.
    /// Stored as stable string keys (`"sow"`, `"transplant"`, …) so the
    /// UI doesn't depend on the enum variant order. When this set holds
    /// every category, the calendar query runs with no filter (i.e.
    /// `None` is passed to `list_task_calendar_rows`).
    pub(crate) task_filter_categories: std::collections::HashSet<String>,
    /// Master toggle for the crop-cycle milestone family on the calendar
    /// (the second filter group). `true` = milestones shown.
    pub(crate) show_milestones: bool,
    /// Stable `RecurrenceUnit` keys (`"days"` / `"weeks"` / `"months"`)
    /// parallel to the task form's recurrence-unit ComboBox model.
    pub(crate) task_form_recurrence_unit_keys: Vec<String>,
    /// Stringified `LocationId`s parallel to the Crop Map's move-picker
    /// list AND the split-form ComboBoxes (same underlying ordering).
    pub(crate) crop_map_location_ids: Vec<String>,
}
