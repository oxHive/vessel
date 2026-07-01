use crate::db::{feedback::ContentFeedback, profiles::Profile};
use crate::generation::{git::GitContext, platforms::Platform};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct HiveMindContext {
    pub memories: Vec<HiveMindMemory>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HiveMindMemory {
    pub title: String,
    pub content: String,
}

pub struct PromptRequest {
    pub git_context: GitContext,
    pub profile: Profile,
    pub platforms: Vec<Platform>,
    pub past_feedback: Vec<ContentFeedback>,
    pub hivemind_context: Option<HiveMindContext>,
    pub context_notes: Option<String>,
    pub generation_id: String,
}

pub fn assemble_prompt(req: &PromptRequest) -> String {
    let mut parts: Vec<String> = vec![];

    parts.push("You are generating release announcement content for a developer project.".into());
    parts.push(format!("Generation ID: `{}`", req.generation_id));
    parts.push(String::new());

    // Project context from HiveMind
    if let Some(ref hm) = req.hivemind_context {
        if !hm.memories.is_empty() {
            parts.push("## Project Context (from HiveMind)".into());
            for mem in &hm.memories {
                parts.push(format!("**{}**: {}", mem.title, mem.content));
            }
            parts.push(String::new());
        }
    }

    // Brand voice
    parts.push("## Brand Voice Profile".into());
    parts.push(format!("Profile: {}", req.profile.name));
    parts.push(format!("- Formality: {}", req.profile.formality));
    parts.push(format!("- Humor: {}", req.profile.humor));
    parts.push(format!(
        "- Technical depth: {}",
        req.profile.technical_depth
    ));
    parts.push(format!(
        "- Self-promotion comfort: {}",
        req.profile.self_promotion
    ));
    parts.push(String::new());

    // Past feedback signals
    if !req.past_feedback.is_empty() {
        parts.push("## Content History Signals".into());
        let liked: Vec<_> = req
            .past_feedback
            .iter()
            .filter(|f| f.signal == "liked" || f.signal == "reused")
            .collect();
        let disliked: Vec<_> = req
            .past_feedback
            .iter()
            .filter(|f| f.signal == "disliked")
            .collect();
        if !liked.is_empty() {
            parts.push(format!(
                "Previously well-received on: {}",
                liked
                    .iter()
                    .map(|f| f.platform.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        if !disliked.is_empty() {
            parts.push(format!(
                "Avoid angles that didn't land on: {}",
                disliked
                    .iter()
                    .map(|f| f.platform.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        parts.push(String::new());
    }

    // Git context
    parts.push("## Release Context".into());
    parts.push(format!("Tag: `{}`", req.git_context.tag));
    if let Some(ref prev) = req.git_context.prev_tag {
        parts.push(format!("Previous tag: `{prev}`"));
    }
    parts.push(format!("Changes: {}", req.git_context.diff_stat));
    parts.push(String::new());

    if !req.git_context.commits.is_empty() {
        parts.push("Commits since last release:".into());
        for c in &req.git_context.commits {
            parts.push(format!("- `{}` {} ({})", c.hash, c.message, c.author));
        }
        parts.push(String::new());
    }

    if let Some(ref cl) = req.git_context.changelog_excerpt {
        parts.push("Changelog excerpt:".into());
        parts.push(format!("```\n{cl}\n```"));
        parts.push(String::new());
    }

    if let Some(ref notes) = req.context_notes {
        parts.push("## Additional Context from Developer".into());
        parts.push(notes.clone());
        parts.push(String::new());
    }

    // Platform instructions
    parts.push("## Output Required".into());
    parts.push(format!(
        "Generate content for {} platform(s):",
        req.platforms.len()
    ));
    for platform in &req.platforms {
        let spec = platform.spec();
        parts.push(String::new());
        parts.push(format!("### {}", spec.name));
        parts.push(format!("Tone: {}", spec.tone_guidance));
        parts.push(format!("Format: {}", spec.format_notes));
        if let Some(limit) = spec.char_limit {
            parts.push(format!(
                "Character limit: {} chars HARD LIMIT — do not exceed.",
                limit
            ));
        }
        parts.push(format!("Hashtags: {}", spec.hashtag_notes));
    }

    parts.push(String::new());
    parts.push("## Instructions".into());
    parts.push(
        "1. Generate content for each platform above following its exact constraints.".into(),
    );
    parts.push("2. Platform tone guidance overrides general tone when they conflict.".into());
    parts.push("3. Respect character limits strictly — count carefully.".into());
    parts.push(format!("4. When done, call the `vessel_save` tool with:"));
    parts.push(format!("   - `generation_id`: `\"{}\"`", req.generation_id));
    parts.push("   - `outputs`: array of `{ platform: string, content: string }` objects".into());
    parts.push(format!(
        "   - Platform slug values: {}",
        req.platforms
            .iter()
            .map(|p| format!("`{}`", p.slug()))
            .collect::<Vec<_>>()
            .join(", ")
    ));

    parts.join("\n")
}
