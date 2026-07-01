use crate::db::Db;
use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentFeedback {
    pub id: String,
    pub generation_id: String,
    pub platform: String,
    pub signal: String, // "liked" | "disliked" | "reused"
    pub created_at: i64,
}

pub async fn record(db: &Db, generation_id: &str, platform: &str, signal: &str) -> Result<()> {
    let id = format!("fb_{}", Uuid::new_v4().simple());
    let now = Utc::now().timestamp();
    let conn = db.connect()?;
    conn.execute(
        "INSERT INTO content_feedback (id, generation_id, platform, signal, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        libsql::params![id, generation_id, platform, signal, now],
    )
    .await?;
    Ok(())
}

pub async fn list_for_generation(db: &Db, gen_id: &str) -> Result<Vec<ContentFeedback>> {
    let conn = db.connect()?;
    let mut rows = conn
        .query(
            "SELECT id, generation_id, platform, signal, created_at
         FROM content_feedback WHERE generation_id = ?1",
            [gen_id],
        )
        .await?;
    let mut out = vec![];
    while let Some(row) = rows.next().await? {
        out.push(ContentFeedback {
            id: row.get(0)?,
            generation_id: row.get(1)?,
            platform: row.get(2)?,
            signal: row.get(3)?,
            created_at: row.get(4)?,
        });
    }
    Ok(out)
}
