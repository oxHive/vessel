use crate::db::Db;
use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub formality: String,
    pub humor: String,
    pub technical_depth: String,
    pub self_promotion: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone)]
pub struct VoiceSettings {
    pub formality: String,
    pub humor: String,
    pub technical_depth: String,
    pub self_promotion: String,
}

impl Default for VoiceSettings {
    fn default() -> Self {
        Self {
            formality: "balanced".into(),
            humor: "subtle".into(),
            technical_depth: "medium".into(),
            self_promotion: "balanced".into(),
        }
    }
}

pub async fn create(db: &Db, name: &str, voice: VoiceSettings) -> Result<Profile> {
    let id = format!("profile_{}", Uuid::new_v4().simple());
    let now = Utc::now().timestamp();
    let conn = db.connect()?;
    conn.execute(
        "INSERT INTO profiles (id, name, formality, humor, technical_depth, self_promotion, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        libsql::params![id.clone(), name, voice.formality.clone(), voice.humor.clone(),
            voice.technical_depth.clone(), voice.self_promotion.clone(), now, now],
    ).await?;
    Ok(Profile {
        id,
        name: name.into(),
        formality: voice.formality,
        humor: voice.humor,
        technical_depth: voice.technical_depth,
        self_promotion: voice.self_promotion,
        created_at: now,
        updated_at: now,
    })
}

pub async fn get(db: &Db, id: &str) -> Result<Option<Profile>> {
    let conn = db.connect()?;
    let mut rows = conn.query(
        "SELECT id, name, formality, humor, technical_depth, self_promotion, created_at, updated_at
         FROM profiles WHERE id = ?1",
        [id],
    ).await?;
    match rows.next().await? {
        None => Ok(None),
        Some(row) => Ok(Some(Profile {
            id: row.get(0)?,
            name: row.get(1)?,
            formality: row.get(2)?,
            humor: row.get(3)?,
            technical_depth: row.get(4)?,
            self_promotion: row.get(5)?,
            created_at: row.get(6)?,
            updated_at: row.get(7)?,
        })),
    }
}

pub async fn list(db: &Db) -> Result<Vec<Profile>> {
    let conn = db.connect()?;
    let mut rows = conn.query(
        "SELECT id, name, formality, humor, technical_depth, self_promotion, created_at, updated_at
         FROM profiles ORDER BY created_at DESC",
        (),
    ).await?;
    let mut profiles = vec![];
    while let Some(row) = rows.next().await? {
        profiles.push(Profile {
            id: row.get(0)?,
            name: row.get(1)?,
            formality: row.get(2)?,
            humor: row.get(3)?,
            technical_depth: row.get(4)?,
            self_promotion: row.get(5)?,
            created_at: row.get(6)?,
            updated_at: row.get(7)?,
        });
    }
    Ok(profiles)
}
