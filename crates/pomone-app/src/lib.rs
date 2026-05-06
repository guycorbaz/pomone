//! Pomone application crate: services, use cases, application state.
//!
//! Application code (the `pomone-ui` and `pomone-cli` binaries) depends on
//! this crate and through it on the `Repository` abstraction in
//! `pomone-db`. The repository implementation is selected by [`AppConfig`]
//! at runtime.

pub mod app;
pub mod config;
pub mod error;
pub mod services;

pub use app::App;
pub use config::{default_config_path, AppConfig, BackendConfig};
pub use error::{AppError, AppResult};
