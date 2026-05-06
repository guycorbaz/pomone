//! [`App`] — the runtime context owning a [`Repository`] and the active
//! configuration.
//!
//! UI and CLI binaries hold a single [`App`] instance and call use-case
//! methods on it (or pass `app.repo()` to free service functions).

use crate::config::{AppConfig, BackendConfig};
use crate::error::AppResult;
use pomone_db::{seed_defaults, MariaDbRepository, Repository, SqliteRepository};

/// Application runtime context.
pub struct App {
    config: AppConfig,
    repo: Box<dyn Repository>,
}

impl std::fmt::Debug for App {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("App")
            .field("config", &self.config)
            .field("repo", &"<dyn Repository>")
            .finish()
    }
}

impl App {
    /// Construct an [`App`] by opening (or creating) the database backend
    /// described by `config`, running migrations, and seeding default
    /// lookup data on a fresh database.
    pub async fn new(config: AppConfig) -> AppResult<Self> {
        let repo: Box<dyn Repository> = build_repo(&config.backend).await?;
        seed_defaults(&*repo).await?;
        Ok(Self { config, repo })
    }

    /// Construct an [`App`] from an existing [`Repository`] (mostly useful
    /// for tests with `SqliteRepository::in_memory()`).
    pub async fn with_repo(config: AppConfig, repo: Box<dyn Repository>) -> AppResult<Self> {
        seed_defaults(&*repo).await?;
        Ok(Self { config, repo })
    }

    #[must_use]
    pub fn config(&self) -> &AppConfig {
        &self.config
    }

    #[must_use]
    pub fn repo(&self) -> &dyn Repository {
        &*self.repo
    }
}

async fn build_repo(backend: &BackendConfig) -> AppResult<Box<dyn Repository>> {
    match backend {
        BackendConfig::Sqlite { path } => {
            // sqlx URL form: `sqlite:<path>?mode=rwc` (create if missing handled
            // by SqliteRepository::connect via SqliteConnectOptions).
            let url = format!("sqlite:{}?mode=rwc", path.display());
            let repo = SqliteRepository::connect(&url).await?;
            Ok(Box::new(repo))
        }
        BackendConfig::Mariadb { url } => {
            let repo = MariaDbRepository::connect(url).await?;
            Ok(Box::new(repo))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper that produces an in-memory SQLite-backed App for tests.
    pub(crate) async fn fresh_test_app() -> App {
        let config = AppConfig {
            backend: BackendConfig::Sqlite {
                path: "::memory::".into(), // unused in this path
            },
            language: "fr".to_owned(),
        };
        let repo = SqliteRepository::in_memory().await.unwrap();
        App::with_repo(config, Box::new(repo)).await.unwrap()
    }

    #[tokio::test]
    async fn with_repo_seeds_defaults() {
        let app = fresh_test_app().await;
        // seed_defaults should have populated lookup tables
        let strata = app.repo().strata_list().await.unwrap();
        assert!(!strata.is_empty(), "seed_defaults should populate strata");
    }

    #[tokio::test]
    async fn config_is_accessible() {
        let app = fresh_test_app().await;
        assert_eq!(app.config().language, "fr");
    }

    #[tokio::test]
    async fn debug_does_not_leak_repo_internals() {
        let app = fresh_test_app().await;
        let repr = format!("{app:?}");
        assert!(repr.contains("App"));
        assert!(repr.contains("<dyn Repository>"));
    }
}
