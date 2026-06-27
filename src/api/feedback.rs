use axum::{extract::State, Json, http::StatusCode};
use serde::Deserialize;
use crate::{api::AppState, db::feedback};

#[derive(Deserialize)]
pub struct FeedbackInput {
    pub generation_id: String,
    pub platform: String,
    pub signal: String,
}

pub async fn create(
    State(state): State<AppState>,
    Json(input): Json<FeedbackInput>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    feedback::record(&state.db, &input.generation_id, &input.platform, &input.signal)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!({ "recorded": true })))
}
