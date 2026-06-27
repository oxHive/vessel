use anyhow::Result;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::Utc;
use crate::db::Db;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Generation {
    pub id: String,
    pub project_id: String,
    pub tag: String,
    pub category: String,
    pub context_notes: Option<String>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationOutput {
    pub id: String,
    pub generation_id: String,
    pub platform: String,
    pub content: String,
    pub revision_number: i64,
    pub created_at: i64,
}

pub async fn create(
    db: &Db,
    project_id: &str,
    tag: &str,
    category: &str,
    context_notes: Option<&str>,
) -> Result<Generation> {
    let id = format!("gen_{}", Uuid::new_v4().simple());
    let now = Utc::now().timestamp();
    let conn = db.connect()?;
    conn.execute(
        "INSERT INTO generations (id, project_id, tag, category, context_notes, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        libsql::params![id.clone(), project_id, tag, category, context_notes, now],
    ).await?;
    Ok(Generation {
        id,
        project_id: project_id.into(),
        tag: tag.into(),
        category: category.into(),
        context_notes: context_notes.map(Into::into),
        created_at: now,
    })
}

pub async fn save_output(
    db: &Db,
    generation_id: &str,
    platform: &str,
    content: &str,
) -> Result<GenerationOutput> {
    let conn = db.connect()?;
    // Determine next revision number for this generation+platform combination
    let mut rows = conn.query(
        "SELECT MAX(revision_number) FROM generation_outputs WHERE generation_id=?1 AND platform=?2",
        libsql::params![generation_id, platform],
    ).await?;
    // MAX() returns NULL when no rows match; get::<i64> on NULL returns Err,
    // so unwrap_or(-1) gives -1, and -1 + 1 = 0 for the first revision.
    let rev: i64 = rows
        .next()
        .await?
        .map(|r| r.get::<i64>(0).unwrap_or(-1))
        .unwrap_or(-1)
        + 1;
    let id = format!("output_{}", Uuid::new_v4().simple());
    let now = Utc::now().timestamp();
    conn.execute(
        "INSERT INTO generation_outputs (id, generation_id, platform, content, revision_number, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        libsql::params![id.clone(), generation_id, platform, content, rev, now],
    ).await?;
    Ok(GenerationOutput {
        id,
        generation_id: generation_id.into(),
        platform: platform.into(),
        content: content.into(),
        revision_number: rev,
        created_at: now,
    })
}

pub async fn list_recent(db: &Db, project_id: &str, limit: u32) -> Result<Vec<Generation>> {
    let conn = db.connect()?;
    let mut rows = conn.query(
        "SELECT id, project_id, tag, category, context_notes, created_at
         FROM generations WHERE project_id = ?1 ORDER BY created_at DESC LIMIT ?2",
        libsql::params![project_id, limit],
    ).await?;
    let mut out = vec![];
    while let Some(row) = rows.next().await? {
        out.push(Generation {
            id: row.get(0)?,
            project_id: row.get(1)?,
            tag: row.get(2)?,
            category: row.get(3)?,
            context_notes: row.get(4)?,
            created_at: row.get(5)?,
        });
    }
    Ok(out)
}

pub async fn get_with_outputs(
    db: &Db,
    gen_id: &str,
) -> Result<Option<(Generation, Vec<GenerationOutput>)>> {
    let conn = db.connect()?;
    let mut rows = conn.query(
        "SELECT id, project_id, tag, category, context_notes, created_at
         FROM generations WHERE id = ?1",
        [gen_id],
    ).await?;
    let generation = match rows.next().await? {
        None => return Ok(None),
        Some(row) => Generation {
            id: row.get(0)?,
            project_id: row.get(1)?,
            tag: row.get(2)?,
            category: row.get(3)?,
            context_notes: row.get(4)?,
            created_at: row.get(5)?,
        },
    };
    let mut rows = conn.query(
        "SELECT id, generation_id, platform, content, revision_number, created_at
         FROM generation_outputs WHERE generation_id = ?1
         ORDER BY platform, revision_number DESC",
        [gen_id],
    ).await?;
    let mut outputs = vec![];
    while let Some(row) = rows.next().await? {
        outputs.push(GenerationOutput {
            id: row.get(0)?,
            generation_id: row.get(1)?,
            platform: row.get(2)?,
            content: row.get(3)?,
            revision_number: row.get(4)?,
            created_at: row.get(5)?,
        });
    }
    Ok(Some((generation, outputs)))
}
