use anyhow::{Context, Result};
use git2::{Repository, Sort};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitSummary {
    pub hash: String,
    pub message: String,
    pub author: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitContext {
    pub tag: String,
    pub prev_tag: Option<String>,
    pub diff_stat: String,
    pub commits: Vec<CommitSummary>,
    pub changelog_excerpt: Option<String>,
}

pub fn list_tags(repo_path: &str) -> Result<Vec<String>> {
    let repo = Repository::open(repo_path)
        .with_context(|| format!("opening repo at {repo_path}"))?;
    let tag_names = repo.tag_names(None)?;
    let mut tags: Vec<String> = tag_names.iter().flatten().map(String::from).collect();
    // sort descending by semver-like (simple lexicographic for now)
    tags.sort_by(|a, b| b.cmp(a));
    Ok(tags)
}

pub fn read_git_context(repo_path: &str, tag: &str) -> Result<GitContext> {
    let repo = Repository::open(repo_path)?;
    let tag_oid = repo
        .revparse_single(&format!("refs/tags/{tag}"))
        .with_context(|| format!("resolving tag {tag}"))?
        .id();
    let tag_commit_oid = repo.find_object(tag_oid, None)?.peel_to_commit()?.id();

    // find previous tag
    let all_tags = list_tags(repo_path)?;
    let tag_idx = all_tags.iter().position(|t| t == tag);
    let prev_tag = tag_idx.and_then(|i| all_tags.get(i + 1)).cloned();

    // collect commits between prev_tag and tag
    let mut revwalk = repo.revwalk()?;
    revwalk.push(tag_commit_oid)?;
    revwalk.set_sorting(Sort::TIME)?;
    if let Some(ref prev) = prev_tag {
        if let Ok(prev_obj) = repo.revparse_single(&format!("refs/tags/{prev}")) {
            if let Ok(prev_commit) = prev_obj.peel_to_commit() {
                revwalk.hide(prev_commit.id())?;
            }
        }
    }

    let mut commits = vec![];
    for oid in revwalk.take(50) {
        let oid = oid?;
        let commit = repo.find_commit(oid)?;
        let msg = commit.summary().unwrap_or("").to_string();
        let author = commit.author().name().unwrap_or("unknown").to_string();
        commits.push(CommitSummary {
            hash: oid.to_string()[..8].to_string(),
            message: msg,
            author,
        });
    }

    // diff stat between prev_tag and tag
    let diff_stat = if let Some(ref prev) = prev_tag {
        compute_diff_stat(&repo, prev, tag)?
    } else {
        "initial release".to_string()
    };

    // look for CHANGELOG.md
    let changelog_excerpt = read_changelog_section(repo_path, tag);

    Ok(GitContext {
        tag: tag.into(),
        prev_tag,
        diff_stat,
        commits,
        changelog_excerpt,
    })
}

fn compute_diff_stat(repo: &Repository, from_tag: &str, to_tag: &str) -> Result<String> {
    let from_obj = repo.revparse_single(&format!("refs/tags/{from_tag}"))?;
    let to_obj = repo.revparse_single(&format!("refs/tags/{to_tag}"))?;
    let from_tree = from_obj.peel_to_commit()?.tree()?;
    let to_tree = to_obj.peel_to_commit()?.tree()?;
    let diff = repo.diff_tree_to_tree(Some(&from_tree), Some(&to_tree), None)?;
    let stats = diff.stats()?;
    Ok(format!(
        "{} files changed, {} insertions(+), {} deletions(-)",
        stats.files_changed(),
        stats.insertions(),
        stats.deletions()
    ))
}

fn read_changelog_section(repo_path: &str, tag: &str) -> Option<String> {
    let changelog = std::path::Path::new(repo_path).join("CHANGELOG.md");
    let content = std::fs::read_to_string(changelog).ok()?;
    // find section starting with ## tag or ## [tag]
    let patterns = [format!("## {tag}"), format!("## [{tag}]")];
    let start = patterns.iter().find_map(|p| content.find(p.as_str()))?;
    let rest = &content[start..];
    // take until next ## heading
    let end = rest[3..].find("\n## ").map(|i| i + 3 + 1).unwrap_or(rest.len());
    let excerpt = &rest[..end.min(1200)]; // cap at 1200 chars
    Some(excerpt.trim().to_string())
}
