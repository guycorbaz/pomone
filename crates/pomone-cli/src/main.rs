//! Pomone admin/debug CLI binary entry point.

use clap::Parser;

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
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    let cli = Cli::parse();
    match cli.command {
        Some(Command::Info) | None => {
            println!("pomone-cli {}", env!("CARGO_PKG_VERSION"));
        }
    }
}
