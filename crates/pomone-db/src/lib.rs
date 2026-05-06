//! Pomone database crate: backend-agnostic [`Repository`] trait, with a
//! SQLite implementation today and a future MariaDB implementation.
//!
//! Application code depends on `dyn Repository` so the backend can be swapped
//! based on user configuration without recompiling.

pub mod error;
pub mod repository;
pub mod seed;
pub mod sqlite;

pub use error::{DbError, DbResult};
pub use repository::{
    CropRepo, FamilyRepo, LocationKindRepo, LocationRepo, PlantingRepo, Repository, StrataRepo,
    VarietyRepo, YearlyHarvestRepo,
};
pub use seed::seed_defaults;
pub use sqlite::SqliteRepository;
