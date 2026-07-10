use crate::{api::AppState, db::revisions};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use serde::Deserialize;
use serde_json::json;
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::{Notify, RwLock, broadcast};

#[derive(Clone, Debug)]
pub struct LoopEvent {
    pub kind: &'static str,
    pub payload: String,
}

pub struct LoopState {
    pub notify: Notify,
    pub sse_tx: broadcast::Sender<LoopEvent>,
}

pub type Loops = Arc<RwLock<HashMap<String, Arc<LoopState>>>>;

pub async fn loop_state(loops: &Loops, id: &str) -> Arc<LoopState> {
    if let Some(ls) = loops.read().await.get(id) {
        return ls.clone();
    }
    let mut w = loops.write().await;
    w.entry(id.to_string())
        .or_insert_with(|| {
            Arc::new(LoopState {
                notify: Notify::new(),
                sse_tx: broadcast::channel(32).0,
            })
        })
        .clone()
}

#[derive(Deserialize)]
pub struct PollQuery {
    pub timeout_ms: Option<u64>,
}

pub async fn poll(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<PollQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mut review = revisions::review_state(&state.db, &id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let ls = loop_state(&state.loops, &id).await;
    let deadline =
        Instant::now() + Duration::from_millis(q.timeout_ms.unwrap_or(55_000).min(120_000));

    loop {
        // Register for wakeups BEFORE checking the queue: enable() stores the
        // permit, so a notify_waiters() landing between drain and select is
        // not lost.
        let mut notified = std::pin::pin!(ls.notify.notified());
        notified.as_mut().enable();

        let drained = revisions::drain_pending(&state.db, &id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        if !drained.is_empty() {
            let revisions: Vec<_> = drained
                .iter()
                .map(|r| json!({ "platform": r.platform, "note": r.note }))
                .collect();
            return Ok(Json(
                json!({ "revisions": revisions, "session_ended": false, "timeout": false }),
            ));
        }
        if review == "done" {
            return Ok(Json(
                json!({ "revisions": [], "session_ended": true, "timeout": false }),
            ));
        }
        let now = Instant::now();
        if now >= deadline {
            return Ok(Json(
                json!({ "revisions": [], "session_ended": false, "timeout": true }),
            ));
        }
        tokio::select! {
            _ = notified.as_mut() => {}
            _ = tokio::time::sleep(deadline - now) => {}
        }
        review = revisions::review_state(&state.db, &id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .unwrap_or(review);
    }
}

#[derive(Deserialize)]
pub struct RevisionInput {
    #[serde(default)]
    pub platform: Option<String>,
    pub note: String,
}

pub async fn create_revision(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(input): Json<RevisionInput>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    if input.note.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    revisions::review_state(&state.db, &id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    revisions::queue_from_dashboard(&state.db, &id, input.platform.as_deref(), &input.note)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let ls = loop_state(&state.loops, &id).await;
    ls.notify.notify_waiters();
    Ok(Json(json!({ "queued": true })))
}

pub async fn mark_done(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let updated = revisions::set_review_done(&state.db, &id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if !updated {
        return Err(StatusCode::NOT_FOUND);
    }
    let ls = loop_state(&state.loops, &id).await;
    ls.notify.notify_waiters();
    let _ = ls.sse_tx.send(LoopEvent {
        kind: "review-done",
        payload: "{}".into(),
    });
    Ok(Json(json!({ "done": true })))
}
