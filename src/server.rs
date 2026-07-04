use crate::{config::VesselConfig, db::Db};
use anyhow::Result;
use axum::{
    body::Body,
    extract::Request,
    http::{HeaderValue, Method, Response, StatusCode},
    response::IntoResponse,
};
use rust_embed::RustEmbed;
use tower_http::cors::CorsLayer;

#[derive(RustEmbed)]
#[folder = "dashboard/dist/"]
struct DashboardAssets;

async fn serve_spa(req: Request) -> impl IntoResponse {
    let path = req.uri().path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };

    match DashboardAssets::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            Response::builder()
                .header("content-type", mime.as_ref())
                .body(Body::from(content.data.into_owned()))
                .unwrap()
        }
        None => {
            // SPA fallback: unknown paths return index.html for client-side routing
            match DashboardAssets::get("index.html") {
                Some(content) => Response::builder()
                    .header("content-type", "text/html; charset=utf-8")
                    .body(Body::from(content.data.into_owned()))
                    .unwrap(),
                None => Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(Body::empty())
                    .unwrap(),
            }
        }
    }
}

pub async fn start(config: VesselConfig, db: Db) -> Result<()> {
    let port = config.server.port;
    let origin: HeaderValue = format!("http://localhost:{port}").parse()?;
    let cors = CorsLayer::new()
        .allow_origin(origin)
        .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::DELETE])
        .allow_headers(tower_http::cors::Any);

    let app = crate::api::router(db, config)
        .fallback(serve_spa)
        .layer(cors);

    let addr = format!("127.0.0.1:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    println!("Vessel running at http://localhost:{port}");
    println!("Dashboard: http://localhost:{port}");
    println!("MCP config for Claude Code:");
    println!(r#"  {{ "mcpServers": {{ "vessel": {{ "command": "vessel", "args": ["mcp"] }} }} }}"#);
    axum::serve(listener, app).await?;
    Ok(())
}
