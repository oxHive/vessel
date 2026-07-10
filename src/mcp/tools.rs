use crate::config::VesselConfig;
use crate::db::{Db, generations as gen_db};
use anyhow::Result;
use rmcp::schemars;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Deserialize, Serialize, schemars::JsonSchema)]
pub struct PlatformOutput {
    pub platform: String,
    pub content: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct VesselSaveInput {
    pub generation_id: String,
    pub outputs: Vec<PlatformOutput>,
}

pub async fn vessel_save(db: &Db, config: &VesselConfig, input: VesselSaveInput) -> Result<String> {
    if input.outputs.is_empty() {
        return Err(anyhow::anyhow!("outputs cannot be empty"));
    }
    for output in &input.outputs {
        gen_db::save_output(db, &input.generation_id, &output.platform, &output.content).await?;
    }
    notify_outputs_updated(config, &input.generation_id).await;
    Ok(format!(
        "Saved {} platform outputs for generation {}. The user can review at http://localhost:{}. \
         Now call vessel_poll_feedback with this generation_id to wait for their review feedback.",
        input.outputs.len(),
        input.generation_id,
        config.server.port
    ))
}

/// vessel_save runs in the MCP process; the dashboard's SSE channel lives in
/// the `vessel up` process, so bridge over localhost. Fire-and-forget: if the
/// server is down, no browser is watching.
async fn notify_outputs_updated(config: &VesselConfig, generation_id: &str) {
    let url = format!(
        "http://127.0.0.1:{}/api/v1/generations/{}/outputs-updated",
        config.server.port, generation_id
    );
    if let Ok(client) = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
    {
        let _ = client.post(url).send().await;
    }
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct VesselPollInput {
    pub generation_id: String,
    /// One-line status to show in the dashboard before blocking,
    /// e.g. a summary of what was just saved or revised.
    pub agent_reply: Option<String>,
}

pub async fn vessel_poll_feedback(
    config: &VesselConfig,
    input: VesselPollInput,
) -> Result<serde_json::Value> {
    let base = format!("http://127.0.0.1:{}", config.server.port);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(70))
        .build()?;

    ensure_server(&client, &base, config, &input.generation_id).await?;

    if let Some(msg) = &input.agent_reply {
        let _ = client
            .post(format!(
                "{base}/api/v1/generations/{}/agent-reply",
                input.generation_id
            ))
            .json(&serde_json::json!({ "message": msg }))
            .send()
            .await;
    }

    let deadline = Instant::now() + Duration::from_secs(600);
    loop {
        let resp = client
            .get(format!(
                "{base}/api/v1/generations/{}/poll",
                input.generation_id
            ))
            .query(&[("timeout_ms", "55000")])
            .send()
            .await?;
        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            anyhow::bail!("generation {} not found", input.generation_id);
        }
        let body: serde_json::Value = resp.error_for_status()?.json().await?;
        let has_revisions = body["revisions"].as_array().is_some_and(|a| !a.is_empty());
        if has_revisions || body["session_ended"] == true {
            return Ok(body);
        }
        if Instant::now() >= deadline {
            return Ok(serde_json::json!({
                "revisions": [],
                "session_ended": false,
                "timeout": true,
                "hint": "No feedback within 10 minutes. Call vessel_poll_feedback again to keep waiting.",
            }));
        }
        // Server-side timeout elapsed with no feedback; re-poll.
    }
}

async fn ensure_server(
    client: &reqwest::Client,
    base: &str,
    config: &VesselConfig,
    generation_id: &str,
) -> Result<()> {
    if health_ok(client, base).await {
        return Ok(());
    }
    spawn_server_detached()?;
    for _ in 0..20 {
        tokio::time::sleep(Duration::from_millis(500)).await;
        if health_ok(client, base).await {
            open_browser(&format!(
                "http://localhost:{}/generation/{}",
                config.server.port, generation_id
            ));
            return Ok(());
        }
    }
    anyhow::bail!(
        "vessel dashboard is not reachable on port {} and could not be started; \
         run `vessel up` manually, then call vessel_poll_feedback again",
        config.server.port
    )
}

async fn health_ok(client: &reqwest::Client, base: &str) -> bool {
    client
        .get(format!("{base}/health"))
        .timeout(Duration::from_secs(2))
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

fn spawn_server_detached() -> Result<()> {
    use std::process::Stdio;
    let exe = std::env::current_exe()?;
    std::process::Command::new(exe)
        .arg("up")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    Ok(())
}

fn open_browser(url: &str) {
    #[cfg(target_os = "macos")]
    let cmd = "open";
    #[cfg(target_os = "linux")]
    let cmd = "xdg-open";
    #[cfg(target_os = "windows")]
    let cmd = "explorer";
    let _ = std::process::Command::new(cmd).arg(url).spawn();
}
