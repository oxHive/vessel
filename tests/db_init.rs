use std::sync::atomic::{AtomicU64, Ordering};
use vessel::{config::VesselConfig, db};

static TEST_DB_COUNTER: AtomicU64 = AtomicU64::new(0);

#[tokio::test]
async fn init_creates_db_file_and_runs_migrations() {
    let n = TEST_DB_COUNTER.fetch_add(1, Ordering::SeqCst);
    let pid = std::process::id();
    let dir = std::env::temp_dir().join(format!("vessel_db_init_test_{}_{}", pid, n));
    let mut config = VesselConfig::default();
    config.storage.path = dir.join("vessel.db").to_string_lossy().to_string();

    let vessel_db = db::init(&config).await.unwrap();
    assert!(dir.join("vessel.db").exists());

    // A connection obtained through the helper should be usable and have FK
    // enforcement enabled without erroring.
    let conn = db::connect(&vessel_db).await.unwrap();
    let mut rows = conn
        .query(
            "SELECT name FROM sqlite_master WHERE type='table' AND name='profiles'",
            (),
        )
        .await
        .unwrap();
    assert!(rows.next().await.unwrap().is_some());

    std::fs::remove_dir_all(&dir).ok();
}
