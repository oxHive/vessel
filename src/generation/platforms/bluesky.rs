use super::PlatformSpec;

pub fn spec() -> PlatformSpec {
    PlatformSpec {
        name: "Bluesky",
        char_limit: Some(300),
        tone_guidance: "Early-adopter developer culture. Technical credibility matters. Conversational but precise. Close to early Twitter.",
        format_notes: "Single post, 300 chars max. Link cards render automatically — no need to describe the URL. No markdown.",
        hashtag_notes: "0-2 hashtags. Optional. Bluesky culture is less hashtag-driven than Twitter.",
    }
}
