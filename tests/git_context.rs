use std::process::Command;
use tempfile::TempDir;
use vessel::generation::git;

fn make_test_repo() -> TempDir {
    let dir = TempDir::new().unwrap();
    let p = dir.path();
    Command::new("git")
        .args(["init"])
        .current_dir(p)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(p)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(p)
        .output()
        .unwrap();
    std::fs::write(p.join("main.rs"), "fn main() {}").unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(p)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "initial"])
        .current_dir(p)
        .output()
        .unwrap();
    Command::new("git")
        .args(["tag", "v0.1.0"])
        .current_dir(p)
        .output()
        .unwrap();
    std::fs::write(p.join("lib.rs"), "pub fn hello() {}").unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(p)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "add hello fn"])
        .current_dir(p)
        .output()
        .unwrap();
    Command::new("git")
        .args(["tag", "v0.2.0"])
        .current_dir(p)
        .output()
        .unwrap();
    dir
}

#[test]
fn list_tags_returns_sorted() {
    let repo = make_test_repo();
    let tags = git::list_tags(repo.path().to_str().unwrap()).unwrap();
    assert_eq!(tags, vec!["v0.2.0", "v0.1.0"]);
}

#[test]
fn read_context_has_commits_and_diff() {
    let repo = make_test_repo();
    let ctx = git::read_git_context(repo.path().to_str().unwrap(), "v0.2.0").unwrap();
    assert_eq!(ctx.tag, "v0.2.0");
    assert_eq!(ctx.prev_tag, Some("v0.1.0".into()));
    assert!(!ctx.commits.is_empty());
    assert!(!ctx.diff_stat.is_empty());
}

#[test]
fn read_context_extracts_changelog_excerpt_for_tag() {
    let repo = make_test_repo();
    std::fs::write(
        repo.path().join("CHANGELOG.md"),
        "# Changelog\n\n## v0.2.0\n- Added hello fn\n- Fixed nothing\n\n## v0.1.0\n- Initial release\n",
    )
    .unwrap();
    let ctx = git::read_git_context(repo.path().to_str().unwrap(), "v0.2.0").unwrap();
    let excerpt = ctx.changelog_excerpt.expect("expected a changelog excerpt");
    assert!(excerpt.contains("Added hello fn"));
    assert!(!excerpt.contains("Initial release"));
}

#[test]
fn read_context_has_no_changelog_excerpt_without_file() {
    let repo = make_test_repo();
    let ctx = git::read_git_context(repo.path().to_str().unwrap(), "v0.2.0").unwrap();
    assert!(ctx.changelog_excerpt.is_none());
}

#[test]
fn read_context_for_initial_tag_has_no_prev_tag() {
    let repo = make_test_repo();
    let ctx = git::read_git_context(repo.path().to_str().unwrap(), "v0.1.0").unwrap();
    assert_eq!(ctx.prev_tag, None);
    assert_eq!(ctx.diff_stat, "initial release");
}
