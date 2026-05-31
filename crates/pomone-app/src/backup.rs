//! Backup & restore for the SQLite backend (issue #58).
//!
//! Pomone keeps years of records in a single SQLite file. These helpers make a
//! plain file-level snapshot of that database and restore it, so the user has a
//! safety net before destructive operations (and a simple export/import).
//!
//! ## Why a file copy is enough
//!
//! The SQLite backend opens with the default rollback-journal mode (no WAL —
//! see `SqliteRepository::connect`), so every committed row lives in the main
//! `.sqlite` file; the `-journal` sidecar is transient and gone after each
//! commit. A `std::fs::copy` of the main file taken while the app isn't
//! mid-write (it's a single-user desktop app) is therefore a consistent
//! snapshot, with no WAL/SHM sidecars to chase.
//!
//! MariaDB backups are out of scope: use the server's native tooling
//! (`mysqldump`) — the CLI says so rather than pretending to handle it.

use crate::error::{AppError, AppResult};
use std::path::{Path, PathBuf};

/// Build a timestamped backup path next to `db_path`. `stamp` is supplied by
/// the caller (e.g. `"2026-05-31_2204"`) so this stays pure and testable.
///
/// `pomone.sqlite` + `"2026-05-31_2204"` → `pomone.sqlite.2026-05-31_2204.bak`.
#[must_use]
pub fn backup_path_for(db_path: &Path, stamp: &str) -> PathBuf {
    let name = db_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("pomone.sqlite");
    let dir = db_path.parent().unwrap_or_else(|| Path::new("."));
    dir.join(format!("{name}.{stamp}.bak"))
}

/// Copy the SQLite database at `db_path` to `dest`. Intended to run while the
/// app isn't writing. Fails if the source doesn't exist or `dest` already
/// exists (never silently overwrite an existing backup).
pub fn backup_sqlite(db_path: &Path, dest: &Path) -> AppResult<()> {
    if !db_path.exists() {
        return Err(AppError::Config(format!(
            "database file not found: {}",
            db_path.display()
        )));
    }
    if dest.exists() {
        return Err(AppError::Config(format!(
            "backup destination already exists: {}",
            dest.display()
        )));
    }
    if let Some(parent) = dest.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    std::fs::copy(db_path, dest)?;
    Ok(())
}

/// Restore `backup_path` over `db_path`. Before overwriting, the current
/// database (if any) is snapshotted to `<db_path>.pre-restore.bak` and that
/// path is returned, so a mistaken restore is itself reversible.
pub fn restore_sqlite(backup_path: &Path, db_path: &Path) -> AppResult<PathBuf> {
    if !backup_path.exists() {
        return Err(AppError::Config(format!(
            "backup file not found: {}",
            backup_path.display()
        )));
    }
    if let Some(parent) = db_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    let safety = db_path.with_file_name(format!(
        "{}.pre-restore.bak",
        db_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("pomone.sqlite")
    ));
    if db_path.exists() {
        std::fs::copy(db_path, &safety)?;
    }
    std::fs::copy(backup_path, db_path)?;
    Ok(safety)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Unique scratch directory for a test (no external tempfile dependency).
    fn scratch(tag: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("pomone-backup-test-{}-{}", std::process::id(), tag));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn backup_path_appends_stamp_and_keeps_dir() {
        let p = backup_path_for(Path::new("/data/pomone.sqlite"), "2026-05-31_2204");
        assert_eq!(p, PathBuf::from("/data/pomone.sqlite.2026-05-31_2204.bak"));
    }

    #[test]
    fn backup_then_restore_round_trips_contents() {
        let dir = scratch("roundtrip");
        let db = dir.join("pomone.sqlite");
        std::fs::write(&db, b"ORIGINAL").unwrap();

        let dest = backup_path_for(&db, "stamp1");
        backup_sqlite(&db, &dest).unwrap();
        assert_eq!(std::fs::read(&dest).unwrap(), b"ORIGINAL");

        // Mutate the live DB, then restore from the backup.
        std::fs::write(&db, b"CHANGED").unwrap();
        let safety = restore_sqlite(&dest, &db).unwrap();
        assert_eq!(std::fs::read(&db).unwrap(), b"ORIGINAL");
        // The pre-restore snapshot captured the mutated state.
        assert_eq!(std::fs::read(&safety).unwrap(), b"CHANGED");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn backup_refuses_missing_source_and_existing_dest() {
        let dir = scratch("guards");
        let db = dir.join("pomone.sqlite");
        let dest = dir.join("out.bak");

        // Missing source.
        assert!(backup_sqlite(&db, &dest).is_err());

        // Existing destination is never clobbered.
        std::fs::write(&db, b"X").unwrap();
        std::fs::write(&dest, b"PREV").unwrap();
        assert!(backup_sqlite(&db, &dest).is_err());
        assert_eq!(std::fs::read(&dest).unwrap(), b"PREV");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn restore_refuses_missing_backup() {
        let dir = scratch("restore-missing");
        let db = dir.join("pomone.sqlite");
        let missing = dir.join("nope.bak");
        assert!(restore_sqlite(&missing, &db).is_err());
        std::fs::remove_dir_all(&dir).ok();
    }
}
