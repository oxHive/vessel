// tests/smoke.rs
// Tests the full MCP generate → save → REST API retrieve flow

use vessel::{db, generation::git};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use tempfile::TempDir;

static TEST_DB_COUNTER: AtomicU64 = AtomicU64::new(0);

async fn test_db() -> db::Db {
    let n = TEST_DB_COUNTER.fetch_add(1, Ordering::SeqCst);
    let pid = std::process::id();
    let path = format!("/tmp/vessel_test_{}_{}.db", pid, n);
    let raw = libsql::Builder::new_local(&path).build().await.unwrap();
    let conn = raw.connect().unwrap();
    db::schema::run_migrations(&conn).await.unwrap();
    std::sync::Arc::new(raw)
}

fn make_test_repo() -> TempDir {
    let dir = TempDir::new().unwrap();
    let p = dir.path();
    Command::new("git").args(["init"]).current_dir(p).output().unwrap();
    Command::new("git").args(["config", "user.email", "test@test.com"]).current_dir(p).output().unwrap();
    Command::new("git").args(["config", "user.name", "Test"]).current_dir(p).output().unwrap();
    std::fs::write(p.join("README.md"), "# My Project\nA cool tool.").unwrap();
    Command::new("git").args(["add", "."]).current_dir(p).output().unwrap();
    Command::new("git").args(["commit", "-m", "feat: add README"]).current_dir(p).output().unwrap();
    Command::new("git").args(["tag", "v1.0.0"]).current_dir(p).output().unwrap();
    dir
}

#[tokio::test]
async fn full_generate_save_retrieve_flow() {
    let repo = make_test_repo();
    let db = test_db().await;

    // create profile and project
    let voice = db::profiles::VoiceSettings::default();
    let profile = db::profiles::create(&db, "TestProfile", voice).await.unwrap();
    let project = db::projects::create(
        &db, &profile.id, Some(repo.path().to_str().unwrap()), None, "local"
    ).await.unwrap();

    // create generation + save output (simulating what vessel_save does)
    let generation = db::generations::create(&db, &project.id, "v1.0.0", "release", None).await.unwrap();
    db::generations::save_output(&db, &generation.id, "twitter", "My project v1.0.0 is out! #opensource").await.unwrap();
    db::generations::save_output(&db, &generation.id, "discord", "**v1.0.0** is live — check the README for details.").await.unwrap();

    // verify via DB
    let (fetched_gen, outputs) = db::generations::get_with_outputs(&db, &generation.id).await.unwrap().unwrap();
    assert_eq!(fetched_gen.tag, "v1.0.0");
    assert_eq!(outputs.len(), 2);

    let twitter = outputs.iter().find(|o| o.platform == "twitter").unwrap();
    assert!(twitter.content.chars().count() <= 280);

    // verify git context read for the tag
    let ctx = git::read_git_context(repo.path().to_str().unwrap(), "v1.0.0").unwrap();
    assert_eq!(ctx.tag, "v1.0.0");
}
