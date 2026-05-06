//! Database-layer errors.

use pomone_domain::DomainError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DbError {
    #[error("database error: {0}")]
    Sqlx(#[from] sqlx::Error),

    #[error("migration error: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),

    #[error("entity not found: {kind} {id}")]
    NotFound { kind: &'static str, id: String },

    #[error("domain validation error: {0}")]
    Domain(#[from] DomainError),

    #[error("location hierarchy would form a cycle")]
    HierarchyCycle,

    #[error("database returned malformed data: {0}")]
    Malformed(String),
}

pub type DbResult<T> = Result<T, DbError>;
