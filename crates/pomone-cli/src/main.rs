//! Pomone admin/debug CLI binary entry point.

use anyhow::{Context, Result};
use chrono::Local;
use clap::Parser;
use pomone_app::{seed_demo_data, App, AppConfig, BackendConfig};

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
    }
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
