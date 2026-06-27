use anyhow::Result;
use tower_http::cors::{CorsLayer, Any};
use crate::{config::VesselConfig, db::Db};

pub async fn start(config: VesselConfig, db: Db) -> Result<()> {
    let port = config.server.port;
    let app = crate::api::router(db, config)
        .layer(CorsLayer::new().allow_origin(Any).allow_headers(Any).allow_methods(Any));

    let addr = format!("127.0.0.1:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    println!("Vessel running at http://localhost:{port}");
    println!("Dashboard: http://localhost:{port}");
    println!("MCP config for Claude Code:");
    println!(r#"  {{ "mcpServers": {{ "vessel": {{ "command": "vessel", "args": ["mcp"] }} }} }}"#);
    axum::serve(listener, app).await?;
    Ok(())
}
