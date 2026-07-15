//! Pomone database crate: backend-agnostic [`Repository`] trait, with a
//! SQLite implementation today and a future MariaDB implementation.
//!
//! Application code depends on `dyn Repository` so the backend can be swapped
//! based on user configuration without recompiling.

pub(crate) mod codec;
pub mod error;
pub mod mariadb;
pub mod repository;
pub mod seed;
pub mod sqlite;

#[cfg(test)]
mod cross_backend_tests;

pub use error::{DbError, DbResult};
pub use mariadb::MariaDbRepository;
pub use repository::{
    CropPlanLineRepo, CropRepo, FactOutcome, FactsRepo, FamilyRepo, FieldEventRepo, ItkRepo,
    LocationKindRepo, LocationRepo, PlantingRepo, Repository, StrataRepo, TaskImplementRepo,
    TaskMethodRepo, TaskProjection, TaskRepo, TaskSeriesRepo, TaskTypeRepo, TreatmentRepo,
    VarietyRepo, YearlyHarvestRepo,
};
pub use seed::seed_defaults;
pub use sqlite::SqliteRepository;
