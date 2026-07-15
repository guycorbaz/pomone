//! MariaDB implementation of the [`Repository`](crate::repository::Repository)
//! trait.
//!
//! Mirrors the SQLite backend semantically. Dialect-only differences
//! (BINARY(16) vs BLOB, native DECIMAL/DATE, ON DUPLICATE KEY UPDATE) are
//! confined to this module and the `migrations/mariadb/` SQL.

mod crop;
mod crop_plan_line;
mod facts;
mod family;
mod field_event;
mod itk;
mod location;
mod location_kind;
mod planned_planting;
mod planting;
mod strata;
mod task;
mod treatment;
mod variety;
mod yearly_harvest;

#[cfg(test)]
pub(crate) mod test_helpers;

use crate::error::DbResult;
use crate::repository::Repository;
use sqlx::mysql::{MySqlPool, MySqlPoolOptions};

#[derive(Debug, Clone)]
pub struct MariaDbRepository {
    pub(crate) pool: MySqlPool,
}

impl MariaDbRepository {
    /// Connect to a MariaDB/MySQL database at the given URL.
    ///
    /// URL format: `mysql://user[:password]@host:port/database`.
    /// The schema is migrated automatically on first connect.
    pub async fn connect(url: &str) -> DbResult<Self> {
        let pool = MySqlPoolOptions::new()
            .max_connections(8)
            .connect(url)
            .await?;
        sqlx::migrate!("../../migrations/mariadb")
            .run(&pool)
            .await?;
        Ok(Self { pool })
    }

    /// Access the underlying pool.
    #[must_use]
    pub fn pool(&self) -> &MySqlPool {
        &self.pool
    }
}

impl Repository for MariaDbRepository {}
