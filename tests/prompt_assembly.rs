use vessel::db::feedback::ContentFeedback;
use vessel::db::profiles::Profile;
use vessel::generation::git::{CommitSummary, GitContext};
use vessel::generation::platforms::Platform;
use vessel::generation::prompt::{HiveMindContext, HiveMindMemory, PromptRequest, assemble_prompt};

fn test_git_context() -> GitContext {
    GitContext {
        tag: "v1.2.0".into(),
        prev_tag: Some("v1.1.0".into()),
        diff_stat: "3 files changed, 45 insertions(+), 12 deletions(-)".into(),
        commits: vec![CommitSummary {
            hash: "abc1234".into(),
            message: "fix: handle empty tags".into(),
            author: "alice".into(),
        }],
        changelog_excerpt: Some(
            "## v1.2.0\n- Fixed empty tag handling\n- Added retry logic".into(),
        ),
    }
}

fn test_profile() -> Profile {
    Profile {
        id: "profile_test".into(),
        name: "Personal".into(),
        formality: "casual".into(),
        humor: "subtle".into(),
        technical_depth: "high".into(),
        self_promotion: "balanced".into(),
        created_at: 0,
        updated_at: 0,
    }
}

#[test]
fn prompt_contains_tag_and_platform_instructions() {
    let req = PromptRequest {
        git_context: test_git_context(),
        profile: test_profile(),
        platforms: vec![Platform::Twitter, Platform::LinkedIn],
        past_feedback: vec![],
        hivemind_context: None,
        context_notes: None,
        generation_id: "gen_test123".into(),
    };
    let prompt = assemble_prompt(&req);
    assert!(prompt.contains("v1.2.0"));
    assert!(prompt.contains("Twitter"));
    assert!(prompt.contains("LinkedIn"));
    assert!(prompt.contains("vessel_save"));
    assert!(prompt.contains("gen_test123"));
}

#[test]
fn prompt_includes_changelog_when_present() {
    let req = PromptRequest {
        git_context: test_git_context(),
        profile: test_profile(),
        platforms: vec![Platform::Discord],
        past_feedback: vec![],
        hivemind_context: None,
        context_notes: Some("Highlight the retry logic addition".into()),
        generation_id: "gen_test456".into(),
    };
    let prompt = assemble_prompt(&req);
    assert!(prompt.contains("retry logic"));
    assert!(prompt.contains("Highlight"));
}

#[test]
fn prompt_includes_hivemind_context_when_present() {
    let req = PromptRequest {
        git_context: test_git_context(),
        profile: test_profile(),
        platforms: vec![Platform::Mastodon],
        past_feedback: vec![],
        hivemind_context: Some(HiveMindContext {
            memories: vec![HiveMindMemory {
                title: "Architecture".into(),
                content: "Backend is Rust/axum".into(),
            }],
        }),
        context_notes: None,
        generation_id: "gen_hivemind".into(),
    };
    let prompt = assemble_prompt(&req);
    assert!(prompt.contains("Project Context (from HiveMind)"));
    assert!(prompt.contains("Architecture"));
    assert!(prompt.contains("Backend is Rust/axum"));
}

#[test]
fn prompt_omits_hivemind_section_when_memories_empty() {
    let req = PromptRequest {
        git_context: test_git_context(),
        profile: test_profile(),
        platforms: vec![Platform::Bluesky],
        past_feedback: vec![],
        hivemind_context: Some(HiveMindContext { memories: vec![] }),
        context_notes: None,
        generation_id: "gen_no_hivemind".into(),
    };
    let prompt = assemble_prompt(&req);
    assert!(!prompt.contains("Project Context (from HiveMind)"));
}

#[test]
fn prompt_includes_past_feedback_signals() {
    let req = PromptRequest {
        git_context: test_git_context(),
        profile: test_profile(),
        platforms: vec![Platform::GitHubRelease],
        past_feedback: vec![
            ContentFeedback {
                id: "fb_1".into(),
                generation_id: "gen_prev".into(),
                platform: "twitter".into(),
                signal: "liked".into(),
                created_at: 0,
            },
            ContentFeedback {
                id: "fb_2".into(),
                generation_id: "gen_prev".into(),
                platform: "linkedin".into(),
                signal: "disliked".into(),
                created_at: 0,
            },
        ],
        hivemind_context: None,
        context_notes: None,
        generation_id: "gen_feedback".into(),
    };
    let prompt = assemble_prompt(&req);
    assert!(prompt.contains("Content History Signals"));
    assert!(prompt.contains("Previously well-received on: twitter"));
    assert!(prompt.contains("Avoid angles that didn't land on: linkedin"));
}
