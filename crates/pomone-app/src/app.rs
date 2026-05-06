//! [`App`] — the runtime context owning a [`Repository`] and the active
//! configuration.
//!
//! UI and CLI binaries hold a single [`App`] instance and call use-case
//! methods on it (or pass `app.repo()` to free service functions).

use crate::config::{AppConfig, BackendConfig};
use crate::error::AppResult;
use crate::i18n::{I18n, Lang};
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
        let repo: Box<dyn Repository> = build_repo(&config.backend).await?;
        seed_defaults(&*repo).await?;
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
