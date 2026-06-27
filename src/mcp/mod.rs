pub mod prompts;
pub mod tools;

use anyhow::Result;
use rmcp::{ServerHandler, ServiceExt, RoleServer, model::*, tool, transport::stdio};
use rmcp::service::RequestContext;
use rmcp::Error as McpError;
use std::sync::Arc;
use std::collections::HashMap;
use crate::{config::VesselConfig, db::Db};

#[derive(Clone)]
pub struct VesselMcp {
    db: Db,
    config: Arc<VesselConfig>,
}

impl VesselMcp {
    pub fn new(db: Db, config: VesselConfig) -> Self {
        Self { db, config: Arc::new(config) }
    }
}

#[tool(tool_box)]
impl VesselMcp {
    #[tool(description = "Save Vessel-generated content to local storage. Call this after generating platform content from a vessel-generate prompt.")]
    async fn vessel_save(
        &self,
        #[tool(param)]
        generation_id: String,
        #[tool(param)]
        outputs: Vec<tools::PlatformOutput>,
    ) -> String {
        let input = tools::VesselSaveInput { generation_id, outputs };
        match tools::vessel_save(&self.db, input).await {
            Ok(msg) => msg,
            Err(e) => format!("Error saving outputs: {e}"),
        }
    }
}

#[tool(tool_box)]
impl ServerHandler for VesselMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::LATEST,
            capabilities: ServerCapabilities::builder()
                .enable_prompts()
                .enable_tools()
                .build(),
            server_info: Implementation {
                name: "vessel".into(),
                version: env!("CARGO_PKG_VERSION").into(),
            },
            instructions: Some("Vessel release announcement tool. Use /vessel-generate to create social content for a release. Use /vessel-status to see recent activity.".into()),
        }
    }

    async fn list_prompts(
        &self,
        _request: PaginatedRequestParam,
        _ctx: RequestContext<RoleServer>,
    ) -> Result<ListPromptsResult, McpError> {
        Ok(ListPromptsResult {
            next_cursor: None,
            prompts: vec![
                Prompt {
                    name: "vessel-generate".into(),
                    description: Some("Generate platform-optimized release content for a git tag".into()),
                    arguments: Some(vec![
                        PromptArgument { name: "repo_path".into(), description: Some("Absolute path to the git repo (defaults to current directory)".into()), required: Some(false) },
                        PromptArgument { name: "tag".into(), description: Some("Git tag to generate content for (defaults to latest tag)".into()), required: Some(false) },
                        PromptArgument { name: "category".into(), description: Some("release | update | milestone | announcement".into()), required: Some(false) },
                        PromptArgument { name: "context_notes".into(), description: Some("Optional extra context to include in generation".into()), required: Some(false) },
                    ]),
                },
                Prompt {
                    name: "vessel-status".into(),
                    description: Some("Show recent Vessel generations and dashboard link".into()),
                    arguments: None,
                },
                Prompt {
                    name: "vessel-revise".into(),
                    description: Some("Revise previously generated content with new notes".into()),
                    arguments: Some(vec![
                        PromptArgument { name: "generation_id".into(), description: Some("The generation ID to revise".into()), required: Some(true) },
                        PromptArgument { name: "notes".into(), description: Some("Revision instructions".into()), required: Some(true) },
                    ]),
                },
                Prompt {
                    name: "vessel-profile".into(),
                    description: Some("View or describe the active brand voice profile".into()),
                    arguments: None,
                },
            ],
        })
    }

    async fn get_prompt(
        &self,
        req: GetPromptRequestParam,
        _ctx: RequestContext<RoleServer>,
    ) -> Result<GetPromptResult, McpError> {
        let content = match req.name.as_str() {
            "vessel-generate" => {
                // Convert JsonObject (Map<String, Value>) to HashMap<String, String>
                let args: Option<HashMap<String, String>> = req.arguments.map(|m| {
                    m.into_iter()
                        .filter_map(|(k, v)| {
                            v.as_str().map(|s| (k, s.to_string()))
                        })
                        .collect()
                });
                prompts::handle_vessel_generate(&self.db, &self.config, args).await
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?
            }
            "vessel-status" => {
                prompts::handle_vessel_status(&self.db).await
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?
            }
            "vessel-revise" => {
                let args = req.arguments.unwrap_or_default();
                let gen_id = args.get("generation_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                let notes = args.get("notes")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                prompts::handle_vessel_revise(&self.db, &gen_id, &notes).await
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?
            }
            "vessel-profile" => {
                prompts::handle_vessel_profile(&self.db).await
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?
            }
            other => return Err(McpError::invalid_params(format!("Unknown prompt: {other}"), None)),
        };
        Ok(GetPromptResult {
            description: None,
            messages: vec![PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text(content),
            }],
        })
    }
}

pub async fn serve(config: VesselConfig, db: Db) -> Result<()> {
    let server = VesselMcp::new(db, config);
    let service = server.serve(stdio()).await
        .map_err(|e| anyhow::anyhow!("MCP serve error: {e}"))?;
    service.waiting().await
        .map_err(|e| anyhow::anyhow!("MCP wait error: {e}"))?;
    Ok(())
}
