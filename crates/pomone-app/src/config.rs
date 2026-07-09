//! Application configuration.
//!
//! Configuration is stored as TOML in the OS-specific user config directory:
//! - Linux:   `$XDG_CONFIG_HOME/pomone/config.toml` (defaults to `~/.config/pomone/`)
//! - macOS:   `~/Library/Application Support/pomone/config.toml`
//! - Windows: `%APPDATA%\pomone\config.toml`

use crate::error::{AppError, AppResult};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Top-level application configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppConfig {
    pub backend: BackendConfig,
    /// IETF BCP 47 language tag, e.g. "fr" or "en". Phase 5 will validate.
    pub language: String,
    /// Public-holiday region code (issue #35), e.g. `"ch-vd"` — see
    /// `pomone_domain::HolidayRegion::code`. Empty string = feature off.
    /// Defaults to Vaud (Pomone's primary audience is French-speaking
    /// Switzerland); `#[serde(default)]` keeps pre-#35 config files loading.
    #[serde(default = "default_holiday_region")]
    pub holiday_region: String,
}

fn default_holiday_region() -> String {
    "ch-vd".to_owned()
}

/// Database backend selection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BackendConfig {
    /// File-backed SQLite database. Path is OS-specific by default.
    Sqlite { path: PathBuf },
    /// Connection URL: `mysql://user:pass@host:port/database`.
    Mariadb { url: String },
}

impl AppConfig {
    /// A reasonable default: SQLite in the OS-specific user data directory,
    /// language `fr`.
    #[must_use]
    pub fn default_for_user() -> Self {
        Self {
            backend: BackendConfig::Sqlite {
                path: default_sqlite_path(),
            },
            language: "fr".to_owned(),
            holiday_region: default_holiday_region(),
        }
    }

    /// Load from a specific TOML file.
    pub fn load_from(path: &Path) -> AppResult<Self> {
        let text = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&text)?;
        Ok(config)
    }

    /// Save to a specific TOML file (creates parent directories if needed).
    pub fn save_to(&self, path: &Path) -> AppResult<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let text = toml::to_string_pretty(self)?;
        std::fs::write(path, text)?;
        Ok(())
    }

    /// Load from the OS-specific default location, falling back to default
    /// values if the file does not exist.
    pub fn load_or_default() -> AppResult<Self> {
        let path = default_config_path()?;
        if path.exists() {
            Self::load_from(&path)
        } else {
            Ok(Self::default_for_user())
        }
    }

    /// Save to the OS-specific default location.
    pub fn save_default(&self) -> AppResult<()> {
        let path = default_config_path()?;
        self.save_to(&path)
    }
}

/// Persisted main-window size (logical pixels), so the app reopens at the size
/// the user last left it. Stored in its own file rather than `AppConfig` so it
/// never races the backend-config save path (which `swap_backend` rewrites).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowGeometry {
    pub width: u32,
    pub height: u32,
}

impl WindowGeometry {
    /// Load the saved geometry, or `None` if absent / unreadable / malformed
    /// (any failure just means "use the default size").
    #[must_use]
    pub fn load() -> Option<Self> {
        let path = window_geometry_path().ok()?;
        let text = std::fs::read_to_string(path).ok()?;
        toml::from_str(&text).ok()
    }

    /// Save the geometry to its OS-specific file (creating parent dirs).
    pub fn save(self) -> AppResult<()> {
        let path = window_geometry_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, toml::to_string_pretty(&self)?)?;
        Ok(())
    }
}

/// OS-specific path of the window-geometry file.
pub fn window_geometry_path() -> AppResult<PathBuf> {
    Ok(project_dirs()?.config_dir().join("window.toml"))
}

fn project_dirs() -> AppResult<ProjectDirs> {
    ProjectDirs::from("", "", "pomone")
        .ok_or_else(|| AppError::Config("cannot determine OS user directories".into()))
}

/// OS-specific path of the TOML config file.
pub fn default_config_path() -> AppResult<PathBuf> {
    Ok(project_dirs()?.config_dir().join("config.toml"))
}

/// OS-specific default path of the SQLite database (used by
/// `AppConfig::default_for_user`).
fn default_sqlite_path() -> PathBuf {
    project_dirs().map_or_else(
        |_| PathBuf::from("pomone.sqlite"),
        |d| d.data_dir().join("pomone.sqlite"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn default_uses_sqlite() {
        let cfg = AppConfig::default_for_user();
        assert_eq!(cfg.language, "fr");
        assert!(matches!(cfg.backend, BackendConfig::Sqlite { .. }));
    }

    #[test]
    fn save_load_roundtrip() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.toml");
        let original = AppConfig {
            backend: BackendConfig::Sqlite {
                path: PathBuf::from("/tmp/pomone.sqlite"),
            },
            language: "en".to_owned(),
            holiday_region: "ch-vd".to_owned(),
        };
        original.save_to(&path).unwrap();
        let loaded = AppConfig::load_from(&path).unwrap();
        assert_eq!(loaded, original);
    }

    #[test]
    fn save_creates_parent_directories() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("nested").join("deeper").join("config.toml");
        let cfg = AppConfig::default_for_user();
        cfg.save_to(&path).unwrap();
        assert!(path.exists());
    }

    #[test]
    fn mariadb_backend_serializes_with_kind_tag() {
        let cfg = AppConfig {
            backend: BackendConfig::Mariadb {
                url: "mysql://user@host/db".to_owned(),
            },
            language: "fr".to_owned(),
            holiday_region: "ch-vd".to_owned(),
        };
        let text = toml::to_string(&cfg).unwrap();
        assert!(text.contains("kind = \"mariadb\""));
        assert!(text.contains("url = \"mysql://user@host/db\""));
    }

    #[test]
    fn pre_35_config_without_holiday_region_still_loads() {
        // Config files written before issue #35 lack the field; serde's
        // default must kick in instead of failing deserialization.
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("old.toml");
        std::fs::write(
            &path,
            "language = \"fr\"\n\n[backend]\nkind = \"sqlite\"\npath = \"/tmp/p.sqlite\"\n",
        )
        .unwrap();
        let cfg = AppConfig::load_from(&path).unwrap();
        assert_eq!(cfg.holiday_region, "ch-vd");
    }

    #[test]
    fn malformed_toml_yields_app_error() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("bad.toml");
        std::fs::write(&path, "not = [valid").unwrap();
        let err = AppConfig::load_from(&path).unwrap_err();
        assert!(matches!(err, AppError::TomlDeserialize(_)));
    }

    #[test]
    fn missing_file_yields_io_error() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("missing.toml");
        let err = AppConfig::load_from(&path).unwrap_err();
        assert!(matches!(err, AppError::Io(_)));
    }
}
