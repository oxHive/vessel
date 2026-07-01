use std::sync::atomic::{AtomicU64, Ordering};
use vessel::{
    db,
    mcp::tools::{PlatformOutput, VesselSaveInput, vessel_save},
};

static TEST_DB_COUNTER: AtomicU64 = AtomicU64::new(0);

async fn test_db() -> db::Db {
    let n = TEST_DB_COUNTER.fetch_add(1, Ordering::SeqCst);
    let pid = std::process::id();
    let path = format!("/tmp/vessel_mcp_tools_test_{}_{}.db", pid, n);
    let raw = libsql::Builder::new_local(&path).build().await.unwrap();
    let conn = raw.connect().unwrap();
    db::schema::run_migrations(&conn).await.unwrap();
    std::sync::Arc::new(raw)
}

#[tokio::test]
async fn vessel_save_persists_outputs_and_returns_summary() {
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

    let input = VesselSaveInput {
        generation_id: generation.id.clone(),
        outputs: vec![
            PlatformOutput {
                platform: "twitter".into(),
                content: "Shipped v1.0.0!".into(),
            },
            PlatformOutput {
                platform: "discord".into(),
                content: "**v1.0.0** is out.".into(),
            },
        ],
    };

    let msg = vessel_save(&db, input).await.unwrap();
    assert!(msg.contains("Saved 2 platform outputs"));
    assert!(msg.contains(&generation.id));

    let (_gen, outputs) = db::generations::get_with_outputs(&db, &generation.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(outputs.len(), 2);
}

#[tokio::test]
async fn vessel_save_errors_on_empty_outputs() {
    let db = test_db().await;
    let input = VesselSaveInput {
        generation_id: "gen_whatever".into(),
        outputs: vec![],
    };
    let result = vessel_save(&db, input).await;
    assert!(result.is_err());
}
