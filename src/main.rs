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
    Up {
        /// Override the dashboard port from the config file
        #[arg(long)]
        port: Option<u16>,
    },
    Mcp,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();
    let mut config = config::VesselConfig::load()?;

    match cli.command {
        Commands::Up { port } => {
            if let Some(port) = port {
                config.server.port = port;
            }
            let db = db::init(&config).await?;
            server::start(config, db).await
        }
        Commands::Mcp => {
            let db = db::init(&config).await?;
            mcp::serve(config, db).await
        }
    }
}
