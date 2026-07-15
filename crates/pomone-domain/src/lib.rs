//! Pomone domain crate: pure types and business rules.
//!
//! This crate must remain free of I/O (no DB, no filesystem, no network).
//! All types are deterministic and easily testable in isolation.
//!
//! # Layout
//!
//! - [`ids`]: strongly-typed UUID newtypes for each entity.
//! - [`error`]: validation errors returned by constructors.
//! - [`validation`] (private): shared validation helpers.
//! - [`family`], [`strata`], [`location_kind`]: small user-managed lookup
//!   tables (with seed data living elsewhere, e.g. `pomone-db`).
//! - [`location`]: physical hierarchical locations (parcels, beds, orchards…).
//! - [`crop`]: crops with their `Lifespan` (annual vs pluriannual).
//! - [`variety`]: varieties of a crop with their cultivation profile.
//! - [`planting`]: concrete plantings with their `PlantingSchedule`.
//! - [`harvest`]: yearly harvest records for perennial plantings.
//! - [`date_calc`]: pure date arithmetic (replaces Qrop's SQL triggers).

#![cfg_attr(not(test), warn(clippy::print_stdout, clippy::print_stderr))]

pub mod crop;
pub mod crop_plan;
pub mod date_calc;
pub mod error;
pub mod family;
pub mod field_event;
pub mod harvest;
pub mod holidays;
pub mod ids;
pub mod location;
pub mod location_kind;
pub mod planting;
pub mod strata;
pub mod task;
pub mod task_series;
pub mod task_type;
pub mod treatment;
pub mod variety;

mod validation;

// Re-export the most commonly used types at the crate root for ergonomics.
pub use crop::{Crop, Lifespan, ProductivePattern, PruningSeason};
pub use crop_plan::CropPlanLine;
pub use error::{DomainError, DomainResult};
pub use family::{Family, DEFAULT_FAMILY_COLOR};
pub use field_event::{skip_payload, FactKind, FieldEvent, SkipReason};
pub use harvest::YearlyHarvest;
pub use holidays::{holiday_on, holidays_in_year, Holiday, HolidayRegion};
pub use ids::{
    CropId, CropPlanLineId, FamilyId, FieldEventId, LocationId, LocationKindId, PlantingId,
    StrataId, TaskId, TaskImplementId, TaskMethodId, TaskSeriesId, TaskTypeId, TreatmentId,
    VarietyId,
};
pub use location::Location;
pub use location_kind::LocationKind;
pub use planting::{Planting, PlantingSchedule, PlantingStatus};
pub use strata::Strata;
pub use task::{Task, TaskDisplayState};
pub use task_series::{RecurrenceRule, RecurrenceUnit, TaskSeries};
pub use task_type::{TaskCategory, TaskImplement, TaskMethod, TaskType};
pub use treatment::Treatment;
pub use variety::{AnnualProfile, PluriannualProfile, Variety, VarietyProfile};
