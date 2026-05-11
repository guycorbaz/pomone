//! Pomone application crate: services, use cases, application state.
//!
//! Application code (the `pomone-ui` and `pomone-cli` binaries) depends on
//! this crate and through it on the `Repository` abstraction in
//! `pomone-db`. The repository implementation is selected by [`AppConfig`]
//! at runtime.

pub mod app;
pub mod config;
pub mod error;
pub mod i18n;
pub mod plantings_view;
pub mod services;

pub use app::App;
pub use config::{default_config_path, AppConfig, BackendConfig};
pub use error::{AppError, AppResult};
pub use i18n::{I18n, Lang};
pub use plantings_view::{
    list_location_options, list_plantings, list_variety_options, parse_id, parse_iso_date,
    seed_demo, LocationOption, PlantingRow, VarietyOption,
};
