use anyhow::Result;
use chrono::Utc;
use uuid::Uuid;
use std::collections::HashMap;
use crate::{
    config::VesselConfig,
    db::{Db, profiles, projects, generations as gen_db, feedback},
    generation::{
        git,
        platforms::Platform,
        prompt::{PromptRequest, assemble_prompt},
    },
    hivemind::HiveMindClient,
};

pub async fn handle_vessel_generate(
    db: &Db,
    config: &VesselConfig,
    args: Option<HashMap<String, String>>,
) -> Result<String> {
    let args = args.unwrap_or_default();
    let repo_path = args.get("repo_path").cloned()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default().to_string_lossy().to_string());
    let tag = args.get("tag").cloned();
    let category = args.get("category").cloned().unwrap_or_else(|| "release".into());
    let context_notes = args.get("context_notes").cloned();

    // resolve tag — use most recent if not specified
    let tags = git::list_tags(&repo_path)?;
    let tag = match tag {
        Some(t) => t,
        None => tags.first().cloned()
            .ok_or_else(|| anyhow::anyhow!("No git tags found in {}", repo_path))?,
    };

    let git_ctx = git::read_git_context(&repo_path, &tag)?;

    // look up or auto-create project and profile
    let project = match projects::find_by_repo(db, &repo_path).await? {
        Some(p) => p,
        None => {
            let default_profiles = profiles::list(db).await?;
            let profile_id = default_profiles.first()
                .map(|p| p.id.clone())
                .unwrap_or_else(|| {
                    format!("profile_{}", Uuid::new_v4().simple())
                });
            // ensure default profile exists
            if default_profiles.is_empty() {
                profiles::create(db, "Default", profiles::VoiceSettings::default()).await?;
                let profiles = profiles::list(db).await?;
                let pid = profiles[0].id.clone();
                projects::create(db, &pid, Some(&repo_path), None, "local").await?
            } else {
                projects::create(db, &profile_id, Some(&repo_path), None, "local").await?
            }
        }
    };

    let profile = profiles::get(db, &project.profile_id).await?
        .ok_or_else(|| anyhow::anyhow!("Profile {} not found", project.profile_id))?;

    // fetch feedback history
    let recent_gens = gen_db::list_recent(db, &project.id, 10).await?;
    let mut past_feedback = vec![];
    for generation in &recent_gens {
        let mut fb = feedback::list_for_generation(db, &generation.id).await?;
        past_feedback.append(&mut fb);
    }

    // HiveMind context (best-effort)
    let hivemind_ctx = HiveMindClient::new(config.hivemind.port)
        .read_project_context(&repo_path).await.ok();

    // create generation record
    let generation = gen_db::create(db, &project.id, &tag, &category, context_notes.as_deref()).await?;

    let req = PromptRequest {
        git_context: git_ctx,
        profile,
        platforms: Platform::all_v1(),
        past_feedback,
        hivemind_context: hivemind_ctx,
        context_notes,
        generation_id: generation.id,
    };

    Ok(assemble_prompt(&req))
}

pub async fn handle_vessel_status(db: &Db) -> Result<String> {
    let projects = crate::db::projects::list(db).await?;
    if projects.is_empty() {
        return Ok("No projects configured. Run `/vessel-generate` in a git repo to get started.\nDashboard: http://localhost:3458".into());
    }
    let mut lines = vec!["## Vessel Status".to_string(), String::new()];
    for project in &projects {
        let gens = gen_db::list_recent(db, &project.id, 3).await?;
        let repo = project.repo_path.as_deref().unwrap_or(project.github_repo.as_deref().unwrap_or("unknown"));
        lines.push(format!("**{}**", repo));
        for generation in gens {
            lines.push(format!("  - {} `{}` ({})", generation.category, generation.tag,
                chrono::DateTime::from_timestamp(generation.created_at, 0)
                    .map(|d| d.format("%Y-%m-%d").to_string())
                    .unwrap_or_default()));
        }
    }
    lines.push(String::new());
    lines.push("Dashboard: http://localhost:3458".into());
    Ok(lines.join("\n"))
}

pub async fn handle_vessel_revise(db: &Db, gen_id: &str, notes: &str) -> Result<String> {
    let (generation, outputs) = gen_db::get_with_outputs(db, gen_id).await?
        .ok_or_else(|| anyhow::anyhow!("Generation {} not found", gen_id))?;

    // store revision notes
    let note_id = format!("note_{}", Uuid::new_v4().simple());
    let now = Utc::now().timestamp();
    let conn = db.connect()?;
    conn.execute(
        "INSERT INTO revision_notes (id, generation_id, notes, created_at) VALUES (?1, ?2, ?3, ?4)",
        libsql::params![note_id, gen_id, notes, now],
    ).await?;

    let mut lines = vec![
        format!("## Vessel Revision — `{}` tag `{}`", generation.category, generation.tag),
        String::new(),
        format!("**Revision notes:** {notes}"),
        String::new(),
        "## Current Content".into(),
    ];
    for output in &outputs {
        lines.push(format!("\n### {} (revision {})", output.platform, output.revision_number));
        lines.push(output.content.clone());
    }
    lines.push(String::new());
    lines.push("## Instructions".into());
    lines.push("Revise the content above for each platform, applying the revision notes.".into());
    lines.push(format!("When done, call `vessel_save` with `generation_id: \"{}\"` and the revised outputs.", gen_id));

    Ok(lines.join("\n"))
}

pub async fn handle_vessel_profile(db: &Db) -> Result<String> {
    let profiles = profiles::list(db).await?;
    if profiles.is_empty() {
        return Ok("No brand voice profiles configured. Visit http://localhost:3458/profiles to create one.".into());
    }
    let mut lines = vec!["## Vessel Brand Voice Profiles".to_string(), String::new()];
    for p in &profiles {
        lines.push(format!("**{}** ({})", p.name, p.id));
        lines.push(format!("  Formality: {} | Humor: {} | Technical depth: {} | Self-promotion: {}",
            p.formality, p.humor, p.technical_depth, p.self_promotion));
    }
    lines.push(String::new());
    lines.push("Manage profiles at: http://localhost:3458/profiles".into());
    Ok(lines.join("\n"))
}
