use anyhow::Result;
use crate::generation::prompt::HiveMindContext;

/// Minimal stub for Task 7. Task 8 will replace the internals.
pub struct HiveMindClient {
    port: u16,
}

impl HiveMindClient {
    pub fn new(port: u16) -> Self {
        Self { port }
    }

    pub async fn read_project_context(&self, _repo_path: &str) -> Result<HiveMindContext> {
        // Task 8 will implement real HTTP communication.
        // For now, return an error so .ok() gives None in the caller.
        let _ = self.port;
        Err(anyhow::anyhow!("HiveMind client not yet implemented"))
    }
}
