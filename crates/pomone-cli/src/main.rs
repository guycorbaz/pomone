//! Pomone admin/debug CLI binary entry point.

use anyhow::{bail, Context, Result};
use chrono::Local;
use clap::Parser;
use pomone_app::{
    backup_path_for, backup_sqlite, backup_stamp_now, restore_sqlite, seed_demo_data, App,
    AppConfig, BackendConfig,
};
use std::path::{Path, PathBuf};

#[derive(Debug, Parser)]
#[command(name = "pomone-cli", version, about = "Pomone admin/debug tools")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, clap::Subcommand)]
enum Command {
    /// Print version and runtime info.
    Info,
    /// Populate the configured backend with a realistic demo dataset
    /// (5 crops, 7 varieties, 7 locations, 7 plantings, 1 recurring
    /// task series). Bails out if the database already contains any
    /// crop — we don't want to scramble a populated database.
    SeedDemo,
    /// Snapshot the SQLite database to a timestamped `.bak` file (a plain
    /// file copy). Take it while the app is closed. SQLite backend only.
    Backup {
        /// Directory to write the backup into (default: next to the database).
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Restore the SQLite database from a backup file, replacing the current
    /// one. The current database is first snapshotted to `<db>.pre-restore.bak`.
    /// SQLite backend only; run while the app is closed.
    Restore {
        /// Path to the `.bak` file to restore.
        file: PathBuf,
    },
    /// Export the rough plain-text weekly print of the tasks planned this week
    /// (Monday→Sunday of the target date) to a dated file next to the database
    /// (story 1.4 — dogfooding). SQLite default location.
    PrintWeek {
        /// A date inside the target week, `YYYY-MM-DD` (default: today).
        #[arg(long)]
        week: Option<String>,
    },
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    let cli = Cli::parse();
    match cli.command {
        None | Some(Command::Info) => {
            println!("pomone-cli {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Some(Command::SeedDemo) => seed_demo(),
        Some(Command::Backup { output }) => backup(output),
        Some(Command::Restore { file }) => restore(file),
        Some(Command::PrintWeek { week }) => print_week(week),
    }
}

/// Build + write the rough weekly print, then report the path.
fn print_week(week: Option<String>) -> Result<()> {
    use chrono::NaiveDate;

    let reference = match week {
        Some(s) => NaiveDate::parse_from_str(&s, "%Y-%m-%d")
            .with_context(|| format!("invalid --week date {s:?} (expected YYYY-MM-DD)"))?,
        None => Local::now().date_naive(),
    };

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("failed to build tokio runtime")?;
    let config = AppConfig::load_or_default().context("failed to load Pomone config")?;
    let db_path = sqlite_db_path(&config)?;
    let dir = db_path
        .parent()
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf);

    let app = runtime
        .block_on(App::new(config))
        .context("failed to open the backend")?;
    let path = runtime
        .block_on(async {
            pomone_app::printdoc::export_week_sheet(app.repo(), app.i18n(), reference, &dir).await
        })
        .context("failed to export the weekly print")?;

    println!("Weekly print written to {}", path.display());
    Ok(())
}

/// Resolve the SQLite database path from the active config, or bail with a
/// clear message when the backend is MariaDB (use the server's native tools).
fn sqlite_db_path(config: &AppConfig) -> Result<PathBuf> {
    match &config.backend {
        BackendConfig::Sqlite { path } => Ok(path.clone()),
        BackendConfig::Mariadb { .. } => bail!(
            "backup/restore is only supported for the SQLite backend; \
             use your MariaDB server's native tools (e.g. mysqldump)"
        ),
    }
}

fn backup(output: Option<PathBuf>) -> Result<()> {
    let config = AppConfig::load_or_default().context("failed to load Pomone config")?;
    let db_path = sqlite_db_path(&config)?;
    let stamp = backup_stamp_now();
    let dest = match output {
        Some(dir) => {
            let name = db_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("pomone.sqlite");
            dir.join(format!("{name}.{stamp}.bak"))
        }
        None => backup_path_for(&db_path, &stamp),
    };
    backup_sqlite(&db_path, &dest).context("backup failed")?;
    println!("Backup written to {}", dest.display());
    Ok(())
}

fn restore(file: PathBuf) -> Result<()> {
    let config = AppConfig::load_or_default().context("failed to load Pomone config")?;
    let db_path = sqlite_db_path(&config)?;
    let safety = restore_sqlite(&file, &db_path).context("restore failed")?;
    println!(
        "Restored {} → {}\nPrevious database saved to {}",
        file.display(),
        db_path.display(),
        safety.display()
    );
    Ok(())
}

fn seed_demo() -> Result<()> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("failed to build tokio runtime")?;

    let config = AppConfig::load_or_default().context("failed to load Pomone config")?;
    println!("Using backend: {}", backend_label(&config));

    let app = runtime
        .block_on(App::new(config))
        .context("failed to open the backend (migrations / seed defaults)")?;

    let today = Local::now().date_naive();
    let summary = runtime
        .block_on(async { seed_demo_data(app.repo(), today).await })
        .context("failed to seed demo data")?;

    if summary.crops_created == 0 {
        println!(
            "Demo data NOT inserted: the database already contains data (crop_list is non-empty). \
             Drop the database and retry if you really want to start from scratch."
        );
    } else {
        println!("Demo data inserted: {}", summary.one_line());
    }
    Ok(())
}

fn backend_label(config: &AppConfig) -> String {
    match &config.backend {
        BackendConfig::Sqlite { path } => format!("SQLite at {}", path.display()),
        // Strip credentials from the URL before printing.
        BackendConfig::Mariadb { url: _ } => "MariaDB".to_owned(),
    }
}
