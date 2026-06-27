use super::PlatformSpec;

pub fn spec() -> PlatformSpec {
    PlatformSpec {
        name: "Mastodon",
        char_limit: Some(500),
        tone_guidance: "Community-first. Self-promotion should be soft and contextual — lead with what it does for others, not what you built. Technical and open source audiences respond well.",
        format_notes: "500 chars (standard instance limit). Single post. No markdown except line breaks. CW not needed for standard release posts.",
        hashtag_notes: "3-5 hashtags at end. Mastodon search is hashtag-driven — use them. #FediDev #OpenSource are common.",
    }
}
