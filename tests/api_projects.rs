use axum::http::StatusCode;
use axum_test::TestServer;
use serde_json::json;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use tempfile::TempDir;
use vessel::{api, config::VesselConfig, db};

static TEST_DB_COUNTER: AtomicU64 = AtomicU64::new(0);

async fn test_app() -> (TestServer, db::Db) {
    let db = {
        let n = TEST_DB_COUNTER.fetch_add(1, Ordering::SeqCst);
        let pid = std::process::id();
        let path = format!("/tmp/vessel_api_projects_test_{}_{}.db", pid, n);
        let raw = libsql::Builder::new_local(&path).build().await.unwrap();
        let conn = raw.connect().unwrap();
        db::schema::run_migrations(&conn).await.unwrap();
        std::sync::Arc::new(raw)
    };
    let config = VesselConfig::default();
    let app = api::router(db.clone(), config);
    (TestServer::new(app), db)
}

fn make_tagged_repo() -> TempDir {
    let dir = TempDir::new().unwrap();
    let p = dir.path();
    Command::new("git")
        .args(["init"])
        .current_dir(p)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(p)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(p)
        .output()
        .unwrap();
    std::fs::write(p.join("f.txt"), "hi").unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(p)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "initial"])
        .current_dir(p)
        .output()
        .unwrap();
    Command::new("git")
        .args(["tag", "v0.1.0"])
        .current_dir(p)
        .output()
        .unwrap();
    dir
}

#[tokio::test]
async fn create_list_and_get_project() {
    let (server, db) = test_app().await;
    let profile = db::profiles::create(&db, "dev", db::profiles::VoiceSettings::default())
        .await
        .unwrap();

    let create_resp = server
        .post("/api/v1/projects")
        .json(&json!({ "profile_id": profile.id, "repo_path": "/tmp/some-repo" }))
        .await;
    create_resp.assert_status(StatusCode::CREATED);
    let created: serde_json::Value = create_resp.json();
    let id = created["id"].as_str().unwrap().to_string();
    assert!(id.starts_with("project_"));

    let list_resp = server.get("/api/v1/projects").await;
    let listed: serde_json::Value = list_resp.json();
    assert_eq!(listed["count"], 1);

    let get_resp = server.get(&format!("/api/v1/projects/{id}")).await;
    get_resp.assert_status(StatusCode::OK);
    let fetched: serde_json::Value = get_resp.json();
    assert_eq!(fetched["repo_path"], "/tmp/some-repo");
    assert_eq!(fetched["provider"], "local");
}

#[tokio::test]
async fn get_missing_project_returns_404() {
    let (server, _db) = test_app().await;
    let resp = server.get("/api/v1/projects/project_doesnotexist").await;
    resp.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn list_tags_returns_git_tags_for_project() {
    let (server, db) = test_app().await;
    let profile = db::profiles::create(&db, "dev", db::profiles::VoiceSettings::default())
        .await
        .unwrap();
    let repo = make_tagged_repo();
    let project = db::projects::create(
        &db,
        &profile.id,
        Some(repo.path().to_str().unwrap()),
        None,
        "local",
    )
    .await
    .unwrap();

    let resp = server
        .get(&format!("/api/v1/projects/{}/tags", project.id))
        .await;
    resp.assert_status(StatusCode::OK);
    let body: serde_json::Value = resp.json();
    assert_eq!(body["tags"][0], "v0.1.0");
}

#[tokio::test]
async fn list_tags_returns_empty_for_project_without_repo_path() {
    let (server, db) = test_app().await;
    let profile = db::profiles::create(&db, "dev", db::profiles::VoiceSettings::default())
        .await
        .unwrap();
    let project = db::projects::create(&db, &profile.id, None, Some("owner/repo"), "github")
        .await
        .unwrap();

    let resp = server
        .get(&format!("/api/v1/projects/{}/tags", project.id))
        .await;
    resp.assert_status(StatusCode::OK);
    let body: serde_json::Value = resp.json();
    assert_eq!(body["tags"], json!([]));
}

#[tokio::test]
async fn list_tags_for_missing_project_returns_404() {
    let (server, _db) = test_app().await;
    let resp = server
        .get("/api/v1/projects/project_doesnotexist/tags")
        .await;
    resp.assert_status(StatusCode::NOT_FOUND);
}
