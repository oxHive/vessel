use crate::{api::AppState, db::projects, generation::git};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize)]
pub struct CreateProjectInput {
    pub profile_id: String,
    pub repo_path: Option<String>,
    pub github_repo: Option<String>,
    pub provider: Option<String>,
}

pub async fn list(State(state): State<AppState>) -> Result<Json<serde_json::Value>, StatusCode> {
    let ps = projects::list(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(json!({ "count": ps.len(), "projects": ps })))
}

pub async fn create(
    State(state): State<AppState>,
    Json(input): Json<CreateProjectInput>,
) -> Result<(StatusCode, Json<serde_json::Value>), StatusCode> {
    let project = projects::create(
        &state.db,
        &input.profile_id,
        input.repo_path.as_deref(),
        input.github_repo.as_deref(),
        input.provider.as_deref().unwrap_or("local"),
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok((StatusCode::CREATED, Json(json!({ "id": project.id }))))
}

pub async fn get_one(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let conn = state
        .db
        .connect()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut rows = conn
        .query(
            "SELECT id, profile_id, repo_path, github_repo, provider, created_at
             FROM projects WHERE id=?1",
            [id],
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    match rows
        .next()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    {
        None => Err(StatusCode::NOT_FOUND),
        Some(row) => Ok(Json(json!({
            "id": row.get::<String>(0).unwrap_or_default(),
            "profile_id": row.get::<String>(1).unwrap_or_default(),
            "repo_path": row.get::<Option<String>>(2).unwrap_or(None),
            "github_repo": row.get::<Option<String>>(3).unwrap_or(None),
            "provider": row.get::<String>(4).unwrap_or_default(),
            "created_at": row.get::<i64>(5).unwrap_or_default(),
        }))),
    }
}

pub async fn list_tags(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let conn = state
        .db
        .connect()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut rows = conn
        .query("SELECT repo_path FROM projects WHERE id=?1", [id])
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let repo_path = match rows
        .next()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    {
        None => return Err(StatusCode::NOT_FOUND),
        Some(row) => row.get::<Option<String>>(0).unwrap_or(None),
    };
    match repo_path {
        None => Ok(Json(json!({ "tags": [] }))),
        Some(path) => {
            let tags = git::list_tags(&path).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            Ok(Json(json!({ "tags": tags })))
        }
    }
}
