use crate::{
    api::AppState,
    db::profiles::{self, VoiceSettings},
};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize)]
pub struct CreateProfileInput {
    pub name: String,
    pub formality: Option<String>,
    pub humor: Option<String>,
    pub technical_depth: Option<String>,
    pub self_promotion: Option<String>,
}

pub async fn list(State(state): State<AppState>) -> Result<Json<serde_json::Value>, StatusCode> {
    let ps = profiles::list(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(json!({ "count": ps.len(), "profiles": ps })))
}

pub async fn create(
    State(state): State<AppState>,
    Json(input): Json<CreateProfileInput>,
) -> Result<(StatusCode, Json<serde_json::Value>), StatusCode> {
    let voice = VoiceSettings {
        formality: input.formality.unwrap_or_else(|| "balanced".into()),
        humor: input.humor.unwrap_or_else(|| "subtle".into()),
        technical_depth: input.technical_depth.unwrap_or_else(|| "medium".into()),
        self_promotion: input.self_promotion.unwrap_or_else(|| "balanced".into()),
    };
    let profile = profiles::create(&state.db, &input.name, voice)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok((StatusCode::CREATED, Json(json!({ "id": profile.id }))))
}

pub async fn get_one(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    match profiles::get(&state.db, &id).await {
        Ok(Some(p)) => Ok(Json(json!(p))),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(input): Json<CreateProfileInput>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let conn = state
        .db
        .connect()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let now = chrono::Utc::now().timestamp();
    conn.execute(
        "UPDATE profiles SET name=?1, formality=COALESCE(?2,formality), humor=COALESCE(?3,humor),
         technical_depth=COALESCE(?4,technical_depth), self_promotion=COALESCE(?5,self_promotion),
         updated_at=?6 WHERE id=?7",
        libsql::params![
            input.name,
            input.formality,
            input.humor,
            input.technical_depth,
            input.self_promotion,
            now,
            id.clone()
        ],
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(json!({ "updated": true, "id": id })))
}
