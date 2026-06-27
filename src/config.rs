use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VesselConfig {
    #[serde(default = "default_server")]
    pub server: ServerConfig,
    #[serde(default = "default_storage")]
    pub storage: StorageConfig,
    #[serde(default = "default_hivemind")]
    pub hivemind: HiveMindConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StorageConfig {
    pub path: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HiveMindConfig {
    pub port: u16,
}

fn default_server() -> ServerConfig { ServerConfig { port: 3458 } }
fn default_storage() -> StorageConfig {
    StorageConfig { path: "~/.vessel/vessel.db".into() }
}
fn default_hivemind() -> HiveMindConfig { HiveMindConfig { port: 3456 } }

impl Default for VesselConfig {
    fn default() -> Self {
        Self {
            server: default_server(),
            storage: default_storage(),
            hivemind: default_hivemind(),
        }
    }
}

impl VesselConfig {
    pub fn load() -> Result<Self> {
        let path = config_path();
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(&path)?;
        Ok(toml::from_str(&content)?)
    }

    pub fn db_path(&self) -> PathBuf {
        let raw = self.storage.path.replace('~', &dirs::home_dir()
            .unwrap_or_default().to_string_lossy());
        PathBuf::from(raw)
    }
}

fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_default())
        .join("vessel")
        .join("vessel.toml")
}
