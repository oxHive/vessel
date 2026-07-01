use anyhow::Result;
use rmcp::schemars;
use serde::{Deserialize, Serialize};
use crate::db::{Db, generations as gen_db};

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

pub async fn vessel_save(db: &Db, input: VesselSaveInput) -> Result<String> {
    if input.outputs.is_empty() {
        return Err(anyhow::anyhow!("outputs cannot be empty"));
    }
    for output in &input.outputs {
        gen_db::save_output(db, &input.generation_id, &output.platform, &output.content).await?;
    }
    Ok(format!(
        "Saved {} platform outputs for generation {}. Open http://localhost:3458 to review.",
        input.outputs.len(),
        input.generation_id
    ))
}
