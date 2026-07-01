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
        let path = format!("/tmp/vessel_api_settings_test_{}_{}.db", pid, n);
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
async fn get_settings_reports_ports_and_version() {
    let (server, _db) = test_app().await;
    let resp = server.get("/api/v1/settings").await;
    resp.assert_status(StatusCode::OK);
    let body: serde_json::Value = resp.json();
    assert_eq!(body["port"], 3458);
    assert_eq!(body["hivemind_port"], 3456);
    assert_eq!(body["hivemind_available"], false);
    assert_eq!(body["version"], env!("CARGO_PKG_VERSION"));
}

#[tokio::test]
async fn store_and_delete_github_token() {
    let (server, db) = test_app().await;
    let profile = db::profiles::create(&db, "dev", db::profiles::VoiceSettings::default())
        .await
        .unwrap();
    let project = db::projects::create(&db, &profile.id, Some("/repo"), None, "local")
        .await
        .unwrap();

    let store_resp = server
        .post("/api/v1/settings/github-token")
        .json(&json!({ "project_id": project.id, "token": "ghp_test123" }))
        .await;
    store_resp.assert_status(StatusCode::CREATED);
    let stored: serde_json::Value = store_resp.json();
    assert_eq!(stored["stored"], true);

    // Storing again for the same project should overwrite, not error.
    let restore_resp = server
        .post("/api/v1/settings/github-token")
        .json(&json!({ "project_id": project.id, "token": "ghp_updated" }))
        .await;
    restore_resp.assert_status(StatusCode::CREATED);

    let delete_resp = server
        .delete(&format!("/api/v1/settings/github-token/{}", project.id))
        .await;
    delete_resp.assert_status(StatusCode::OK);
    let deleted: serde_json::Value = delete_resp.json();
    assert_eq!(deleted["deleted"], true);
}
