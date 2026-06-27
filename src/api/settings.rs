use axum::{extract::{State, Path}, Json, http::StatusCode};
use serde::Deserialize;
use serde_json::json;
use crate::{
    api::AppState,
    generation::github::{encrypt_token, derive_encryption_key},
    hivemind::HiveMindClient,
};
use uuid::Uuid;
use chrono::Utc;

pub async fn get(State(state): State<AppState>) -> Json<serde_json::Value> {
    let hivemind_available = HiveMindClient::new(state.config.hivemind.port)
        .is_available()
        .await;
    Json(json!({
        "port": state.config.server.port,
        "hivemind_port": state.config.hivemind.port,
        "hivemind_available": hivemind_available,
        "db_path": state.config.db_path().to_string_lossy(),
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

#[derive(Deserialize)]
pub struct GithubTokenInput {
    pub project_id: String,
    pub token: String,
}

pub async fn store_github_token(
    State(state): State<AppState>,
    Json(input): Json<GithubTokenInput>,
) -> Result<(StatusCode, Json<serde_json::Value>), StatusCode> {
    let key = derive_encryption_key();
    let (enc, nonce) = encrypt_token(&input.token, &key)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let id = format!("ghtoken_{}", Uuid::new_v4().simple());
    let now = Utc::now().timestamp();
    let conn = state.db.connect().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    conn.execute(
        "INSERT INTO github_tokens (id, project_id, token_enc, nonce, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(project_id) DO UPDATE SET token_enc=?3, nonce=?4, created_at=?5",
        libsql::params![id, input.project_id, enc, nonce, now],
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok((StatusCode::CREATED, Json(json!({ "stored": true }))))
}

pub async fn delete_github_token(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let conn = state.db.connect().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    conn.execute(
        "DELETE FROM github_tokens WHERE project_id=?1",
        [project_id],
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(json!({ "deleted": true })))
}
