pub mod prompts;
pub mod tools;

use crate::{config::VesselConfig, db::Db};
use anyhow::Result;
use rmcp::{
    ErrorData, RoleServer, ServerHandler, ServiceExt,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::*,
    service::RequestContext,
    tool, tool_handler, tool_router,
    transport::stdio,
};
use std::sync::Arc;

#[derive(Clone)]
pub struct VesselMcp {
    db: Db,
    config: Arc<VesselConfig>,
    tool_router: ToolRouter<VesselMcp>,
}

impl VesselMcp {
    pub fn new(db: Db, config: VesselConfig) -> Self {
        Self {
            db,
            config: Arc::new(config),
            tool_router: Self::tool_router(),
        }
    }
}

#[tool_router]
impl VesselMcp {
    #[tool(
        description = "Save Vessel-generated content to local storage. Call this after generating platform content from a vessel-generate prompt."
    )]
    async fn vessel_save(
        &self,
        Parameters(input): Parameters<tools::VesselSaveInput>,
    ) -> Result<CallToolResult, ErrorData> {
        match tools::vessel_save(&self.db, input).await {
            Ok(msg) => Ok(CallToolResult::success(vec![ContentBlock::text(msg)])),
            Err(e) => Err(ErrorData::internal_error(e.to_string(), None)),
        }
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for VesselMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(
            ServerCapabilities::builder()
                .enable_prompts()
                .enable_tools()
                .build(),
        )
        .with_server_info(Implementation::new("vessel", env!("CARGO_PKG_VERSION")))
        .with_instructions("Vessel release announcement tool. Use /vessel-generate to create social content for a release. Use /vessel-status to see recent activity.")
    }

    async fn list_prompts(
        &self,
        _request: Option<PaginatedRequestParams>,
        _ctx: RequestContext<RoleServer>,
    ) -> Result<ListPromptsResult, ErrorData> {
        Ok(ListPromptsResult {
            next_cursor: None,
            prompts: vec![
                Prompt::new(
                    "vessel-generate",
                    Some("Generate platform-optimized release content for a git tag"),
                    Some(vec![
                        PromptArgument::new("repo_path")
                            .with_description(
                                "Absolute path to the git repo (defaults to current directory)",
                            )
                            .with_required(false),
                        PromptArgument::new("tag")
                            .with_description(
                                "Git tag to generate content for (defaults to latest tag)",
                            )
                            .with_required(false),
                        PromptArgument::new("category")
                            .with_description("release | update | milestone | announcement")
                            .with_required(false),
                        PromptArgument::new("context_notes")
                            .with_description("Optional extra context to include in generation")
                            .with_required(false),
                    ]),
                ),
                Prompt::new(
                    "vessel-status",
                    Some("Show recent Vessel generations and dashboard link"),
                    None,
                ),
                Prompt::new(
                    "vessel-revise",
                    Some("Revise previously generated content with new notes"),
                    Some(vec![
                        PromptArgument::new("generation_id")
                            .with_description("The generation ID to revise")
                            .with_required(true),
                        PromptArgument::new("notes")
                            .with_description("Revision instructions")
                            .with_required(true),
                    ]),
                ),
                Prompt::new(
                    "vessel-profile",
                    Some("View or describe the active brand voice profile"),
                    None,
                ),
            ],
            meta: None,
        })
    }

    async fn get_prompt(
        &self,
        req: GetPromptRequestParams,
        _ctx: RequestContext<RoleServer>,
    ) -> Result<GetPromptResult, ErrorData> {
        let content = match req.name.as_str() {
            "vessel-generate" => {
                // Convert JsonObject (Map<String, Value>) to HashMap<String, String>
                let args: Option<std::collections::HashMap<String, String>> =
                    req.arguments.map(|m| {
                        m.into_iter()
                            .filter_map(|(k, v)| v.as_str().map(|s| (k, s.to_string())))
                            .collect()
                    });
                prompts::handle_vessel_generate(&self.db, &self.config, args)
                    .await
                    .map_err(|e| ErrorData::internal_error(e.to_string(), None))?
            }
            "vessel-status" => prompts::handle_vessel_status(&self.db)
                .await
                .map_err(|e| ErrorData::internal_error(e.to_string(), None))?,
            "vessel-revise" => {
                let args = req.arguments.unwrap_or_default();
                let gen_id = args
                    .get("generation_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                let notes = args
                    .get("notes")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                prompts::handle_vessel_revise(&self.db, &gen_id, &notes)
                    .await
                    .map_err(|e| ErrorData::internal_error(e.to_string(), None))?
            }
            "vessel-profile" => prompts::handle_vessel_profile(&self.db)
                .await
                .map_err(|e| ErrorData::internal_error(e.to_string(), None))?,
            other => {
                return Err(ErrorData::invalid_params(
                    format!("Unknown prompt: {other}"),
                    None,
                ));
            }
        };
        Ok(GetPromptResult::new(vec![PromptMessage::new_text(
            Role::User,
            content,
        )]))
    }
}

pub async fn serve(config: VesselConfig, db: Db) -> Result<()> {
    let server = VesselMcp::new(db, config);
    let service = server
        .serve(stdio())
        .await
        .map_err(|e| anyhow::anyhow!("MCP serve error: {e}"))?;
    service
        .waiting()
        .await
        .map_err(|e| anyhow::anyhow!("MCP wait error: {e}"))?;
    Ok(())
}
