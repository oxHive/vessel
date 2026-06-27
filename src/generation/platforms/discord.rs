use super::PlatformSpec;

pub fn spec() -> PlatformSpec {
    PlatformSpec {
        name: "Discord",
        char_limit: None,
        tone_guidance: "Conversational and immediate. Announcement channel tone: excited but not corporate. Assume the audience already knows the project.",
        format_notes: "No hard limit. Use **bold** for version number and key feature names. Keep to 3-5 sentences for the main body. Optional: bullet list of key changes.",
        hashtag_notes: "No hashtags.",
    }
}
