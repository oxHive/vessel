use anyhow::Result;
use clap::{Parser, Subcommand};
use vessel::{config, db, mcp, server};

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
