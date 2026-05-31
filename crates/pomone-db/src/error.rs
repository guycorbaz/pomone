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

impl DbError {
    /// True when this wraps a foreign-key constraint violation from the
    /// database driver (SQLite or MariaDB). Lets the app layer distinguish
    /// "row is still referenced" from any other DB failure without matching on
    /// backend-specific error codes.
    #[must_use]
    pub fn is_foreign_key_violation(&self) -> bool {
        let DbError::Sqlx(e) = self else {
            return false;
        };
        let Some(db) = e.as_database_error() else {
            return false;
        };
        // sqlx's typed check covers MariaDB (SQLSTATE 23000) but misses
        // SQLite's RESTRICT/FK error, so fall back to the message — both
        // backends spell out "foreign key" in it.
        db.is_foreign_key_violation() || db.message().to_lowercase().contains("foreign key")
    }
}

pub type DbResult<T> = Result<T, DbError>;
