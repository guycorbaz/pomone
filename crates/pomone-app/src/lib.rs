//! Pomone application crate: services, use cases, application state.
//!
//! Application code (the `pomone-ui` and `pomone-cli` binaries) depends on
//! this crate and through it on the `Repository` abstraction in
//! `pomone-db`. The repository implementation is selected by [`AppConfig`]
//! at runtime.

pub mod app;
pub mod calendar_view;
pub mod config;
pub mod cultures_view;
pub mod error;
pub mod i18n;
pub mod locations_view;
pub mod plantings_view;
pub mod services;

#[cfg(test)]
mod test_helpers;

pub use app::App;
pub use calendar_view::{list_events_in_range, CalendarEvent, CalendarEventKind};
pub use config::{default_config_path, AppConfig, BackendConfig};
pub use cultures_view::{
    create_crop, create_variety, list_crops, list_family_options, list_strata_options,
    list_varieties_for_crop, CropInput, CropRow, FamilyOption, LifespanKind, StrataOption,
    VarietyInput, VarietyProfileKind, VarietyRow,
};
pub use error::{AppError, AppResult};
pub use i18n::{I18n, Lang};
pub use locations_view::{
    create_location, list_location_kind_options, list_locations_tree, list_parent_options,
    LocationInput, LocationKindOption, LocationListItem, ParentLocationOption,
};
pub use plantings_view::{
    list_location_options, list_plantings, list_variety_options, parse_id, parse_iso_date,
    LocationOption, PlantingRow, VarietyOption,
};
