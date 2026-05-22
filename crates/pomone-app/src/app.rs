//! [`App`] — the runtime context owning a [`Repository`] and the active
//! configuration.
//!
//! UI and CLI binaries hold a single [`App`] instance and call use-case
//! methods on it (or pass `app.repo()` to free service functions).

use crate::config::{AppConfig, BackendConfig};
use crate::error::AppResult;
use crate::i18n::{I18n, Lang};
use crate::migration::{copy_all, MigrationReport};
use pomone_db::{seed_defaults, MariaDbRepository, Repository, SqliteRepository};

/// Application runtime context.
pub struct App {
    config: AppConfig,
    repo: Box<dyn Repository>,
    i18n: I18n,
}

impl std::fmt::Debug for App {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("App")
            .field("config", &self.config)
            .field("repo", &"<dyn Repository>")
            .field("i18n", &self.i18n)
            .finish()
    }
}

impl App {
    /// Construct an [`App`] by opening (or creating) the database backend
    /// described by `config`, running migrations, seeding default lookup
    /// data, and initialising i18n bundles for the configured language.
    pub async fn new(config: AppConfig) -> AppResult<Self> {
        let repo = build_repo(&config.backend).await?;
        let lang = Lang::parse(&config.language)?;
        let i18n = I18n::new(lang)?;
        Ok(Self { config, repo, i18n })
    }

    /// Construct an [`App`] from an existing [`Repository`] (mostly useful
    /// for tests with `SqliteRepository::in_memory()`).
    pub async fn with_repo(config: AppConfig, repo: Box<dyn Repository>) -> AppResult<Self> {
        seed_defaults(&*repo).await?;
        let lang = Lang::parse(&config.language)?;
        let i18n = I18n::new(lang)?;
        Ok(Self { config, repo, i18n })
    }

    #[must_use]
    pub fn config(&self) -> &AppConfig {
        &self.config
    }

    #[must_use]
    pub fn repo(&self) -> &dyn Repository {
        &*self.repo
    }

    #[must_use]
    pub fn i18n(&self) -> &I18n {
        &self.i18n
    }

    /// Switch the active UI language and update the persisted config copy.
    /// Caller is responsible for saving the config to disk if desired.
    pub fn set_lang(&mut self, lang: Lang) {
        self.i18n.set_lang(lang);
        lang.tag().clone_into(&mut self.config.language);
    }

    /// Swap the active backend in place.
    ///
    /// When `migrate_data` is true, every record from the current
    /// repository is copied into the freshly-opened target before the
    /// swap (the target gets schema migrations but no seed defaults so the
    /// copy doesn't collide with seeded primary keys). When false, the
    /// new backend is seeded with defaults and starts empty otherwise.
    ///
    /// The config field is updated to the new backend and persisted to
    /// the OS-specific config file. The returned `MigrationReport`
    /// records counts per entity (zero entries when `migrate_data` is
    /// false).
    pub async fn swap_backend(
        &mut self,
        new_backend: BackendConfig,
        migrate_data: bool,
    ) -> AppResult<MigrationReport> {
        let new_repo = build_repo_inner(&new_backend, !migrate_data).await?;
        let report = if migrate_data {
            copy_all(&*self.repo, &*new_repo).await?
        } else {
            MigrationReport::default()
        };
        self.repo = new_repo;
        self.config.backend = new_backend;
        self.config.save_default()?;
        Ok(report)
    }
}

async fn build_repo(backend: &BackendConfig) -> AppResult<Box<dyn Repository>> {
    build_repo_inner(backend, true).await
}

/// Backend builder shared by [`App::new`] and [`App::swap_backend`].
///
/// When `seed` is true the lookup tables get the seeded defaults
/// (`seed_defaults`); when false the schema is set up but no seed rows
/// are inserted — that's the path used by data migration, where the
/// caller is about to copy the source's lookup rows verbatim.
async fn build_repo_inner(backend: &BackendConfig, seed: bool) -> AppResult<Box<dyn Repository>> {
    let repo: Box<dyn Repository> = match backend {
        BackendConfig::Sqlite { path } => {
            // SQLite's `create_if_missing` only creates the FILE — its parent
            // directories must already exist. Make sure they do, since the
            // OS-specific data dir is unlikely to be present on first launch.
            if let Some(parent) = path.parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent)?;
                }
            }
            let url = format!("sqlite:{}?mode=rwc", path.display());
            Box::new(SqliteRepository::connect(&url).await?)
        }
        BackendConfig::Mariadb { url } => Box::new(MariaDbRepository::connect(url).await?),
    };
    if seed {
        seed_defaults(&*repo).await?;
    }
    Ok(repo)
}

/// Lightweight probe used by the settings screen to validate credentials
/// before saving them. Opens the backend, hits one cheap read, and lets
/// the connection drop.
pub async fn test_backend(backend: &BackendConfig) -> AppResult<()> {
    let repo = build_repo_inner(backend, false).await?;
    let _ = repo.family_list().await?;
    Ok(())
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

    #[tokio::test]
    async fn i18n_uses_configured_language() {
        let app = fresh_test_app().await;
        // Default config.language is "fr"
        assert_eq!(app.i18n().t("crop"), "Culture");
    }

    #[tokio::test]
    async fn set_lang_updates_translations_and_config() {
        let mut app = fresh_test_app().await;
        app.set_lang(Lang::En);
        assert_eq!(app.i18n().t("crop"), "Crop");
        assert_eq!(app.config().language, "en");
    }

    #[tokio::test]
    async fn invalid_language_in_config_rejected_at_construction() {
        let bad_config = AppConfig {
            backend: BackendConfig::Sqlite {
                path: "::memory::".into(),
            },
            language: "klingon".to_owned(),
        };
        let repo = SqliteRepository::in_memory().await.unwrap();
        let err = App::with_repo(bad_config, Box::new(repo))
            .await
            .unwrap_err();
        assert!(matches!(err, crate::AppError::Config(_)));
    }
}
