use crate::{api::AppState, db::generations as gen_db};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use serde_json::json;

pub async fn list(State(state): State<AppState>) -> Result<Json<serde_json::Value>, StatusCode> {
    let conn = state
        .db
        .connect()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut rows = conn
        .query(
            "SELECT id, project_id, tag, category, context_notes, created_at
         FROM generations ORDER BY created_at DESC LIMIT 50",
            (),
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut gens = vec![];
    while let Some(row) = rows
        .next()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    {
        gens.push(json!({
            "id": row.get::<String>(0).unwrap_or_default(),
            "project_id": row.get::<String>(1).unwrap_or_default(),
            "tag": row.get::<String>(2).unwrap_or_default(),
            "category": row.get::<String>(3).unwrap_or_default(),
            "context_notes": row.get::<Option<String>>(4).unwrap_or(None),
            "created_at": row.get::<i64>(5).unwrap_or_default(),
        }));
    }
    Ok(Json(json!({ "count": gens.len(), "generations": gens })))
}

pub async fn get_one(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    match gen_db::get_with_outputs(&state.db, &id).await {
        Ok(Some((generation, outputs))) => Ok(Json(
            json!({ "generation": generation, "outputs": outputs }),
        )),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn list_outputs(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    match gen_db::get_with_outputs(&state.db, &id).await {
        Ok(Some((_, outputs))) => Ok(Json(json!({ "count": outputs.len(), "outputs": outputs }))),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}
