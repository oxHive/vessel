use super::PlatformSpec;

pub fn spec() -> PlatformSpec {
    PlatformSpec {
        name: "LinkedIn",
        char_limit: Some(3000),
        tone_guidance: "Narrative and professional. 'I built X because Y' format performs well. Longer is acceptable — 150-300 words is a good target. First line must hook without seeing 'more'.",
        format_notes: "No markdown. Short paragraphs (1-3 sentences). Line breaks between paragraphs. Optional: bullet list of key changes after opening narrative.",
        hashtag_notes: "3-5 hashtags at the very end on their own line. Mix broad (#developer) and specific (#rustlang).",
    }
}
