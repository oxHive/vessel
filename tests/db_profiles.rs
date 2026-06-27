use vessel::db;
use std::sync::atomic::{AtomicU64, Ordering};

static TEST_DB_COUNTER: AtomicU64 = AtomicU64::new(0);

async fn test_db() -> db::Db {
    let n = TEST_DB_COUNTER.fetch_add(1, Ordering::SeqCst);
    let pid = std::process::id();
    let path = format!("/tmp/vessel_test_{}_{}.db", pid, n);
    let db = libsql::Builder::new_local(&path).build().await.unwrap();
    let conn = db.connect().unwrap();
    db::schema::run_migrations(&conn).await.unwrap();
    std::sync::Arc::new(db)
}

#[tokio::test]
async fn profile_create_and_get() {
    let db = test_db().await;
    let voice = db::profiles::VoiceSettings {
        formality: "professional".into(),
        humor: "none".into(),
        technical_depth: "high".into(),
        self_promotion: "direct".into(),
    };
    let profile = db::profiles::create(&db, "Oxhive", voice).await.unwrap();
    assert!(profile.id.starts_with("profile_"));
    let fetched = db::profiles::get(&db, &profile.id).await.unwrap().unwrap();
    assert_eq!(fetched.name, "Oxhive");
}

#[tokio::test]
async fn profile_list() {
    let db = test_db().await;
    db::profiles::create(&db, "Alice", db::profiles::VoiceSettings::default())
        .await
        .unwrap();
    db::profiles::create(&db, "Bob", db::profiles::VoiceSettings::default())
        .await
        .unwrap();
    let profiles = db::profiles::list(&db).await.unwrap();
    assert_eq!(profiles.len(), 2);
}

#[tokio::test]
async fn profile_get_missing() {
    let db = test_db().await;
    let result = db::profiles::get(&db, "profile_doesnotexist").await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn project_find_by_repo() {
    let db = test_db().await;
    let voice = db::profiles::VoiceSettings::default();
    let profile = db::profiles::create(&db, "test", voice).await.unwrap();
    db::projects::create(&db, &profile.id, Some("/home/user/myrepo"), None, "local")
        .await
        .unwrap();
    let found = db::projects::find_by_repo(&db, "/home/user/myrepo").await.unwrap();
    assert!(found.is_some());
    let project = found.unwrap();
    assert!(project.id.starts_with("project_"));
    assert_eq!(project.repo_path, Some("/home/user/myrepo".into()));
}

#[tokio::test]
async fn project_find_by_repo_missing() {
    let db = test_db().await;
    let found = db::projects::find_by_repo(&db, "/nonexistent").await.unwrap();
    assert!(found.is_none());
}

#[tokio::test]
async fn generation_create_and_list_recent() {
    let db = test_db().await;
    let profile = db::profiles::create(&db, "dev", db::profiles::VoiceSettings::default())
        .await
        .unwrap();
    let project =
        db::projects::create(&db, &profile.id, Some("/repo"), None, "local")
            .await
            .unwrap();
    let generation = db::generations::create(&db, &project.id, "v1.0.0", "release", None)
        .await
        .unwrap();
    assert!(generation.id.starts_with("gen_"));
    let recent = db::generations::list_recent(&db, &project.id, 10)
        .await
        .unwrap();
    assert_eq!(recent.len(), 1);
    assert_eq!(recent[0].tag, "v1.0.0");
}

#[tokio::test]
async fn generation_save_output_and_get_with_outputs() {
    let db = test_db().await;
    let profile = db::profiles::create(&db, "dev", db::profiles::VoiceSettings::default())
        .await
        .unwrap();
    let project =
        db::projects::create(&db, &profile.id, Some("/repo2"), None, "local")
            .await
            .unwrap();
    let generation = db::generations::create(&db, &project.id, "v2.0.0", "release", Some("big release"))
        .await
        .unwrap();
    let out = db::generations::save_output(&db, &generation.id, "twitter", "tweet content")
        .await
        .unwrap();
    assert!(out.id.starts_with("output_"));
    assert_eq!(out.revision_number, 0);
    // Second output on same platform bumps revision
    let out2 = db::generations::save_output(&db, &generation.id, "twitter", "revised tweet")
        .await
        .unwrap();
    assert_eq!(out2.revision_number, 1);

    let result = db::generations::get_with_outputs(&db, &generation.id).await.unwrap();
    assert!(result.is_some());
    let (fetched_gen, outputs) = result.unwrap();
    assert_eq!(fetched_gen.tag, "v2.0.0");
    assert_eq!(fetched_gen.context_notes, Some("big release".into()));
    assert_eq!(outputs.len(), 2);
}

#[tokio::test]
async fn feedback_record_and_list() {
    let db = test_db().await;
    let profile = db::profiles::create(&db, "dev", db::profiles::VoiceSettings::default())
        .await
        .unwrap();
    let project =
        db::projects::create(&db, &profile.id, Some("/repo3"), None, "local")
            .await
            .unwrap();
    let generation = db::generations::create(&db, &project.id, "v3.0.0", "release", None)
        .await
        .unwrap();
    db::feedback::record(&db, &generation.id, "twitter", "liked").await.unwrap();
    db::feedback::record(&db, &generation.id, "linkedin", "reused").await.unwrap();
    let feedbacks = db::feedback::list_for_generation(&db, &generation.id).await.unwrap();
    assert_eq!(feedbacks.len(), 2);
    let signals: Vec<&str> = feedbacks.iter().map(|f| f.signal.as_str()).collect();
    assert!(signals.contains(&"liked"));
    assert!(signals.contains(&"reused"));
    let first = &feedbacks[0];
    assert!(first.id.starts_with("fb_"));
}
