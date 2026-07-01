use axum::{Json, Router, extract::Query, routing::get, routing::post};
use serde_json::{Value, json};
use std::collections::HashMap;
use vessel::hivemind::HiveMindClient;

async fn spawn_mock_server() -> u16 {
    let app = Router::new()
        .route("/api/v1/status", get(|| async { Json(json!({ "status": "ok" })) }))
        .route(
            "/api/v1/search",
            get(|Query(params): Query<HashMap<String, String>>| async move {
                assert_eq!(params.get("q").map(String::as_str), Some("myrepo"));
                Json(json!({
                    "count": 2,
                    "results": [
                        { "id": "mem_1", "title": "Project uses Rust", "content": "Backend is Rust/axum" },
                        { "id": "mem_2", "title": "vessel:audience", "content": "should be filtered out" },
                    ]
                }))
            }),
        )
        .route(
            "/api/v1/memories",
            post(|Json(body): Json<Value>| async move {
                assert_eq!(body["title"], "vessel:audience");
                Json(json!({ "id": "mem_new" }))
            }),
        );

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    port
}

#[tokio::test]
async fn is_available_false_when_nothing_listening() {
    // Port 1 is a privileged, essentially never-bound port on Linux, so
    // connections to it fail immediately without a real service present.
    let client = HiveMindClient::new(1);
    assert!(!client.is_available().await);
}

#[tokio::test]
async fn is_available_true_when_status_endpoint_responds() {
    let port = spawn_mock_server().await;
    let client = HiveMindClient::new(port);
    assert!(client.is_available().await);
}

#[tokio::test]
async fn read_project_context_filters_vessel_prefixed_memories() {
    let port = spawn_mock_server().await;
    let client = HiveMindClient::new(port);
    let ctx = client
        .read_project_context("/home/user/myrepo")
        .await
        .unwrap();
    assert_eq!(ctx.memories.len(), 1);
    assert_eq!(ctx.memories[0].title, "Project uses Rust");
}

#[tokio::test]
async fn write_vessel_memory_posts_prefixed_title() {
    let port = spawn_mock_server().await;
    let client = HiveMindClient::new(port);
    client
        .write_vessel_memory("audience", "developers who ship often", "myrepo")
        .await
        .unwrap();
}
