use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use tempfile::TempDir;
use vessel::{config::VesselConfig, db, mcp::prompts};

static TEST_DB_COUNTER: AtomicU64 = AtomicU64::new(0);

async fn test_db() -> db::Db {
    let n = TEST_DB_COUNTER.fetch_add(1, Ordering::SeqCst);
    let pid = std::process::id();
    let path = format!("/tmp/vessel_mcp_prompts_test_{}_{}.db", pid, n);
    let raw = libsql::Builder::new_local(&path).build().await.unwrap();
    let conn = raw.connect().unwrap();
    db::schema::run_migrations(&conn).await.unwrap();
    std::sync::Arc::new(raw)
}

fn make_test_repo() -> TempDir {
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
    std::fs::write(p.join("README.md"), "# Test Project").unwrap();
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
        .args(["tag", "v1.0.0"])
        .current_dir(p)
        .output()
        .unwrap();
    dir
}

#[tokio::test]
async fn handle_vessel_generate_creates_project_and_prompt() {
    let db = test_db().await;
    let config = VesselConfig::default();
    let repo = make_test_repo();
    let repo_path = repo.path().to_str().unwrap().to_string();

    let mut args = std::collections::HashMap::new();
    args.insert("repo_path".to_string(), repo_path.clone());
    args.insert("category".to_string(), "release".to_string());

    let prompt = prompts::handle_vessel_generate(&db, &config, Some(args))
        .await
        .unwrap();

    assert!(prompt.contains("v1.0.0"));
    assert!(prompt.contains("vessel_save"));

    // Should have auto-created a default profile and project.
    let projects = db::projects::list(&db).await.unwrap();
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0].repo_path, Some(repo_path.clone()));

    // Calling it again should reuse the existing project, not create a new one.
    let mut args2 = std::collections::HashMap::new();
    args2.insert("repo_path".to_string(), repo_path);
    prompts::handle_vessel_generate(&db, &config, Some(args2))
        .await
        .unwrap();
    let projects = db::projects::list(&db).await.unwrap();
    assert_eq!(projects.len(), 1);
}

#[tokio::test]
async fn handle_vessel_generate_errors_without_tags() {
    let db = test_db().await;
    let config = VesselConfig::default();
    let repo = TempDir::new().unwrap();
    let p = repo.path();
    Command::new("git")
        .args(["init"])
        .current_dir(p)
        .output()
        .unwrap();

    let mut args = std::collections::HashMap::new();
    args.insert("repo_path".to_string(), p.to_str().unwrap().to_string());

    let result = prompts::handle_vessel_generate(&db, &config, Some(args)).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn handle_vessel_status_reports_no_projects() {
    let db = test_db().await;
    let status = prompts::handle_vessel_status(&db).await.unwrap();
    assert!(status.contains("No projects configured"));
}

#[tokio::test]
async fn handle_vessel_status_lists_recent_generations() {
    let db = test_db().await;
    let profile = db::profiles::create(&db, "dev", db::profiles::VoiceSettings::default())
        .await
        .unwrap();
    let project = db::projects::create(&db, &profile.id, Some("/some/repo"), None, "local")
        .await
        .unwrap();
    db::generations::create(&db, &project.id, "v1.0.0", "release", None)
        .await
        .unwrap();

    let status = prompts::handle_vessel_status(&db).await.unwrap();
    assert!(status.contains("/some/repo"));
    assert!(status.contains("v1.0.0"));
}

#[tokio::test]
async fn handle_vessel_profile_reports_none_configured() {
    let db = test_db().await;
    let result = prompts::handle_vessel_profile(&db).await.unwrap();
    assert!(result.contains("No brand voice profiles configured"));
}

#[tokio::test]
async fn handle_vessel_profile_lists_configured_profiles() {
    let db = test_db().await;
    db::profiles::create(
        &db,
        "Personal",
        db::profiles::VoiceSettings {
            formality: "casual".into(),
            humor: "present".into(),
            technical_depth: "high".into(),
            self_promotion: "direct".into(),
        },
    )
    .await
    .unwrap();

    let result = prompts::handle_vessel_profile(&db).await.unwrap();
    assert!(result.contains("Personal"));
    assert!(result.contains("casual"));
}

#[tokio::test]
async fn handle_vessel_revise_returns_current_content_and_notes() {
    let db = test_db().await;
    let profile = db::profiles::create(&db, "dev", db::profiles::VoiceSettings::default())
        .await
        .unwrap();
    let project = db::projects::create(&db, &profile.id, Some("/repo"), None, "local")
        .await
        .unwrap();
    let generation = db::generations::create(&db, &project.id, "v1.0.0", "release", None)
        .await
        .unwrap();
    db::generations::save_output(&db, &generation.id, "twitter", "original tweet")
        .await
        .unwrap();

    let result = prompts::handle_vessel_revise(&db, &generation.id, "make it punchier")
        .await
        .unwrap();
    assert!(result.contains("original tweet"));
    assert!(result.contains("make it punchier"));
    assert!(result.contains("vessel_save"));
}

#[tokio::test]
async fn handle_vessel_revise_errors_for_missing_generation() {
    let db = test_db().await;
    let result = prompts::handle_vessel_revise(&db, "gen_doesnotexist", "notes").await;
    assert!(result.is_err());
}
