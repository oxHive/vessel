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
    /// Explicit database path (supports `~`). When unset, vessel resolves an
    /// XDG-compliant default with a fallback to the legacy `~/.vessel` location.
    pub path: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HiveMindConfig {
    pub port: u16,
}

fn default_server() -> ServerConfig {
    ServerConfig { port: 3458 }
}
fn default_storage() -> StorageConfig {
    StorageConfig { path: None }
}
fn default_hivemind() -> HiveMindConfig {
    HiveMindConfig { port: 3456 }
}

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
        match &self.storage.path {
            Some(raw) => expand_tilde(raw),
            None => resolve_default_db_path(
                dirs::data_dir(),
                dirs::home_dir().unwrap_or_default(),
            ),
        }
    }
}

fn expand_tilde(raw: &str) -> PathBuf {
    PathBuf::from(raw.replace('~', &dirs::home_dir().unwrap_or_default().to_string_lossy()))
}

/// Default DB location: the platform data dir (`$XDG_DATA_HOME`/`~/.local/share`
/// on Linux), unless only a legacy `~/.vessel/vessel.db` from an older install
/// exists — then the legacy path keeps working so upgrades don't orphan data.
pub fn resolve_default_db_path(data_dir: Option<PathBuf>, home: PathBuf) -> PathBuf {
    let xdg = data_dir
        .unwrap_or_else(|| home.join(".local").join("share"))
        .join("vessel")
        .join("vessel.db");
    if xdg.exists() {
        return xdg;
    }
    let legacy = home.join(".vessel").join("vessel.db");
    if legacy.exists() {
        static LEGACY_WARN: std::sync::Once = std::sync::Once::new();
        LEGACY_WARN.call_once(|| {
            tracing::warn!(
                "using legacy database at {}; move it to {} (or set storage.path in vessel.toml) to adopt the XDG location",
                legacy.display(),
                xdg.display()
            );
        });
        return legacy;
    }
    xdg
}

fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_default())
        .join("vessel")
        .join("vessel.toml")
}
