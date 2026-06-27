use clap::{Parser, Subcommand};
use anyhow::Result;

mod config;
mod db;
mod mcp;
mod generation;
mod hivemind;
mod api;
mod server;

#[derive(Parser)]
#[command(name = "vessel", about = "Developer release announcement tool")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Up,
    Mcp,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();
    let config = config::VesselConfig::load()?;
    let db = db::init(&config).await?;

    match cli.command {
        Commands::Up => server::start(config, db).await,
        Commands::Mcp => mcp::serve(config, db).await,
    }
}
