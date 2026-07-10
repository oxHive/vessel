use std::sync::atomic::{AtomicU64, Ordering};
use vessel::db;

static TEST_DB_COUNTER: AtomicU64 = AtomicU64::new(0);

async fn test_db() -> db::Db {
    let n = TEST_DB_COUNTER.fetch_add(1, Ordering::SeqCst);
    let pid = std::process::id();
    let path = format!("/tmp/vessel_db_revisions_test_{}_{}.db", pid, n);
    let raw = libsql::Builder::new_local(&path).build().await.unwrap();
    let conn = raw.connect().unwrap();
    db::schema::run_migrations(&conn).await.unwrap();
    std::sync::Arc::new(raw)
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
async fn queue_and_drain_pending_notes() {
    let db = test_db().await;
    let gen_id = seed_generation(&db).await;

    db::revisions::queue_from_dashboard(&db, &gen_id, Some("twitter"), "punchier")
        .await
        .unwrap();
    db::revisions::queue_from_dashboard(&db, &gen_id, None, "shorter overall")
        .await
        .unwrap();

    let drained = db::revisions::drain_pending(&db, &gen_id).await.unwrap();
    assert_eq!(drained.len(), 2);
    assert_eq!(drained[0].platform.as_deref(), Some("twitter"));
    assert_eq!(drained[0].note, "punchier");
    assert_eq!(drained[1].platform, None);

    // Second drain is empty — notes were flipped to delivered
    let again = db::revisions::drain_pending(&db, &gen_id).await.unwrap();
    assert!(again.is_empty());
}

#[tokio::test]
async fn drain_ignores_mcp_sourced_delivered_notes() {
    let db = test_db().await;
    let gen_id = seed_generation(&db).await;

    // Simulate the existing revise-prompt insert (defaults: delivered, mcp)
    let conn = db.connect().unwrap();
    conn.execute(
        "INSERT INTO revision_notes (id, generation_id, notes, created_at)
         VALUES ('note_x', ?1, 'via prompt', 0)",
        [gen_id.as_str()],
    )
    .await
    .unwrap();

    let drained = db::revisions::drain_pending(&db, &gen_id).await.unwrap();
    assert!(drained.is_empty());
}

#[tokio::test]
async fn review_state_lifecycle() {
    let db = test_db().await;
    let gen_id = seed_generation(&db).await;

    assert_eq!(
        db::revisions::review_state(&db, &gen_id).await.unwrap(),
        Some("open".to_string())
    );
    assert_eq!(
        db::revisions::review_state(&db, "gen_missing").await.unwrap(),
        None
    );

    assert!(db::revisions::set_review_done(&db, &gen_id).await.unwrap());
    assert_eq!(
        db::revisions::review_state(&db, &gen_id).await.unwrap(),
        Some("done".to_string())
    );
    assert!(!db::revisions::set_review_done(&db, "gen_missing").await.unwrap());
}
