use anyhow::Result;
use axum::http::{HeaderValue, Method};
use tower_http::cors::CorsLayer;
use crate::{config::VesselConfig, db::Db};

pub async fn start(config: VesselConfig, db: Db) -> Result<()> {
    let port = config.server.port;
    let origin: HeaderValue = format!("http://localhost:{port}").parse()?;
    let cors = CorsLayer::new()
        .allow_origin(origin)
        .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::DELETE])
        .allow_headers(tower_http::cors::Any);
    let app = crate::api::router(db, config).layer(cors);

    let addr = format!("127.0.0.1:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    println!("Vessel running at http://localhost:{port}");
    println!("Dashboard: http://localhost:{port}");
    println!("MCP config for Claude Code:");
    println!(r#"  {{ "mcpServers": {{ "vessel": {{ "command": "vessel", "args": ["mcp"] }} }} }}"#);
    axum::serve(listener, app).await?;
    Ok(())
}
