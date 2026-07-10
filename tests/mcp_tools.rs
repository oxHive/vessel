use std::sync::atomic::{AtomicU64, Ordering};
use vessel::{
    api,
    config::VesselConfig,
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

    let msg = vessel_save(&db, &VesselConfig::default(), input)
        .await
        .unwrap();
    assert!(msg.contains("Saved 2 platform outputs"));
    assert!(msg.contains(&generation.id));
    assert!(msg.contains("vessel_poll_feedback"));

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
    let result = vessel_save(&db, &VesselConfig::default(), input).await;
    assert!(result.is_err());
}

/// Serve the real router on an OS-assigned port; returns a config pointing at it.
async fn live_server(db: &db::Db) -> VesselConfig {
    let mut config = VesselConfig::default();
    let app = api::router(db.clone(), config.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    config.server.port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    config
}

async fn seed_generation(db: &db::Db) -> String {
    let profile = db::profiles::create(db, "dev", db::profiles::VoiceSettings::default())
        .await
        .unwrap();
    let project = db::projects::create(db, &profile.id, Some("/repo"), None, "local")
        .await
        .unwrap();
    let generation = db::generations::create(db, &project.id, "v1.0.0", "release", None)
        .await
        .unwrap();
    generation.id
}

#[tokio::test]
async fn poll_feedback_returns_queued_revisions() {
    let db = test_db().await;
    let gen_id = seed_generation(&db).await;
    let config = live_server(&db).await;
    db::revisions::queue_from_dashboard(&db, &gen_id, Some("twitter"), "punchier")
        .await
        .unwrap();

    let result = vessel::mcp::tools::vessel_poll_feedback(
        &config,
        vessel::mcp::tools::VesselPollInput {
            generation_id: gen_id.clone(),
            agent_reply: Some("first draft saved".into()),
        },
    )
    .await
    .unwrap();

    assert_eq!(result["session_ended"], false);
    assert_eq!(result["revisions"][0]["note"], "punchier");
}

#[tokio::test]
async fn poll_feedback_reports_session_ended() {
    let db = test_db().await;
    let gen_id = seed_generation(&db).await;
    let config = live_server(&db).await;
    db::revisions::set_review_done(&db, &gen_id).await.unwrap();

    let result = vessel::mcp::tools::vessel_poll_feedback(
        &config,
        vessel::mcp::tools::VesselPollInput {
            generation_id: gen_id.clone(),
            agent_reply: None,
        },
    )
    .await
    .unwrap();
    assert_eq!(result["session_ended"], true);
}

#[tokio::test]
async fn poll_feedback_unknown_generation_is_error() {
    let db = test_db().await;
    let config = live_server(&db).await;
    let result = vessel::mcp::tools::vessel_poll_feedback(
        &config,
        vessel::mcp::tools::VesselPollInput {
            generation_id: "gen_missing".into(),
            agent_reply: None,
        },
    )
    .await;
    assert!(result.is_err());
}
