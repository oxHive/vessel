use axum::http::StatusCode;
use axum_test::TestServer;
use std::sync::atomic::{AtomicU64, Ordering};
use vessel::{api, config::VesselConfig, db};

static TEST_DB_COUNTER: AtomicU64 = AtomicU64::new(0);

async fn test_app() -> (TestServer, db::Db) {
    let db = {
        let n = TEST_DB_COUNTER.fetch_add(1, Ordering::SeqCst);
        let pid = std::process::id();
        let path = format!("/tmp/vessel_api_test_{}_{}.db", pid, n);
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
async fn health_returns_ok() {
    let (server, _db) = test_app().await;
    let resp = server.get("/health").await;
    resp.assert_status(StatusCode::OK);
    let body: serde_json::Value = resp.json();
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
async fn generations_list_empty() {
    let (server, _db) = test_app().await;
    let resp = server.get("/api/v1/generations").await;
    resp.assert_status(StatusCode::OK);
    let body: serde_json::Value = resp.json();
    assert_eq!(body["count"], 0);
}

#[tokio::test]
async fn generations_list_includes_review_state() {
    let (server, db) = test_app().await;
    seed_generation(&db).await;

    let resp = server.get("/api/v1/generations").await;
    resp.assert_status(StatusCode::OK);
    let body: serde_json::Value = resp.json();
    assert_eq!(body["generations"][0]["review_state"], "open");
}

#[tokio::test]
async fn generation_response_includes_review_state() {
    let (server, db) = test_app().await;
    let gen_id = seed_generation(&db).await;

    let resp = server
        .get(&format!("/api/v1/generations/{gen_id}"))
        .await;
    resp.assert_status(StatusCode::OK);
    let body: serde_json::Value = resp.json();
    assert_eq!(body["generation"]["review_state"], "open");
}
