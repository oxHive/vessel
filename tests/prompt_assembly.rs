use vessel::generation::prompt::{PromptRequest, assemble_prompt};
use vessel::generation::git::{GitContext, CommitSummary};
use vessel::db::profiles::Profile;
use vessel::generation::platforms::Platform;

fn test_git_context() -> GitContext {
    GitContext {
        tag: "v1.2.0".into(),
        prev_tag: Some("v1.1.0".into()),
        diff_stat: "3 files changed, 45 insertions(+), 12 deletions(-)".into(),
        commits: vec![
            CommitSummary {
                hash: "abc1234".into(),
                message: "fix: handle empty tags".into(),
                author: "alice".into(),
            },
        ],
        changelog_excerpt: Some("## v1.2.0\n- Fixed empty tag handling\n- Added retry logic".into()),
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
