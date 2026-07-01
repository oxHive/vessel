use crate::generation::prompt::{HiveMindContext, HiveMindMemory};
use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Clone)]
pub struct HiveMindClient {
    base_url: String,
    client: Client,
}

#[derive(Deserialize)]
struct SearchResponse {
    results: Vec<MemoryObject>,
}

#[derive(Deserialize)]
struct MemoryObject {
    title: String,
    content: String,
}

impl HiveMindClient {
    pub fn new(port: u16) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(3))
            .build()
            .unwrap();
        Self {
            base_url: format!("http://localhost:{}/api/v1", port),
            client,
        }
    }

    pub async fn is_available(&self) -> bool {
        self.client
            .get(format!("{}/status", self.base_url))
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    pub async fn read_project_context(&self, repo_path: &str) -> Result<HiveMindContext> {
        // derive a search term from the repo path basename
        let repo_name = std::path::Path::new(repo_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(repo_path);

        let resp: SearchResponse = self
            .client
            .get(format!("{}/search", self.base_url))
            .query(&[("q", repo_name), ("limit", "20")])
            .send()
            .await?
            .json()
            .await?;

        // filter out vessel: prefixed memories (those are written by Vessel itself)
        let memories: Vec<HiveMindMemory> = resp
            .results
            .into_iter()
            .filter(|m| !m.title.starts_with("vessel:"))
            .map(|m| HiveMindMemory {
                title: m.title,
                content: m.content,
            })
            .collect();

        Ok(HiveMindContext { memories })
    }

    pub async fn write_vessel_memory(&self, key: &str, value: &str, repo_name: &str) -> Result<()> {
        #[derive(Serialize)]
        struct CreateMemory<'a> {
            title: String,
            content: &'a str,
            tags: Vec<&'a str>,
        }
        let body = CreateMemory {
            title: format!("vessel:{}", key),
            content: value,
            tags: vec!["vessel", repo_name],
        };
        self.client
            .post(format!("{}/memories", self.base_url))
            .json(&body)
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }
}
