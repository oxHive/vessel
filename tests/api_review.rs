use axum::http::StatusCode;
use axum_test::TestServer;
use serde_json::json;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use vessel::{api, config::VesselConfig, db};

static TEST_DB_COUNTER: AtomicU64 = AtomicU64::new(0);

async fn test_app() -> (TestServer, db::Db) {
    let db = {
        let n = TEST_DB_COUNTER.fetch_add(1, Ordering::SeqCst);
        let pid = std::process::id();
        let path = format!("/tmp/vessel_api_review_test_{}_{}.db", pid, n);
        let raw = libsql::Builder::new_local(&path).build().await.unwrap();
        let conn = raw.connect().unwrap();
        db::schema::run_migrations(&conn).await.unwrap();
        std::sync::Arc::new(raw)
    };
    let config = VesselConfig::default();
    let app = api::router(db.clone(), config);
    (TestServer::new(app), db)
}

async fn seed_generation(db: &db::Db) -> String {
    let profile = db::profiles::create(db, "dev", db::profiles::VoiceSettings::default())
        .await
        .unwrap();
    let project = db::projects::create(db, &profile.id, Some("/repo"), None, "local")
        .await
        .unwrap();
    db::generations::create(db, &project.id, "v1.0.0", "release", None)
        .await
        .unwrap()
        .id
}

#[tokio::test]
async fn poll_returns_pending_revisions_immediately() {
    let (server, db) = test_app().await;
    let gen_id = seed_generation(&db).await;
    db::revisions::queue_from_dashboard(&db, &gen_id, Some("twitter"), "punchier")
        .await
        .unwrap();

    let resp = server
        .get(&format!("/api/v1/generations/{gen_id}/poll"))
        .add_query_param("timeout_ms", "1000")
        .await;
    resp.assert_status(StatusCode::OK);
    let body: serde_json::Value = resp.json();
    assert_eq!(body["session_ended"], false);
    assert_eq!(body["timeout"], false);
    assert_eq!(body["revisions"][0]["platform"], "twitter");
    assert_eq!(body["revisions"][0]["note"], "punchier");
}

#[tokio::test]
async fn poll_times_out_when_no_feedback() {
    let (server, db) = test_app().await;
    let gen_id = seed_generation(&db).await;

    let resp = server
        .get(&format!("/api/v1/generations/{gen_id}/poll"))
        .add_query_param("timeout_ms", "200")
        .await;
    resp.assert_status(StatusCode::OK);
    let body: serde_json::Value = resp.json();
    assert_eq!(body["timeout"], true);
    assert_eq!(body["session_ended"], false);
    assert_eq!(body["revisions"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn poll_blocks_then_wakes_on_posted_revision() {
    let (server, db) = test_app().await;
    let gen_id = seed_generation(&db).await;

    let poll = {
        let server = &server;
        let gen_id = gen_id.clone();
        async move {
            server
                .get(&format!("/api/v1/generations/{gen_id}/poll"))
                .add_query_param("timeout_ms", "5000")
                .await
        }
    };
    let post = {
        let server = &server;
        let gen_id = gen_id.clone();
        async move {
            tokio::time::sleep(Duration::from_millis(150)).await;
            server
                .post(&format!("/api/v1/generations/{gen_id}/revisions"))
                .json(&json!({ "platform": null, "note": "tighten the hook" }))
                .await
        }
    };
    let (poll_resp, post_resp) = tokio::join!(poll, post);

    post_resp.assert_status(StatusCode::OK);
    let body: serde_json::Value = poll_resp.json();
    assert_eq!(body["timeout"], false);
    assert_eq!(body["revisions"][0]["note"], "tighten the hook");
    assert_eq!(body["revisions"][0]["platform"], serde_json::Value::Null);
}

#[tokio::test]
async fn done_ends_session_for_current_and_future_polls() {
    let (server, db) = test_app().await;
    let gen_id = seed_generation(&db).await;

    server
        .post(&format!("/api/v1/generations/{gen_id}/done"))
        .await
        .assert_status(StatusCode::OK);

    // Poll after done: immediate session_ended, idempotent
    for _ in 0..2 {
        let resp = server
            .get(&format!("/api/v1/generations/{gen_id}/poll"))
            .add_query_param("timeout_ms", "5000")
            .await;
        let body: serde_json::Value = resp.json();
        assert_eq!(body["session_ended"], true);
    }
}

#[tokio::test]
async fn unknown_generation_is_404_and_empty_note_is_400() {
    let (server, db) = test_app().await;
    let gen_id = seed_generation(&db).await;

    server
        .get("/api/v1/generations/gen_missing/poll")
        .add_query_param("timeout_ms", "100")
        .await
        .assert_status(StatusCode::NOT_FOUND);
    server
        .post("/api/v1/generations/gen_missing/revisions")
        .json(&json!({ "note": "x" }))
        .await
        .assert_status(StatusCode::NOT_FOUND);
    server
        .post(&format!("/api/v1/generations/{gen_id}/revisions"))
        .json(&json!({ "note": "   " }))
        .await
        .assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn agent_reply_and_outputs_updated_accept_known_generation() {
    let (server, db) = test_app().await;
    let gen_id = seed_generation(&db).await;

    // Subscribe directly to the broadcast channel via a first poll touch is not
    // possible from TestServer SSE easily; instead verify the endpoints return
    // 200 and that a subscriber on the loop state receives the events.
    // We reach the loop state through a second router sharing the same maps is
    // not available — so this test asserts endpoint contracts only; channel
    // delivery is covered by the unit-style test below.
    let resp = server
        .post(&format!("/api/v1/generations/{gen_id}/agent-reply"))
        .json(&json!({ "message": "revised twitter, tightened hook" }))
        .await;
    resp.assert_status(StatusCode::OK);

    let resp = server
        .post(&format!("/api/v1/generations/{gen_id}/outputs-updated"))
        .await;
    resp.assert_status(StatusCode::OK);

    // Unknown generation ids must 404, matching poll/revisions/done behavior.
    server
        .post("/api/v1/generations/gen_missing/agent-reply")
        .json(&json!({ "message": "hi" }))
        .await
        .assert_status(StatusCode::NOT_FOUND);
    server
        .post("/api/v1/generations/gen_missing/outputs-updated")
        .await
        .assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn loop_state_broadcasts_events_to_subscribers() {
    use vessel::api::review::{LoopEvent, loop_state};

    let loops: vessel::api::review::Loops = Default::default();
    let ls = loop_state(&loops, "gen_x").await;
    let mut rx = ls.sse_tx.subscribe();

    ls.sse_tx
        .send(LoopEvent {
            kind: "agent-reply",
            payload: r#"{"message":"hi"}"#.into(),
        })
        .unwrap();

    let ev = rx.recv().await.unwrap();
    assert_eq!(ev.kind, "agent-reply");
    assert_eq!(ev.payload, r#"{"message":"hi"}"#);
}
