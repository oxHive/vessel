use anyhow::Result;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::Utc;
use crate::db::Db;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub profile_id: String,
    pub repo_path: Option<String>,
    pub github_repo: Option<String>,
    pub provider: String,
    pub created_at: i64,
}

pub async fn create(
    db: &Db,
    profile_id: &str,
    repo_path: Option<&str>,
    github_repo: Option<&str>,
    provider: &str,
) -> Result<Project> {
    let id = format!("project_{}", Uuid::new_v4().simple());
    let now = Utc::now().timestamp();
    let conn = db.connect()?;
    conn.execute(
        "INSERT INTO projects (id, profile_id, repo_path, github_repo, provider, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        libsql::params![id.clone(), profile_id, repo_path, github_repo, provider, now],
    ).await?;
    Ok(Project {
        id,
        profile_id: profile_id.into(),
        repo_path: repo_path.map(Into::into),
        github_repo: github_repo.map(Into::into),
        provider: provider.into(),
        created_at: now,
    })
}

pub async fn find_by_repo(db: &Db, repo_path: &str) -> Result<Option<Project>> {
    let conn = db.connect()?;
    let mut rows = conn.query(
        "SELECT id, profile_id, repo_path, github_repo, provider, created_at
         FROM projects WHERE repo_path = ?1 LIMIT 1",
        [repo_path],
    ).await?;
    match rows.next().await? {
        None => Ok(None),
        Some(row) => Ok(Some(Project {
            id: row.get(0)?,
            profile_id: row.get(1)?,
            repo_path: row.get(2)?,
            github_repo: row.get(3)?,
            provider: row.get(4)?,
            created_at: row.get(5)?,
        })),
    }
}

pub async fn list(db: &Db) -> Result<Vec<Project>> {
    let conn = db.connect()?;
    let mut rows = conn.query(
        "SELECT id, profile_id, repo_path, github_repo, provider, created_at
         FROM projects ORDER BY created_at DESC",
        (),
    ).await?;
    let mut out = vec![];
    while let Some(row) = rows.next().await? {
        out.push(Project {
            id: row.get(0)?,
            profile_id: row.get(1)?,
            repo_path: row.get(2)?,
            github_repo: row.get(3)?,
            provider: row.get(4)?,
            created_at: row.get(5)?,
        });
    }
    Ok(out)
}
