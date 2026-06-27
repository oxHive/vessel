pub mod generations;
pub mod profiles;
pub mod projects;
pub mod feedback;
pub mod settings;

use axum::{Router, routing::{get, post, delete}, Json};
use std::sync::Arc;
use serde_json::json;
use crate::{config::VesselConfig, db::Db};

#[derive(Clone)]
pub struct AppState {
    pub db: Db,
    pub config: Arc<VesselConfig>,
}

pub fn router(db: Db, config: VesselConfig) -> Router {
    let state = AppState { db, config: Arc::new(config) };
    Router::new()
        .route("/health", get(health))
        .route("/api/v1/generations", get(generations::list))
        .route("/api/v1/generations/:id", get(generations::get_one))
        .route("/api/v1/generations/:id/outputs", get(generations::list_outputs))
        .route("/api/v1/feedback", post(feedback::create))
        .route("/api/v1/profiles", get(profiles::list).post(profiles::create))
        .route("/api/v1/profiles/:id", get(profiles::get_one).patch(profiles::update))
        .route("/api/v1/projects", get(projects::list).post(projects::create))
        .route("/api/v1/projects/:id", get(projects::get_one))
        .route("/api/v1/projects/:id/tags", get(projects::list_tags))
        .route("/api/v1/settings", get(settings::get))
        .route("/api/v1/settings/github-token", post(settings::store_github_token))
        .route("/api/v1/settings/github-token/:project_id", delete(settings::delete_github_token))
        .with_state(state)
}

async fn health() -> Json<serde_json::Value> {
    Json(json!({ "status": "ok", "version": env!("CARGO_PKG_VERSION") }))
}
