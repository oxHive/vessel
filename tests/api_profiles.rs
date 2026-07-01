use axum::http::StatusCode;
use axum_test::TestServer;
use serde_json::json;
use std::sync::atomic::{AtomicU64, Ordering};
use vessel::{api, config::VesselConfig, db};

static TEST_DB_COUNTER: AtomicU64 = AtomicU64::new(0);

async fn test_app() -> TestServer {
    let db = {
        let n = TEST_DB_COUNTER.fetch_add(1, Ordering::SeqCst);
        let pid = std::process::id();
        let path = format!("/tmp/vessel_api_profiles_test_{}_{}.db", pid, n);
        let raw = libsql::Builder::new_local(&path).build().await.unwrap();
        let conn = raw.connect().unwrap();
        db::schema::run_migrations(&conn).await.unwrap();
        std::sync::Arc::new(raw)
    };
    let config = VesselConfig::default();
    let app = api::router(db, config);
    TestServer::new(app)
}

#[tokio::test]
async fn create_list_get_and_update_profile() {
    let server = test_app().await;

    let create_resp = server
        .post("/api/v1/profiles")
        .json(&json!({ "name": "Personal", "formality": "casual" }))
        .await;
    create_resp.assert_status(StatusCode::CREATED);
    let created: serde_json::Value = create_resp.json();
    let id = created["id"].as_str().unwrap().to_string();
    assert!(id.starts_with("profile_"));

    let list_resp = server.get("/api/v1/profiles").await;
    list_resp.assert_status(StatusCode::OK);
    let listed: serde_json::Value = list_resp.json();
    assert_eq!(listed["count"], 1);

    let get_resp = server.get(&format!("/api/v1/profiles/{id}")).await;
    get_resp.assert_status(StatusCode::OK);
    let fetched: serde_json::Value = get_resp.json();
    assert_eq!(fetched["name"], "Personal");
    assert_eq!(fetched["formality"], "casual");

    let update_resp = server
        .patch(&format!("/api/v1/profiles/{id}"))
        .json(&json!({ "name": "Personal Updated", "humor": "present" }))
        .await;
    update_resp.assert_status(StatusCode::OK);

    let refetched_resp = server.get(&format!("/api/v1/profiles/{id}")).await;
    let refetched: serde_json::Value = refetched_resp.json();
    assert_eq!(refetched["name"], "Personal Updated");
    assert_eq!(refetched["humor"], "present");
}

#[tokio::test]
async fn get_missing_profile_returns_404() {
    let server = test_app().await;
    let resp = server.get("/api/v1/profiles/profile_doesnotexist").await;
    resp.assert_status(StatusCode::NOT_FOUND);
}
