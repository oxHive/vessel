use super::PlatformSpec;

pub fn spec() -> PlatformSpec {
    PlatformSpec {
        name: "GitHub Release",
        char_limit: None,
        tone_guidance: "Structured changelog format. Permanent documentation. Developers reading this want to know what changed, what broke, and how to upgrade.",
        format_notes: "GitHub Flavored Markdown. Structure: ## What's Changed (bullet list of changes grouped by Added/Fixed/Changed/Removed), ## Breaking Changes (if any), ## Upgrade Notes (if any), ## Full Changelog link.",
        hashtag_notes: "No hashtags.",
    }
}
