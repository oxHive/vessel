use super::PlatformSpec;

pub fn spec() -> PlatformSpec {
    PlatformSpec {
        name: "X (Twitter)",
        char_limit: Some(280),
        tone_guidance: "Punchy, opinionated, direct. Every word earns its place. Threads are fine for longer announcements — break at natural thought boundaries.",
        format_notes: "Single tweet preferred. If over 280 chars, structure as a thread: first tweet is the hook, subsequent tweets expand. No markdown.",
        hashtag_notes: "1-3 hashtags max, placed at end of tweet or thread. Use only well-known tech hashtags (#rustlang, #opensource, #devtools). Never hashtag common words.",
    }
}
