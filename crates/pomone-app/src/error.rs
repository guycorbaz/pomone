//! Application-level errors.

use pomone_db::DbError;
use pomone_domain::DomainError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("domain validation error: {0}")]
    Domain(#[from] DomainError),

    #[error("database error: {0}")]
    Db(#[from] DbError),

    #[error("configuration error: {0}")]
    Config(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("TOML serialization error: {0}")]
    TomlSerialize(#[from] toml::ser::Error),

    #[error("TOML parsing error: {0}")]
    TomlDeserialize(#[from] toml::de::Error),

    #[error("entity not found: {kind} {id}")]
    NotFound { kind: &'static str, id: String },

    #[error("inconsistent state: {0}")]
    Inconsistent(String),

    /// The destination of a data migration already holds records. Copying the
    /// source (whose primary keys are reused verbatim) into it would collide
    /// and leave it partially written — so the copy is refused up front.
    #[error("migration target is not empty")]
    MigrationTargetNotEmpty,

    /// A planting carrying real activity (completed tasks or logged labor) was
    /// asked to be deleted. We refuse so its history survives (issue #63); the
    /// caller should mark it terminal (Completed / Failed / Abandoned) instead.
    #[error("planting has recorded activity and cannot be deleted")]
    PlantingHasActivity,
}

pub type AppResult<T> = Result<T, AppError>;
