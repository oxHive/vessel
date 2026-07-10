//! End-to-end tests that exercise the real `vessel` binary:
//! `vessel up` over HTTP/SSE and `vessel mcp` over stdio JSON-RPC.
//!
//! Each test runs against an isolated HOME/XDG environment in a tempdir,
//! so no user data is touched and tests can run in parallel.

use serde_json::{Value, json};
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines};

const BIN: &str = env!("CARGO_BIN_EXE_vessel");

struct ChildGuard(std::process::Child);
impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

struct TestEnv {
    _temp: tempfile::TempDir,
    home: std::path::PathBuf,
}

fn test_env() -> TestEnv {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().to_path_buf();
    TestEnv { _temp: temp, home }
}

fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

fn env_vars(env: &TestEnv) -> Vec<(&'static str, std::path::PathBuf)> {
    vec![
        ("HOME", env.home.clone()),
        ("XDG_DATA_HOME", env.home.join(".local/share")),
        ("XDG_CONFIG_HOME", env.home.join(".config")),
    ]
}

fn spawn_up(env: &TestEnv, args: &[&str]) -> ChildGuard {
    let mut cmd = std::process::Command::new(BIN);
    cmd.arg("up")
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    for (k, v) in env_vars(env) {
        cmd.env(k, v);
    }
    ChildGuard(cmd.spawn().unwrap())
}

async fn wait_healthy(client: &reqwest::Client, base: &str) {
    for _ in 0..100 {
        if let Ok(resp) = client.get(format!("{base}/health")).send().await {
            if resp.status().is_success() {
                return;
            }
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("server did not become healthy at {base}");
}

/// Open a second connection to the server's on-disk DB (migrations already
/// ran at server startup) and seed a reviewable generation.
async fn seed_generation(env: &TestEnv) -> (vessel::db::Db, String) {
    let db_path = env.home.join(".local/share/vessel/vessel.db");
    let raw = libsql::Builder::new_local(db_path).build().await.unwrap();
    let db: vessel::db::Db = std::sync::Arc::new(raw);
    let profile =
        vessel::db::profiles::create(&db, "dev", vessel::db::profiles::VoiceSettings::default())
            .await
            .unwrap();
    let project = vessel::db::projects::create(&db, &profile.id, Some("/repo"), None, "local")
        .await
        .unwrap();
    let generation = vessel::db::generations::create(&db, &project.id, "v1.0.0", "release", None)
        .await
        .unwrap();
    vessel::db::generations::save_output(&db, &generation.id, "twitter", "hello world")
        .await
        .unwrap();
    (db, generation.id)
}

#[tokio::test]
async fn binary_serves_full_review_loop_over_http() {
    let env = test_env();
    let port = free_port();
    let _server = spawn_up(&env, &["--port", &port.to_string()]);
    let base = format!("http://127.0.0.1:{port}");
    let client = reqwest::Client::new();
    wait_healthy(&client, &base).await;

    let (_db, gen_id) = seed_generation(&env).await;

    // Embedded SPA fallback serves the dashboard shell for the review route.
    let html = client
        .get(format!("{base}/generation/{gen_id}"))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(
        html.contains("<title>"),
        "SPA shell not served: {html:.200}"
    );

    // Detail endpoint carries outputs and review_state.
    let detail: Value = client
        .get(format!("{base}/api/v1/generations/{gen_id}"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(detail["generation"]["review_state"], "open");
    assert_eq!(detail["outputs"][0]["platform"], "twitter");

    // Subscribe to SSE before triggering events.
    let mut sse = client
        .get(format!("{base}/api/v1/generations/{gen_id}/events"))
        .send()
        .await
        .unwrap();

    // A blocking poll wakes when the browser posts a revision.
    let poll = tokio::spawn({
        let client = client.clone();
        let url = format!("{base}/api/v1/generations/{gen_id}/poll?timeout_ms=15000");
        async move {
            client
                .get(url)
                .send()
                .await
                .unwrap()
                .json::<Value>()
                .await
                .unwrap()
        }
    });
    tokio::time::sleep(Duration::from_millis(300)).await;
    let queued = client
        .post(format!("{base}/api/v1/generations/{gen_id}/revisions"))
        .json(&json!({ "platform": "twitter", "note": "punchier" }))
        .send()
        .await
        .unwrap();
    assert!(queued.status().is_success());

    let poll_body = tokio::time::timeout(Duration::from_secs(10), poll)
        .await
        .expect("poll did not wake")
        .unwrap();
    assert_eq!(poll_body["revisions"][0]["note"], "punchier");
    assert_eq!(poll_body["session_ended"], false);

    // Agent replies, saves (outputs-updated), user finishes review.
    for (path, body) in [
        ("agent-reply", Some(json!({ "message": "revised twitter" }))),
        ("outputs-updated", None),
        ("done", None),
    ] {
        let mut req = client.post(format!("{base}/api/v1/generations/{gen_id}/{path}"));
        if let Some(body) = body {
            req = req.json(&body);
        }
        assert!(req.send().await.unwrap().status().is_success(), "{path}");
    }

    // All three SSE frames arrive with the exact event names the Vue store
    // subscribes to.
    let mut buf = String::new();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    while !(buf.contains("event: agent-reply")
        && buf.contains("event: outputs-updated")
        && buf.contains("event: review-done"))
    {
        let chunk = tokio::time::timeout_at(deadline, sse.chunk())
            .await
            .expect("timed out waiting for SSE frames")
            .unwrap()
            .expect("SSE stream closed early");
        buf.push_str(&String::from_utf8_lossy(&chunk));
    }
    assert!(buf.contains(r#"{"message":"revised twitter"}"#));

    // Done is terminal: the next poll returns session_ended immediately.
    let after: Value = client
        .get(format!(
            "{base}/api/v1/generations/{gen_id}/poll?timeout_ms=2000"
        ))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(after["session_ended"], true);
}

// --- MCP stdio ---

async fn send_rpc(stdin: &mut tokio::process::ChildStdin, msg: Value) {
    let mut line = msg.to_string();
    line.push('\n');
    stdin.write_all(line.as_bytes()).await.unwrap();
    stdin.flush().await.unwrap();
}

async fn read_rpc_response(
    lines: &mut Lines<BufReader<tokio::process::ChildStdout>>,
    id: u64,
) -> Value {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(20);
    loop {
        let line = tokio::time::timeout_at(deadline, lines.next_line())
            .await
            .expect("timed out waiting for MCP response")
            .unwrap()
            .expect("MCP stdout closed");
        if line.trim().is_empty() {
            continue;
        }
        let msg: Value = serde_json::from_str(&line).unwrap();
        if msg["id"] == json!(id) {
            return msg;
        }
        // Skip notifications and unrelated messages.
    }
}

#[tokio::test]
async fn mcp_stdio_exposes_tools_and_returns_queued_feedback() {
    let env = test_env();
    let port = free_port();

    // Both child processes read the same config file, so they agree on the
    // port the MCP poll tool bridges to.
    let cfg_dir = env.home.join(".config/vessel");
    std::fs::create_dir_all(&cfg_dir).unwrap();
    std::fs::write(
        cfg_dir.join("vessel.toml"),
        format!("[server]\nport = {port}\n"),
    )
    .unwrap();

    let _server = spawn_up(&env, &[]);
    let base = format!("http://127.0.0.1:{port}");
    let client = reqwest::Client::new();
    wait_healthy(&client, &base).await;

    let (db, gen_id) = seed_generation(&env).await;
    vessel::db::revisions::queue_from_dashboard(&db, &gen_id, Some("twitter"), "shorter")
        .await
        .unwrap();

    let mut cmd = tokio::process::Command::new(BIN);
    cmd.arg("mcp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    for (k, v) in env_vars(&env) {
        cmd.env(k, v);
    }
    let mut mcp = cmd.spawn().unwrap();
    let mut stdin = mcp.stdin.take().unwrap();
    let mut lines = BufReader::new(mcp.stdout.take().unwrap()).lines();

    send_rpc(
        &mut stdin,
        json!({
            "jsonrpc": "2.0", "id": 1, "method": "initialize",
            "params": {
                "protocolVersion": "2025-03-26",
                "capabilities": {},
                "clientInfo": { "name": "vessel-e2e", "version": "0.0.0" }
            }
        }),
    )
    .await;
    let init = read_rpc_response(&mut lines, 1).await;
    assert_eq!(init["result"]["serverInfo"]["name"], "vessel");

    send_rpc(
        &mut stdin,
        json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }),
    )
    .await;

    send_rpc(
        &mut stdin,
        json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list" }),
    )
    .await;
    let tools = read_rpc_response(&mut lines, 2).await;
    let names: Vec<&str> = tools["result"]["tools"]
        .as_array()
        .expect("tools/list returned no array")
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert!(names.contains(&"vessel_save"), "tools: {names:?}");
    assert!(names.contains(&"vessel_poll_feedback"), "tools: {names:?}");

    // The poll tool bridges over HTTP to the `vessel up` process and drains
    // the queued note immediately.
    send_rpc(
        &mut stdin,
        json!({
            "jsonrpc": "2.0", "id": 3, "method": "tools/call",
            "params": {
                "name": "vessel_poll_feedback",
                "arguments": { "generation_id": gen_id }
            }
        }),
    )
    .await;
    let call = read_rpc_response(&mut lines, 3).await;
    let text = call["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_else(|| panic!("unexpected tools/call result: {call}"));
    let body: Value = serde_json::from_str(text).unwrap();
    assert_eq!(body["revisions"][0]["note"], "shorter");
    assert_eq!(body["revisions"][0]["platform"], "twitter");
    assert_eq!(body["session_ended"], false);
}
