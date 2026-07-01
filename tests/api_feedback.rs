use axum::http::StatusCode;
use axum_test::TestServer;
use serde_json::json;
use std::sync::atomic::{AtomicU64, Ordering};
use vessel::{api, config::VesselConfig, db};

static TEST_DB_COUNTER: AtomicU64 = AtomicU64::new(0);

async fn test_app() -> (TestServer, db::Db) {
    let db = {
        let n = TEST_DB_COUNTER.fetch_add(1, Ordering::SeqCst);
        let pid = std::process::id();
        let path = format!("/tmp/vessel_api_feedback_test_{}_{}.db", pid, n);
        let raw = libsql::Builder::new_local(&path).build().await.unwrap();
        let conn = raw.connect().unwrap();
        db::schema::run_migrations(&conn).await.unwrap();
        std::sync::Arc::new(raw)
    };
    let config = VesselConfig::default();
    let app = api::router(db.clone(), config);
    (TestServer::new(app), db)
}

#[tokio::test]
async fn post_feedback_records_signal() {
    let (server, db) = test_app().await;
    let profile = db::profiles::create(&db, "dev", db::profiles::VoiceSettings::default())
        .await
        .unwrap();
    let project = db::projects::create(&db, &profile.id, Some("/repo"), None, "local")
        .await
        .unwrap();
    let generation = db::generations::create(&db, &project.id, "v1.0.0", "release", None)
        .await
        .unwrap();

    let resp = server
        .post("/api/v1/feedback")
        .json(&json!({
            "generation_id": generation.id,
            "platform": "twitter",
            "signal": "liked",
        }))
        .await;
    resp.assert_status(StatusCode::OK);
    let body: serde_json::Value = resp.json();
    assert_eq!(body["recorded"], true);

    let stored = db::feedback::list_for_generation(&db, &generation.id)
        .await
        .unwrap();
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].signal, "liked");
}
