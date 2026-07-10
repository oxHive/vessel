use crate::db::Db;
use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevisionRequest {
    pub id: String,
    pub platform: Option<String>,
    pub note: String,
}

pub async fn queue_from_dashboard(
    db: &Db,
    generation_id: &str,
    platform: Option<&str>,
    note: &str,
) -> Result<String> {
    let id = format!("note_{}", Uuid::new_v4().simple());
    let now = Utc::now().timestamp();
    let conn = db.connect()?;
    conn.execute(
        "INSERT INTO revision_notes (id, generation_id, notes, created_at, platform, status, source)
         VALUES (?1, ?2, ?3, ?4, ?5, 'pending', 'dashboard')",
        libsql::params![id.clone(), generation_id, note, now, platform],
    )
    .await?;
    Ok(id)
}

pub async fn drain_pending(db: &Db, generation_id: &str) -> Result<Vec<RevisionRequest>> {
    let conn = db.connect()?;
    // UPDATE ... RETURNING marks delivery and reads in one statement, so a
    // note can never be delivered twice even across concurrent polls.
    let mut rows = conn
        .query(
            "UPDATE revision_notes SET status = 'delivered'
             WHERE generation_id = ?1 AND status = 'pending'
             RETURNING id, platform, notes",
            [generation_id],
        )
        .await?;
    let mut out = vec![];
    while let Some(row) = rows.next().await? {
        out.push(RevisionRequest {
            id: row.get(0)?,
            platform: row.get(1)?,
            note: row.get(2)?,
        });
    }
    Ok(out)
}

pub async fn review_state(db: &Db, generation_id: &str) -> Result<Option<String>> {
    let conn = db.connect()?;
    let mut rows = conn
        .query(
            "SELECT review_state FROM generations WHERE id = ?1",
            [generation_id],
        )
        .await?;
    Ok(match rows.next().await? {
        Some(row) => Some(row.get(0)?),
        None => None,
    })
}

pub async fn set_review_done(db: &Db, generation_id: &str) -> Result<bool> {
    let conn = db.connect()?;
    let n = conn
        .execute(
            "UPDATE generations SET review_state = 'done' WHERE id = ?1",
            [generation_id],
        )
        .await?;
    Ok(n > 0)
}
