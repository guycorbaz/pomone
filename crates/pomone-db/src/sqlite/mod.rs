//! SQLite implementation of the [`Repository`](crate::repository::Repository)
//! trait.

mod crop;
mod crop_plan_line;
mod facts;
mod family;
mod field_event;
mod itk;
mod location;
mod location_kind;
mod planting;
mod strata;
mod task;
mod treatment;
mod variety;
mod yearly_harvest;

use crate::error::DbResult;
use crate::repository::Repository;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use std::str::FromStr;

/// SQLite-backed [`Repository`].
///
/// Construct with [`SqliteRepository::connect`] (file-backed) or
/// [`SqliteRepository::in_memory`] (tests).
#[derive(Debug, Clone)]
pub struct SqliteRepository {
    pub(crate) pool: SqlitePool,
}

impl SqliteRepository {
    /// Open a SQLite database at the given URL.
    ///
    /// Examples:
    /// - `sqlite:./pomone.db?mode=rwc`
    /// - `sqlite::memory:` (single-connection only)
    ///
    /// Foreign-key enforcement is enabled and the schema is migrated.
    pub async fn connect(url: &str) -> DbResult<Self> {
        let opts = SqliteConnectOptions::from_str(url)?
            .foreign_keys(true)
            .create_if_missing(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(8)
            .connect_with(opts)
            .await?;
        sqlx::migrate!("../../migrations/sqlite").run(&pool).await?;
        Ok(Self { pool })
    }

    /// Spin up a fresh in-memory database (single connection, schema migrated).
    /// Convenient for tests; not for production.
    pub async fn in_memory() -> DbResult<Self> {
        let opts = SqliteConnectOptions::from_str("sqlite::memory:")?.foreign_keys(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(opts)
            .await?;
        sqlx::migrate!("../../migrations/sqlite").run(&pool).await?;
        Ok(Self { pool })
    }

    /// Access the underlying pool (escape hatch — prefer the `Repository`
    /// trait methods when possible).
    #[must_use]
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}

impl Repository for SqliteRepository {}
