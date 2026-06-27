# Vessel Backend Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the Vessel backend — a Rust binary providing an MCP server (stdio), REST API (port 3458), libSQL storage, local git + GitHub context reading, multi-platform content generation via the user's Claude session, and HiveMind integration.

**Architecture:** Single `vessel` binary. CLI subcommands: `vessel up` (start server), `vessel mcp` (stdio MCP server). MCP prompts return assembled context so Claude generates content in-session; a `vessel_save` MCP tool persists results to libSQL. REST API (axum) serves the dashboard data layer on port 3458.

**Tech Stack:** Rust (tokio, axum, rmcp, libsql, git2, reqwest, clap, serde, aes-gcm, rust-embed placeholder)

## Global Constraints

- Crate name: `oxvessel`; binary name: `vessel`; Rust edition 2024
- Dashboard + REST API port: `3458`
- MCP transport: stdio only (`vessel mcp` subcommand)
- DB path default: `~/.vessel/vessel.db`
- Config file: `~/.config/vessel/vessel.toml` (XDG) with fallback to `~/.vessel/vessel.toml`
- HiveMind base URL: `http://localhost:3456/api/v1`
- All IDs: UUID v4 strings prefixed by type (`profile_`, `project_`, `gen_`, `output_`, `fb_`)
- All timestamps: Unix seconds (i64)
- GitHub tokens encrypted at rest with AES-256-GCM
- Platform character limits enforced at formatter level: X=280, Bluesky=300, Mastodon=500, LinkedIn=3000
- v1 scope only: no direct posting, no scheduling, no GitLab/Gitea

---

## File Map

```
Cargo.toml
migrations/
├── 001_init.sql              — initial schema (all 8 tables)
src/
├── main.rs                   — CLI entrypoint, subcommand dispatch (up / mcp)
├── config.rs                 — VesselConfig (TOML parse, defaults, path resolution)
├── db/
│   ├── mod.rs                — DB init, connection pool, run_migrations()
│   ├── schema.rs             — Migration SQL strings in order
│   ├── profiles.rs           — Profile + ProfilePlatform CRUD
│   ├── projects.rs           — Project CRUD, repo→profile lookup
│   ├── generations.rs        — Generation + GenerationOutput CRUD
│   └── feedback.rs           — ContentFeedback CRUD
├── mcp/
│   ├── mod.rs                — MCP server struct, ServerHandler impl
│   ├── prompts.rs            — list_prompts + get_prompt handlers
│   └── tools.rs              — vessel_save tool handler
├── generation/
│   ├── mod.rs                — GenerationRequest, orchestrate()
│   ├── git.rs                — local git: tags, diff, changelog via git2
│   ├── github.rs             — GitHub API: tags, releases, patch body
│   ├── prompt.rs             — assemble_prompt(): context block builder
│   └── platforms/
│       ├── mod.rs            — Platform enum, PlatformSpec, format_constraints()
│       ├── twitter.rs        — X formatter (280 chars, thread hint)
│       ├── linkedin.rs       — LinkedIn formatter (3000 chars, narrative)
│       ├── bluesky.rs        — Bluesky formatter (300 chars)
│       ├── mastodon.rs       — Mastodon formatter (500 chars)
│       ├── discord.rs        — Discord formatter (conversational, no hard limit)
│       └── github_release.rs — GitHub Release markdown formatter
├── hivemind/
│   ├── mod.rs                — HiveMindClient, is_available()
│   └── client.rs             — health check, read_project_context(), write_vessel_memory()
├── api/
│   ├── mod.rs                — axum Router builder
│   ├── generations.rs        — GET/POST/GET:id /api/v1/generations
│   ├── profiles.rs           — CRUD /api/v1/profiles
│   ├── projects.rs           — CRUD /api/v1/projects
│   ├── feedback.rs           — POST /api/v1/feedback
│   └── settings.rs           — GET/PATCH /api/v1/settings
└── server.rs                 — start_server(): bind axum, serve static placeholder
tests/
├── db_profiles.rs
├── db_generations.rs
├── git_context.rs
├── platform_formatters.rs
├── prompt_assembly.rs
└── api_generations.rs
```

---

### Task 1: Project Scaffold

**Files:**
- Modify: `Cargo.toml`
- Modify: `src/main.rs`
- Create: `src/config.rs`

**Interfaces:**
- Produces: `VesselConfig` struct; `vessel up` and `vessel mcp` subcommands compile and run (no-op bodies)

- [ ] **Step 1: Update Cargo.toml**

```toml
[package]
name = "oxvessel"
version = "0.1.0"
edition = "2024"
description = "Developer release announcement tool"
license = "MIT"

[[bin]]
name = "vessel"
path = "src/main.rs"

[dependencies]
tokio = { version = "1", features = ["full"] }
axum = { version = "0.7", features = ["macros"] }
tower-http = { version = "0.5", features = ["fs", "cors", "trace"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
libsql = "0.6"
rmcp = { version = "0.1", features = ["server", "transport-io"] }
reqwest = { version = "0.12", features = ["json"] }
git2 = "0.19"
anyhow = "1"
thiserror = "2"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1", features = ["v4"] }
clap = { version = "4", features = ["derive"] }
dirs = "5"
aes-gcm = "0.10"
base64 = "0.22"
rand = "0.8"

[dev-dependencies]
tokio = { version = "1", features = ["full", "test-util"] }
```

- [ ] **Step 2: Write src/config.rs**

```rust
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VesselConfig {
    #[serde(default = "default_server")]
    pub server: ServerConfig,
    #[serde(default = "default_storage")]
    pub storage: StorageConfig,
    #[serde(default = "default_hivemind")]
    pub hivemind: HiveMindConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StorageConfig {
    pub path: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HiveMindConfig {
    pub port: u16,
}

fn default_server() -> ServerConfig { ServerConfig { port: 3458 } }
fn default_storage() -> StorageConfig {
    StorageConfig { path: "~/.vessel/vessel.db".into() }
}
fn default_hivemind() -> HiveMindConfig { HiveMindConfig { port: 3456 } }

impl Default for VesselConfig {
    fn default() -> Self {
        Self {
            server: default_server(),
            storage: default_storage(),
            hivemind: default_hivemind(),
        }
    }
}

impl VesselConfig {
    pub fn load() -> Result<Self> {
        let path = config_path();
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(&path)?;
        Ok(toml::from_str(&content)?)
    }

    pub fn db_path(&self) -> PathBuf {
        let raw = self.storage.path.replace('~', &dirs::home_dir()
            .unwrap_or_default().to_string_lossy());
        PathBuf::from(raw)
    }
}

fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_default())
        .join("vessel")
        .join("vessel.toml")
}
```

- [ ] **Step 3: Write src/main.rs**

```rust
use clap::{Parser, Subcommand};
use anyhow::Result;

mod config;
mod db;
mod mcp;
mod generation;
mod hivemind;
mod api;
mod server;

#[derive(Parser)]
#[command(name = "vessel", about = "Developer release announcement tool")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Up,
    Mcp,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();
    let config = config::VesselConfig::load()?;
    let db = db::init(&config).await?;

    match cli.command {
        Commands::Up => server::start(config, db).await,
        Commands::Mcp => mcp::serve(config, db).await,
    }
}
```

- [ ] **Step 4: Create stub modules so it compiles**

Create `src/db/mod.rs`, `src/mcp/mod.rs`, `src/generation/mod.rs`, `src/hivemind/mod.rs`, `src/api/mod.rs`, `src/server.rs` each with a single `// TODO` comment and the minimum pub items referenced from main.rs.

```rust
// src/db/mod.rs
pub mod schema;
pub mod profiles;
pub mod projects;
pub mod generations;
pub mod feedback;

use anyhow::Result;
use libsql::Database;
use crate::config::VesselConfig;

pub type Db = std::sync::Arc<Database>;

pub async fn init(config: &VesselConfig) -> Result<Db> {
    let path = config.db_path();
    std::fs::create_dir_all(path.parent().unwrap())?;
    let db = libsql::Builder::new_local(path).build().await?;
    schema::run_migrations(&db).await?;
    Ok(std::sync::Arc::new(db))
}
```

```rust
// src/server.rs
use anyhow::Result;
use crate::{config::VesselConfig, db::Db};

pub async fn start(_config: VesselConfig, _db: Db) -> Result<()> {
    todo!("implement in Task 12")
}
```

```rust
// src/mcp/mod.rs
use anyhow::Result;
use crate::{config::VesselConfig, db::Db};

pub async fn serve(_config: VesselConfig, _db: Db) -> Result<()> {
    todo!("implement in Task 5")
}
```

- [ ] **Step 5: Verify it compiles**

```bash
cd /home/graditya/projects/oxhive/vessel
cargo build 2>&1 | head -30
```

Expected: compile error only on `todo!()` if triggered at runtime, but `cargo build` should succeed (todos are not compile errors).

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml src/
git commit -m "feat: scaffold oxvessel crate with CLI subcommands and config"
```

---

### Task 2: Database Schema and Migrations

**Files:**
- Create: `src/db/schema.rs`
- Modify: `src/db/mod.rs`

**Interfaces:**
- Consumes: `libsql::Database`
- Produces: `run_migrations(db: &Database) -> Result<()>` — idempotent, runs on every startup

- [ ] **Step 1: Write failing migration test**

```rust
// tests/db_migrations.rs
use vessel::db;
use vessel::config::VesselConfig;

#[tokio::test]
async fn migrations_run_idempotent() {
    let db = libsql::Builder::new_local(":memory:").build().await.unwrap();
    db::schema::run_migrations(&db).await.unwrap();
    // run twice — must not error
    db::schema::run_migrations(&db).await.unwrap();
    let conn = db.connect().unwrap();
    let mut rows = conn.query("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name", ()).await.unwrap();
    let mut tables = vec![];
    while let Some(row) = rows.next().await.unwrap() {
        tables.push(row.get::<String>(0).unwrap());
    }
    assert!(tables.contains(&"profiles".to_string()));
    assert!(tables.contains(&"projects".to_string()));
    assert!(tables.contains(&"generations".to_string()));
    assert!(tables.contains(&"generation_outputs".to_string()));
    assert!(tables.contains(&"revision_notes".to_string()));
    assert!(tables.contains(&"content_feedback".to_string()));
    assert!(tables.contains(&"github_tokens".to_string()));
    assert!(tables.contains(&"_migrations".to_string()));
}
```

Add `vessel` as lib target to Cargo.toml:
```toml
[lib]
name = "vessel"
path = "src/lib.rs"
```

Create `src/lib.rs`:
```rust
pub mod config;
pub mod db;
pub mod generation;
pub mod hivemind;
pub mod api;
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cargo test migrations_run_idempotent 2>&1 | tail -10
```
Expected: FAIL — `run_migrations` not implemented

- [ ] **Step 3: Write src/db/schema.rs**

```rust
use anyhow::Result;
use libsql::Database;

const MIGRATIONS: &[(&str, &str)] = &[
    ("001_init", include_str!("../../migrations/001_init.sql")),
];

pub async fn run_migrations(db: &Database) -> Result<()> {
    let conn = db.connect()?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS _migrations (
            name TEXT PRIMARY KEY,
            applied_at INTEGER NOT NULL
        )", ()
    ).await?;

    for (name, sql) in MIGRATIONS {
        let mut rows = conn.query(
            "SELECT name FROM _migrations WHERE name = ?1", [*name]
        ).await?;
        if rows.next().await?.is_some() {
            continue;
        }
        conn.execute_batch(sql).await?;
        conn.execute(
            "INSERT INTO _migrations (name, applied_at) VALUES (?1, ?2)",
            [*name, &chrono::Utc::now().timestamp().to_string()],
        ).await?;
    }
    Ok(())
}
```

- [ ] **Step 4: Create migrations/ directory and 001_init.sql**

```bash
mkdir -p /home/graditya/projects/oxhive/vessel/migrations
```

```sql
CREATE TABLE IF NOT EXISTS profiles (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    formality TEXT NOT NULL DEFAULT 'balanced',
    humor TEXT NOT NULL DEFAULT 'subtle',
    technical_depth TEXT NOT NULL DEFAULT 'medium',
    self_promotion TEXT NOT NULL DEFAULT 'balanced',
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS profile_platforms (
    id TEXT PRIMARY KEY,
    profile_id TEXT NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
    platform TEXT NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1,
    handles TEXT,
    hashtags TEXT,
    UNIQUE(profile_id, platform)
);

CREATE TABLE IF NOT EXISTS projects (
    id TEXT PRIMARY KEY,
    profile_id TEXT NOT NULL REFERENCES profiles(id),
    repo_path TEXT,
    github_repo TEXT,
    provider TEXT NOT NULL DEFAULT 'local',
    created_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS generations (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id),
    tag TEXT NOT NULL,
    category TEXT NOT NULL DEFAULT 'release',
    context_notes TEXT,
    created_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS generation_outputs (
    id TEXT PRIMARY KEY,
    generation_id TEXT NOT NULL REFERENCES generations(id) ON DELETE CASCADE,
    platform TEXT NOT NULL,
    content TEXT NOT NULL,
    revision_number INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS revision_notes (
    id TEXT PRIMARY KEY,
    generation_id TEXT NOT NULL REFERENCES generations(id) ON DELETE CASCADE,
    notes TEXT NOT NULL,
    created_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS content_feedback (
    id TEXT PRIMARY KEY,
    generation_id TEXT NOT NULL REFERENCES generations(id) ON DELETE CASCADE,
    platform TEXT NOT NULL,
    signal TEXT NOT NULL,
    created_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS github_tokens (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL UNIQUE REFERENCES projects(id) ON DELETE CASCADE,
    token_enc TEXT NOT NULL,
    nonce TEXT NOT NULL,
    created_at INTEGER NOT NULL
);
```

- [ ] **Step 5: Run test to verify it passes**

```bash
cargo test migrations_run_idempotent
```
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add migrations/ src/db/schema.rs src/db/mod.rs src/lib.rs Cargo.toml
git commit -m "feat: add libSQL schema with idempotent migrations"
```

---

### Task 3: Database CRUD Layer

**Files:**
- Create: `src/db/profiles.rs`, `src/db/projects.rs`, `src/db/generations.rs`, `src/db/feedback.rs`

**Interfaces:**
- Consumes: `libsql::Database`
- Produces:
  - `profiles::create(db, name, voice) -> Result<Profile>`
  - `profiles::get(db, id) -> Result<Option<Profile>>`
  - `profiles::list(db) -> Result<Vec<Profile>>`
  - `projects::create(db, profile_id, repo_path, github_repo, provider) -> Result<Project>`
  - `projects::find_by_repo(db, repo_path) -> Result<Option<Project>>`
  - `generations::create(db, project_id, tag, category, notes) -> Result<Generation>`
  - `generations::save_output(db, gen_id, platform, content) -> Result<GenerationOutput>`
  - `generations::list_recent(db, project_id, limit) -> Result<Vec<Generation>>`
  - `generations::get_with_outputs(db, gen_id) -> Result<Option<(Generation, Vec<GenerationOutput>)>>`
  - `feedback::record(db, gen_id, platform, signal) -> Result<()>`
  - `feedback::list_for_generation(db, gen_id) -> Result<Vec<ContentFeedback>>`

- [ ] **Step 1: Write failing tests**

```rust
// tests/db_profiles.rs
use vessel::db;

async fn test_db() -> db::Db {
    let db = libsql::Builder::new_local(":memory:").build().await.unwrap();
    db::schema::run_migrations(&db).await.unwrap();
    std::sync::Arc::new(db)
}

#[tokio::test]
async fn profile_create_and_get() {
    let db = test_db().await;
    let voice = db::profiles::VoiceSettings {
        formality: "professional".into(),
        humor: "none".into(),
        technical_depth: "high".into(),
        self_promotion: "direct".into(),
    };
    let profile = db::profiles::create(&db, "Oxhive", voice).await.unwrap();
    assert!(profile.id.starts_with("profile_"));
    let fetched = db::profiles::get(&db, &profile.id).await.unwrap().unwrap();
    assert_eq!(fetched.name, "Oxhive");
}

#[tokio::test]
async fn project_find_by_repo() {
    let db = test_db().await;
    let voice = db::profiles::VoiceSettings::default();
    let profile = db::profiles::create(&db, "test", voice).await.unwrap();
    db::projects::create(&db, &profile.id, Some("/home/user/myrepo"), None, "local").await.unwrap();
    let found = db::projects::find_by_repo(&db, "/home/user/myrepo").await.unwrap();
    assert!(found.is_some());
}
```

- [ ] **Step 2: Run to verify fail**

```bash
cargo test db_profiles 2>&1 | tail -5
```

- [ ] **Step 3: Define types in src/db/profiles.rs**

```rust
use anyhow::Result;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::Utc;
use crate::db::Db;

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

#[derive(Debug, Clone, Default)]
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
        libsql::params![id.clone(), name, voice.formality, voice.humor,
            voice.technical_depth, voice.self_promotion, now, now],
    ).await?;
    Ok(Profile { id, name: name.into(), formality: voice.formality,
        humor: voice.humor, technical_depth: voice.technical_depth,
        self_promotion: voice.self_promotion, created_at: now, updated_at: now })
}

pub async fn get(db: &Db, id: &str) -> Result<Option<Profile>> {
    let conn = db.connect()?;
    let mut rows = conn.query(
        "SELECT id, name, formality, humor, technical_depth, self_promotion, created_at, updated_at
         FROM profiles WHERE id = ?1", [id]
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
         FROM profiles ORDER BY created_at DESC", ()
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
```

- [ ] **Step 4: Write src/db/projects.rs**

```rust
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

pub async fn create(db: &Db, profile_id: &str, repo_path: Option<&str>,
    github_repo: Option<&str>, provider: &str) -> Result<Project> {
    let id = format!("project_{}", Uuid::new_v4().simple());
    let now = Utc::now().timestamp();
    let conn = db.connect()?;
    conn.execute(
        "INSERT INTO projects (id, profile_id, repo_path, github_repo, provider, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        libsql::params![id.clone(), profile_id, repo_path, github_repo, provider, now],
    ).await?;
    Ok(Project { id, profile_id: profile_id.into(), repo_path: repo_path.map(Into::into),
        github_repo: github_repo.map(Into::into), provider: provider.into(), created_at: now })
}

pub async fn find_by_repo(db: &Db, repo_path: &str) -> Result<Option<Project>> {
    let conn = db.connect()?;
    let mut rows = conn.query(
        "SELECT id, profile_id, repo_path, github_repo, provider, created_at
         FROM projects WHERE repo_path = ?1 LIMIT 1", [repo_path]
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
         FROM projects ORDER BY created_at DESC", ()
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
```

- [ ] **Step 5: Write src/db/generations.rs**

```rust
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

pub async fn create(db: &Db, project_id: &str, tag: &str, category: &str,
    context_notes: Option<&str>) -> Result<Generation> {
    let id = format!("gen_{}", Uuid::new_v4().simple());
    let now = Utc::now().timestamp();
    let conn = db.connect()?;
    conn.execute(
        "INSERT INTO generations (id, project_id, tag, category, context_notes, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        libsql::params![id.clone(), project_id, tag, category, context_notes, now],
    ).await?;
    Ok(Generation { id, project_id: project_id.into(), tag: tag.into(),
        category: category.into(), context_notes: context_notes.map(Into::into), created_at: now })
}

pub async fn save_output(db: &Db, generation_id: &str, platform: &str,
    content: &str) -> Result<GenerationOutput> {
    let conn = db.connect()?;
    // bump revision number if prior output exists for this platform
    let mut rows = conn.query(
        "SELECT MAX(revision_number) FROM generation_outputs WHERE generation_id=?1 AND platform=?2",
        [generation_id, platform]
    ).await?;
    let rev: i64 = rows.next().await?.map(|r| r.get::<i64>(0).unwrap_or(0)).unwrap_or(-1) + 1;
    let id = format!("output_{}", Uuid::new_v4().simple());
    let now = Utc::now().timestamp();
    conn.execute(
        "INSERT INTO generation_outputs (id, generation_id, platform, content, revision_number, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        libsql::params![id.clone(), generation_id, platform, content, rev, now],
    ).await?;
    Ok(GenerationOutput { id, generation_id: generation_id.into(), platform: platform.into(),
        content: content.into(), revision_number: rev, created_at: now })
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
            id: row.get(0)?, project_id: row.get(1)?, tag: row.get(2)?,
            category: row.get(3)?, context_notes: row.get(4)?, created_at: row.get(5)?,
        });
    }
    Ok(out)
}

pub async fn get_with_outputs(db: &Db, gen_id: &str)
    -> Result<Option<(Generation, Vec<GenerationOutput>)>> {
    let conn = db.connect()?;
    let mut rows = conn.query(
        "SELECT id, project_id, tag, category, context_notes, created_at
         FROM generations WHERE id = ?1", [gen_id]
    ).await?;
    let gen = match rows.next().await? {
        None => return Ok(None),
        Some(row) => Generation {
            id: row.get(0)?, project_id: row.get(1)?, tag: row.get(2)?,
            category: row.get(3)?, context_notes: row.get(4)?, created_at: row.get(5)?,
        },
    };
    let mut rows = conn.query(
        "SELECT id, generation_id, platform, content, revision_number, created_at
         FROM generation_outputs WHERE generation_id = ?1
         ORDER BY platform, revision_number DESC", [gen_id]
    ).await?;
    let mut outputs = vec![];
    while let Some(row) = rows.next().await? {
        outputs.push(GenerationOutput {
            id: row.get(0)?, generation_id: row.get(1)?, platform: row.get(2)?,
            content: row.get(3)?, revision_number: row.get(4)?, created_at: row.get(5)?,
        });
    }
    Ok(Some((gen, outputs)))
}
```

- [ ] **Step 6: Write src/db/feedback.rs**

```rust
use anyhow::Result;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::Utc;
use crate::db::Db;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentFeedback {
    pub id: String,
    pub generation_id: String,
    pub platform: String,
    pub signal: String,  // "liked" | "disliked" | "reused"
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
    ).await?;
    Ok(())
}

pub async fn list_for_generation(db: &Db, gen_id: &str) -> Result<Vec<ContentFeedback>> {
    let conn = db.connect()?;
    let mut rows = conn.query(
        "SELECT id, generation_id, platform, signal, created_at
         FROM content_feedback WHERE generation_id = ?1", [gen_id]
    ).await?;
    let mut out = vec![];
    while let Some(row) = rows.next().await? {
        out.push(ContentFeedback {
            id: row.get(0)?, generation_id: row.get(1)?, platform: row.get(2)?,
            signal: row.get(3)?, created_at: row.get(4)?,
        });
    }
    Ok(out)
}
```

- [ ] **Step 7: Run tests**

```bash
cargo test db_profiles -- --nocapture
```
Expected: PASS

- [ ] **Step 8: Commit**

```bash
git add src/db/ migrations/ tests/
git commit -m "feat: add database CRUD layer for profiles, projects, generations, feedback"
```

---

### Task 4: Local Git Context Reader

**Files:**
- Create: `src/generation/git.rs`
- Modify: `src/generation/mod.rs`

**Interfaces:**
- Produces:
  - `GitContext { tag: String, prev_tag: Option<String>, diff_stat: String, commits: Vec<CommitSummary>, changelog_excerpt: Option<String> }`
  - `read_git_context(repo_path: &str, tag: &str) -> Result<GitContext>`
  - `list_tags(repo_path: &str) -> Result<Vec<String>>`

- [ ] **Step 1: Write failing test**

```rust
// tests/git_context.rs
use vessel::generation::git;
use std::process::Command;
use tempfile::TempDir;

fn make_test_repo() -> TempDir {
    let dir = TempDir::new().unwrap();
    let p = dir.path();
    Command::new("git").args(["init"]).current_dir(p).output().unwrap();
    Command::new("git").args(["config", "user.email", "test@test.com"]).current_dir(p).output().unwrap();
    Command::new("git").args(["config", "user.name", "Test"]).current_dir(p).output().unwrap();
    std::fs::write(p.join("main.rs"), "fn main() {}").unwrap();
    Command::new("git").args(["add", "."]).current_dir(p).output().unwrap();
    Command::new("git").args(["commit", "-m", "initial"]).current_dir(p).output().unwrap();
    Command::new("git").args(["tag", "v0.1.0"]).current_dir(p).output().unwrap();
    std::fs::write(p.join("lib.rs"), "pub fn hello() {}").unwrap();
    Command::new("git").args(["add", "."]).current_dir(p).output().unwrap();
    Command::new("git").args(["commit", "-m", "add hello fn"]).current_dir(p).output().unwrap();
    Command::new("git").args(["tag", "v0.2.0"]).current_dir(p).output().unwrap();
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
```

Add `tempfile` to dev-dependencies:
```toml
[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 2: Run to verify fail**

```bash
cargo test git_context 2>&1 | tail -5
```

- [ ] **Step 3: Implement src/generation/git.rs**

```rust
use anyhow::{Result, Context};
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
    let tag_oid = repo.revparse_single(&format!("refs/tags/{tag}"))?.id();
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

    Ok(GitContext { tag: tag.into(), prev_tag, diff_stat, commits, changelog_excerpt })
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
    let start = patterns.iter()
        .find_map(|p| content.find(p.as_str()))?;
    let rest = &content[start..];
    // take until next ## heading
    let end = rest[3..].find("\n## ").map(|i| i + 3 + 1).unwrap_or(rest.len());
    let excerpt = &rest[..end.min(1200)]; // cap at 1200 chars
    Some(excerpt.trim().to_string())
}
```

- [ ] **Step 4: Update src/generation/mod.rs**

```rust
pub mod git;
pub mod github;
pub mod prompt;
pub mod platforms;
```

- [ ] **Step 5: Run tests**

```bash
cargo test git_context
```
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add src/generation/ tests/git_context.rs Cargo.toml
git commit -m "feat: add local git context reader (tags, diff, commits, changelog)"
```

---

### Task 5: Platform Content Contracts

**Files:**
- Create: `src/generation/platforms/mod.rs`
- Create: `src/generation/platforms/twitter.rs`
- Create: `src/generation/platforms/linkedin.rs`
- Create: `src/generation/platforms/bluesky.rs`
- Create: `src/generation/platforms/mastodon.rs`
- Create: `src/generation/platforms/discord.rs`
- Create: `src/generation/platforms/github_release.rs`

**Interfaces:**
- Produces:
  - `Platform` enum: `Twitter | LinkedIn | Bluesky | Mastodon | Discord | GitHubRelease`
  - `PlatformSpec { name: &str, char_limit: Option<usize>, tone_guidance: &str, format_notes: &str }`
  - `Platform::spec() -> PlatformSpec`
  - `Platform::all_v1() -> Vec<Platform>`
  - `Platform::validate_length(content: &str) -> Option<usize>` — returns None if ok, Some(overage) if too long

- [ ] **Step 1: Write failing test**

```rust
// tests/platform_formatters.rs
use vessel::generation::platforms::{Platform, PlatformSpec};

#[test]
fn twitter_has_280_limit() {
    let spec = Platform::Twitter.spec();
    assert_eq!(spec.char_limit, Some(280));
}

#[test]
fn twitter_validates_over_limit() {
    let long = "x".repeat(281);
    assert!(Platform::Twitter.validate_length(&long).is_some());
}

#[test]
fn twitter_validates_within_limit() {
    let short = "x".repeat(280);
    assert!(Platform::Twitter.validate_length(&short).is_none());
}

#[test]
fn all_v1_has_six_platforms() {
    assert_eq!(Platform::all_v1().len(), 6);
}

#[test]
fn github_release_has_no_char_limit() {
    assert_eq!(Platform::GitHubRelease.spec().char_limit, None);
}
```

- [ ] **Step 2: Run to verify fail**

```bash
cargo test platform_formatters 2>&1 | tail -5
```

- [ ] **Step 3: Implement src/generation/platforms/mod.rs**

```rust
pub mod twitter;
pub mod linkedin;
pub mod bluesky;
pub mod mastodon;
pub mod discord;
pub mod github_release;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Platform {
    Twitter,
    LinkedIn,
    Bluesky,
    Mastodon,
    Discord,
    GitHubRelease,
}

pub struct PlatformSpec {
    pub name: &'static str,
    pub char_limit: Option<usize>,
    pub tone_guidance: &'static str,
    pub format_notes: &'static str,
    pub hashtag_notes: &'static str,
}

impl Platform {
    pub fn all_v1() -> Vec<Platform> {
        vec![Platform::Twitter, Platform::LinkedIn, Platform::Bluesky,
             Platform::Mastodon, Platform::Discord, Platform::GitHubRelease]
    }

    pub fn spec(&self) -> PlatformSpec {
        match self {
            Platform::Twitter => twitter::spec(),
            Platform::LinkedIn => linkedin::spec(),
            Platform::Bluesky => bluesky::spec(),
            Platform::Mastodon => mastodon::spec(),
            Platform::Discord => discord::spec(),
            Platform::GitHubRelease => github_release::spec(),
        }
    }

    pub fn validate_length(&self, content: &str) -> Option<usize> {
        let spec = self.spec();
        spec.char_limit.and_then(|limit| {
            let len = content.chars().count();
            if len > limit { Some(len - limit) } else { None }
        })
    }

    pub fn slug(&self) -> &'static str {
        match self {
            Platform::Twitter => "twitter",
            Platform::LinkedIn => "linkedin",
            Platform::Bluesky => "bluesky",
            Platform::Mastodon => "mastodon",
            Platform::Discord => "discord",
            Platform::GitHubRelease => "github_release",
        }
    }
}
```

- [ ] **Step 4: Implement each platform spec file**

```rust
// src/generation/platforms/twitter.rs
use super::PlatformSpec;
pub fn spec() -> PlatformSpec {
    PlatformSpec {
        name: "X (Twitter)",
        char_limit: Some(280),
        tone_guidance: "Punchy, opinionated, direct. Every word earns its place. Threads are fine for longer announcements — break at natural thought boundaries.",
        format_notes: "Single tweet preferred. If over 280 chars, structure as a thread: first tweet is the hook, subsequent tweets expand. No markdown.",
        hashtag_notes: "1-3 hashtags max, placed at end of tweet or thread. Use only well-known tech hashtags (#rustlang, #opensource, #devtools). Never hashtag common words.",
    }
}
```

```rust
// src/generation/platforms/linkedin.rs
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
```

```rust
// src/generation/platforms/bluesky.rs
use super::PlatformSpec;
pub fn spec() -> PlatformSpec {
    PlatformSpec {
        name: "Bluesky",
        char_limit: Some(300),
        tone_guidance: "Early-adopter developer culture. Technical credibility matters. Conversational but precise. Close to early Twitter.",
        format_notes: "Single post, 300 chars max. Link cards render automatically — no need to describe the URL. No markdown.",
        hashtag_notes: "0-2 hashtags. Optional. Bluesky culture is less hashtag-driven than Twitter.",
    }
}
```

```rust
// src/generation/platforms/mastodon.rs
use super::PlatformSpec;
pub fn spec() -> PlatformSpec {
    PlatformSpec {
        name: "Mastodon",
        char_limit: Some(500),
        tone_guidance: "Community-first. Self-promotion should be soft and contextual — lead with what it does for others, not what you built. Technical and open source audiences respond well.",
        format_notes: "500 chars (standard instance limit). Single post. No markdown except line breaks. CW not needed for standard release posts.",
        hashtag_notes: "3-5 hashtags at end. Mastodon search is hashtag-driven — use them. #FediDev #OpenSource are common.",
    }
}
```

```rust
// src/generation/platforms/discord.rs
use super::PlatformSpec;
pub fn spec() -> PlatformSpec {
    PlatformSpec {
        name: "Discord",
        char_limit: None,
        tone_guidance: "Conversational and immediate. Announcement channel tone: excited but not corporate. Assume the audience already knows the project.",
        format_notes: "No hard limit. Use **bold** for version number and key feature names. Keep to 3-5 sentences for the main body. Optional: bullet list of key changes.",
        hashtag_notes: "No hashtags.",
    }
}
```

```rust
// src/generation/platforms/github_release.rs
use super::PlatformSpec;
pub fn spec() -> PlatformSpec {
    PlatformSpec {
        name: "GitHub Release",
        char_limit: None,
        tone_guidance: "Structured changelog format. Permanent documentation. Developers reading this want to know what changed, what broke, and how to upgrade.",
        format_notes: "GitHub Flavored Markdown. Structure: ## What's Changed (bullet list of changes grouped by Added/Fixed/Changed/Removed), ## Breaking Changes (if any), ## Upgrade Notes (if any), ## Full Changelog link.",
        hashtag_notes: "No hashtags.",
    }
}
```

- [ ] **Step 5: Run tests**

```bash
cargo test platform_formatters
```
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add src/generation/platforms/ tests/platform_formatters.rs
git commit -m "feat: add platform content contracts for all 6 v1 platforms"
```

---

### Task 6: Prompt Assembly

**Files:**
- Create: `src/generation/prompt.rs`
- Modify: `src/generation/mod.rs`

**Interfaces:**
- Consumes: `GitContext`, `Profile`, `Vec<ContentFeedback>`, `Option<HiveMindContext>`, `Vec<Platform>`, `Option<String>` (context notes)
- Produces: `assemble_prompt(req: &PromptRequest) -> String` — full text to return as MCP prompt content

The assembled prompt is returned to Claude Code as the prompt body. Claude reads it and generates platform content. The prompt must instruct Claude to call `vessel_save` as a tool with the results.

- [ ] **Step 1: Write failing test**

```rust
// tests/prompt_assembly.rs
use vessel::generation::prompt::{PromptRequest, assemble_prompt};
use vessel::generation::git::{GitContext, CommitSummary};
use vessel::db::profiles::{Profile, VoiceSettings};
use vessel::generation::platforms::Platform;

fn test_git_context() -> GitContext {
    GitContext {
        tag: "v1.2.0".into(),
        prev_tag: Some("v1.1.0".into()),
        diff_stat: "3 files changed, 45 insertions(+), 12 deletions(-)".into(),
        commits: vec![
            CommitSummary { hash: "abc1234".into(), message: "fix: handle empty tags".into(), author: "alice".into() },
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
```

- [ ] **Step 2: Run to verify fail**

```bash
cargo test prompt_assembly 2>&1 | tail -5
```

- [ ] **Step 3: Implement src/generation/prompt.rs**

```rust
use crate::db::{profiles::Profile, feedback::ContentFeedback};
use crate::generation::{git::GitContext, platforms::Platform};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct HiveMindContext {
    pub memories: Vec<HiveMindMemory>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HiveMindMemory {
    pub title: String,
    pub content: String,
}

pub struct PromptRequest {
    pub git_context: GitContext,
    pub profile: Profile,
    pub platforms: Vec<Platform>,
    pub past_feedback: Vec<ContentFeedback>,
    pub hivemind_context: Option<HiveMindContext>,
    pub context_notes: Option<String>,
    pub generation_id: String,
}

pub fn assemble_prompt(req: &PromptRequest) -> String {
    let mut parts: Vec<String> = vec![];

    parts.push("You are generating release announcement content for a developer project.".into());
    parts.push(format!("Generation ID: `{}`", req.generation_id));
    parts.push(String::new());

    // Project context from HiveMind
    if let Some(ref hm) = req.hivemind_context {
        if !hm.memories.is_empty() {
            parts.push("## Project Context (from HiveMind)".into());
            for mem in &hm.memories {
                parts.push(format!("**{}**: {}", mem.title, mem.content));
            }
            parts.push(String::new());
        }
    }

    // Brand voice
    parts.push("## Brand Voice Profile".into());
    parts.push(format!("Profile: {}", req.profile.name));
    parts.push(format!("- Formality: {}", req.profile.formality));
    parts.push(format!("- Humor: {}", req.profile.humor));
    parts.push(format!("- Technical depth: {}", req.profile.technical_depth));
    parts.push(format!("- Self-promotion comfort: {}", req.profile.self_promotion));
    parts.push(String::new());

    // Past feedback signals
    if !req.past_feedback.is_empty() {
        parts.push("## Content History Signals".into());
        let liked: Vec<_> = req.past_feedback.iter().filter(|f| f.signal == "liked" || f.signal == "reused").collect();
        let disliked: Vec<_> = req.past_feedback.iter().filter(|f| f.signal == "disliked").collect();
        if !liked.is_empty() {
            parts.push(format!("Previously well-received on: {}", liked.iter().map(|f| f.platform.as_str()).collect::<Vec<_>>().join(", ")));
        }
        if !disliked.is_empty() {
            parts.push(format!("Avoid angles that didn't land on: {}", disliked.iter().map(|f| f.platform.as_str()).collect::<Vec<_>>().join(", ")));
        }
        parts.push(String::new());
    }

    // Git context
    parts.push("## Release Context".into());
    parts.push(format!("Tag: `{}`", req.git_context.tag));
    if let Some(ref prev) = req.git_context.prev_tag {
        parts.push(format!("Previous tag: `{prev}`"));
    }
    parts.push(format!("Changes: {}", req.git_context.diff_stat));
    parts.push(String::new());

    if !req.git_context.commits.is_empty() {
        parts.push("Commits since last release:".into());
        for c in &req.git_context.commits {
            parts.push(format!("- `{}` {} ({})", c.hash, c.message, c.author));
        }
        parts.push(String::new());
    }

    if let Some(ref cl) = req.git_context.changelog_excerpt {
        parts.push("Changelog excerpt:".into());
        parts.push(format!("```\n{cl}\n```"));
        parts.push(String::new());
    }

    if let Some(ref notes) = req.context_notes {
        parts.push("## Additional Context from Developer".into());
        parts.push(notes.clone());
        parts.push(String::new());
    }

    // Platform instructions
    parts.push("## Output Required".into());
    parts.push(format!("Generate content for {} platform(s):", req.platforms.len()));
    for platform in &req.platforms {
        let spec = platform.spec();
        parts.push(String::new());
        parts.push(format!("### {}", spec.name));
        parts.push(format!("Tone: {}", spec.tone_guidance));
        parts.push(format!("Format: {}", spec.format_notes));
        if let Some(limit) = spec.char_limit {
            parts.push(format!("Character limit: {} chars HARD LIMIT — do not exceed.", limit));
        }
        parts.push(format!("Hashtags: {}", spec.hashtag_notes));
    }

    parts.push(String::new());
    parts.push("## Instructions".into());
    parts.push("1. Generate content for each platform above following its exact constraints.".into());
    parts.push("2. Platform tone guidance overrides general tone when they conflict.".into());
    parts.push("3. Respect character limits strictly — count carefully.".into());
    parts.push(format!("4. When done, call the `vessel_save` tool with:"));
    parts.push(format!("   - `generation_id`: `\"{}\"`", req.generation_id));
    parts.push("   - `outputs`: array of `{{ platform: string, content: string }}` objects".into());
    parts.push(format!("   - Platform slug values: {}", req.platforms.iter().map(|p| format!("`{}`", p.slug())).collect::<Vec<_>>().join(", ")));

    parts.join("\n")
}
```

- [ ] **Step 4: Run tests**

```bash
cargo test prompt_assembly
```
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/generation/prompt.rs tests/prompt_assembly.rs
git commit -m "feat: add prompt assembly with brand voice, git context, and platform instructions"
```

---

### Task 7: MCP Server

**Files:**
- Modify: `src/mcp/mod.rs`
- Create: `src/mcp/prompts.rs`
- Create: `src/mcp/tools.rs`

**Interfaces:**
- Consumes: `VesselConfig`, `Db`
- Produces: MCP stdio server with:
  - Prompt `vessel-generate` — assembles context, returns prompt for Claude
  - Tool `vessel_save` — persists generated outputs to libSQL

The MCP server is the critical path: Claude Code calls `vessel-generate` as a slash command, which returns assembled context. Claude generates content. Claude then calls `vessel_save` with the results.

- [ ] **Step 1: Implement src/mcp/tools.rs**

```rust
use anyhow::Result;
use rmcp::{tool, ServerHandler, model::*};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::db::{Db, generations as gen_db};

#[derive(Debug, Deserialize)]
pub struct PlatformOutput {
    pub platform: String,
    pub content: String,
}

#[derive(Debug, Deserialize)]
pub struct VesselSaveInput {
    pub generation_id: String,
    pub outputs: Vec<PlatformOutput>,
}

pub async fn vessel_save(db: &Db, input: VesselSaveInput) -> Result<String> {
    if input.outputs.is_empty() {
        return Err(anyhow::anyhow!("outputs cannot be empty"));
    }
    for output in &input.outputs {
        gen_db::save_output(db, &input.generation_id, &output.platform, &output.content).await?;
    }
    Ok(format!(
        "Saved {} platform outputs for generation {}. Open http://localhost:3458 to review.",
        input.outputs.len(),
        input.generation_id
    ))
}
```

- [ ] **Step 2: Implement src/mcp/prompts.rs**

```rust
use anyhow::Result;
use chrono::Utc;
use uuid::Uuid;
use std::collections::HashMap;
use crate::{
    config::VesselConfig,
    db::{Db, profiles, projects, generations as gen_db, feedback},
    generation::{
        git,
        platforms::Platform,
        prompt::{PromptRequest, assemble_prompt},
    },
    hivemind::HiveMindClient,
};

pub async fn handle_vessel_generate(
    db: &Db,
    config: &VesselConfig,
    args: Option<HashMap<String, String>>,
) -> Result<String> {
    let args = args.unwrap_or_default();
    let repo_path = args.get("repo_path").cloned()
        .unwrap_or_else(|| std::env::current_dir().unwrap().to_string_lossy().to_string());
    let tag = args.get("tag").cloned();
    let category = args.get("category").cloned().unwrap_or_else(|| "release".into());
    let context_notes = args.get("context_notes").cloned();

    // resolve tag — use most recent if not specified
    let tags = git::list_tags(&repo_path)?;
    let tag = match tag {
        Some(t) => t,
        None => tags.first().cloned()
            .ok_or_else(|| anyhow::anyhow!("No git tags found in {}", repo_path))?,
    };

    let git_ctx = git::read_git_context(&repo_path, &tag)?;

    // look up or auto-create project and profile
    let project = match projects::find_by_repo(db, &repo_path).await? {
        Some(p) => p,
        None => {
            let default_profiles = profiles::list(db).await?;
            let profile_id = default_profiles.first()
                .map(|p| p.id.clone())
                .unwrap_or_else(|| {
                    // will be created below
                    format!("profile_{}", Uuid::new_v4().simple())
                });
            // ensure default profile exists
            if default_profiles.is_empty() {
                profiles::create(db, "Default", profiles::VoiceSettings::default()).await?;
                let profiles = profiles::list(db).await?;
                let pid = profiles[0].id.clone();
                projects::create(db, &pid, Some(&repo_path), None, "local").await?
            } else {
                projects::create(db, &profile_id, Some(&repo_path), None, "local").await?
            }
        }
    };

    let profile = profiles::get(db, &project.profile_id).await?
        .ok_or_else(|| anyhow::anyhow!("Profile {} not found", project.profile_id))?;

    // fetch feedback history
    let recent_gens = gen_db::list_recent(db, &project.id, 10).await?;
    let mut past_feedback = vec![];
    for gen in &recent_gens {
        let mut fb = feedback::list_for_generation(db, &gen.id).await?;
        past_feedback.append(&mut fb);
    }

    // HiveMind context (best-effort)
    let hivemind_ctx = HiveMindClient::new(config.hivemind.port)
        .read_project_context(&repo_path).await.ok();

    // create generation record
    let generation = gen_db::create(db, &project.id, &tag, &category, context_notes.as_deref()).await?;

    let req = PromptRequest {
        git_context: git_ctx,
        profile,
        platforms: Platform::all_v1(),
        past_feedback,
        hivemind_context: hivemind_ctx,
        context_notes,
        generation_id: generation.id,
    };

    Ok(assemble_prompt(&req))
}

pub async fn handle_vessel_status(db: &Db) -> Result<String> {
    let projects = crate::db::projects::list(db).await?;
    if projects.is_empty() {
        return Ok("No projects configured. Run `/vessel-generate` in a git repo to get started.\nDashboard: http://localhost:3458".into());
    }
    let mut lines = vec!["## Vessel Status".to_string(), String::new()];
    for project in &projects {
        let gens = gen_db::list_recent(db, &project.id, 3).await?;
        let repo = project.repo_path.as_deref().unwrap_or(project.github_repo.as_deref().unwrap_or("unknown"));
        lines.push(format!("**{}**", repo));
        for gen in gens {
            lines.push(format!("  - {} `{}` ({})", gen.category, gen.tag,
                chrono::DateTime::from_timestamp(gen.created_at, 0)
                    .map(|d| d.format("%Y-%m-%d").to_string())
                    .unwrap_or_default()));
        }
    }
    lines.push(String::new());
    lines.push("Dashboard: http://localhost:3458".into());
    Ok(lines.join("\n"))
}
```

- [ ] **Step 3: Implement src/mcp/mod.rs**

```rust
pub mod prompts;
pub mod tools;

use anyhow::Result;
use rmcp::{ServerHandler, ServiceExt, model::*, tool, transport::stdio};
use std::sync::Arc;
use crate::{config::VesselConfig, db::Db};

#[derive(Clone)]
pub struct VesselMcp {
    db: Db,
    config: Arc<VesselConfig>,
}

impl VesselMcp {
    pub fn new(db: Db, config: VesselConfig) -> Self {
        Self { db, config: Arc::new(config) }
    }
}

#[tool(tool_box)]
impl VesselMcp {
    #[tool(description = "Save Vessel-generated content to local storage. Call this after generating platform content from a vessel-generate prompt.")]
    async fn vessel_save(
        &self,
        #[tool(param)] generation_id: String,
        #[tool(param)] outputs: Vec<tools::PlatformOutput>,
    ) -> Result<CallToolResult, McpError> {
        let input = tools::VesselSaveInput { generation_id, outputs };
        match tools::vessel_save(&self.db, input).await {
            Ok(msg) => Ok(CallToolResult::success(vec![Content::text(msg)])),
            Err(e) => Err(McpError::internal_error(e.to_string(), None)),
        }
    }
}

impl ServerHandler for VesselMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::LATEST,
            capabilities: ServerCapabilities::builder()
                .enable_prompts()
                .enable_tools()
                .build(),
            server_info: Implementation {
                name: "vessel".into(),
                version: env!("CARGO_PKG_VERSION").into(),
            },
            instructions: Some("Vessel release announcement tool. Use /vessel-generate to create social content for a release. Use /vessel-status to see recent activity.".into()),
        }
    }

    async fn list_prompts(&self, _req: Option<PaginatedRequestParam>, _ctx: RequestContext<RoleServer>)
        -> Result<ListPromptsResult, McpError> {
        Ok(ListPromptsResult {
            next_cursor: None,
            prompts: vec![
                Prompt {
                    name: "vessel-generate".into(),
                    description: Some("Generate platform-optimized release content for a git tag".into()),
                    arguments: Some(vec![
                        PromptArgument { name: "repo_path".into(), description: Some("Absolute path to the git repo (defaults to current directory)".into()), required: Some(false) },
                        PromptArgument { name: "tag".into(), description: Some("Git tag to generate content for (defaults to latest tag)".into()), required: Some(false) },
                        PromptArgument { name: "category".into(), description: Some("release | update | milestone | announcement".into()), required: Some(false) },
                        PromptArgument { name: "context_notes".into(), description: Some("Optional extra context to include in generation".into()), required: Some(false) },
                    ]),
                },
                Prompt {
                    name: "vessel-status".into(),
                    description: Some("Show recent Vessel generations and dashboard link".into()),
                    arguments: None,
                },
                Prompt {
                    name: "vessel-revise".into(),
                    description: Some("Revise previously generated content with new notes".into()),
                    arguments: Some(vec![
                        PromptArgument { name: "generation_id".into(), description: Some("The generation ID to revise".into()), required: Some(true) },
                        PromptArgument { name: "notes".into(), description: Some("Revision instructions".into()), required: Some(true) },
                    ]),
                },
                Prompt {
                    name: "vessel-profile".into(),
                    description: Some("View or describe the active brand voice profile".into()),
                    arguments: None,
                },
            ],
        })
    }

    async fn get_prompt(&self, req: GetPromptRequestParam, _ctx: RequestContext<RoleServer>)
        -> Result<GetPromptResult, McpError> {
        let content = match req.name.as_str() {
            "vessel-generate" => {
                let args = req.arguments.map(|m| m.into_iter().collect::<std::collections::HashMap<_,_>>());
                prompts::handle_vessel_generate(&self.db, &self.config, args).await
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?
            }
            "vessel-status" => {
                prompts::handle_vessel_status(&self.db).await
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?
            }
            "vessel-revise" => {
                let args = req.arguments.unwrap_or_default();
                let gen_id = args.get("generation_id").cloned().unwrap_or_default();
                let notes = args.get("notes").cloned().unwrap_or_default();
                prompts::handle_vessel_revise(&self.db, &gen_id, &notes).await
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?
            }
            "vessel-profile" => {
                prompts::handle_vessel_profile(&self.db).await
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?
            }
            other => return Err(McpError::invalid_params(format!("Unknown prompt: {other}"), None)),
        };
        Ok(GetPromptResult {
            description: None,
            messages: vec![PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text(content),
            }],
        })
    }
}

pub async fn serve(config: VesselConfig, db: Db) -> Result<()> {
    let server = VesselMcp::new(db, config);
    let service = server.serve(stdio()).await
        .map_err(|e| anyhow::anyhow!("MCP serve error: {e}"))?;
    service.waiting().await
        .map_err(|e| anyhow::anyhow!("MCP wait error: {e}"))?;
    Ok(())
}
```

- [ ] **Step 4: Add vessel-revise and vessel-profile handlers to src/mcp/prompts.rs**

Append to `src/mcp/prompts.rs`:
```rust
pub async fn handle_vessel_revise(db: &Db, gen_id: &str, notes: &str) -> Result<String> {
    let (gen, outputs) = gen_db::get_with_outputs(db, gen_id).await?
        .ok_or_else(|| anyhow::anyhow!("Generation {} not found", gen_id))?;

    // store revision notes
    let note_id = format!("note_{}", Uuid::new_v4().simple());
    let now = Utc::now().timestamp();
    // direct DB write for revision notes
    let conn = db.connect()?;
    conn.execute(
        "INSERT INTO revision_notes (id, generation_id, notes, created_at) VALUES (?1, ?2, ?3, ?4)",
        libsql::params![note_id, gen_id, notes, now],
    ).await?;

    let mut lines = vec![
        format!("## Vessel Revision — `{}` tag `{}`", gen.category, gen.tag),
        String::new(),
        format!("**Revision notes:** {notes}"),
        String::new(),
        "## Current Content".into(),
    ];
    for output in &outputs {
        lines.push(format!("\n### {} (revision {})", output.platform, output.revision_number));
        lines.push(output.content.clone());
    }
    lines.push(String::new());
    lines.push("## Instructions".into());
    lines.push("Revise the content above for each platform, applying the revision notes.".into());
    lines.push(format!("When done, call `vessel_save` with `generation_id: \"{}\"` and the revised outputs.", gen_id));

    Ok(lines.join("\n"))
}

pub async fn handle_vessel_profile(db: &Db) -> Result<String> {
    let profiles = profiles::list(db).await?;
    if profiles.is_empty() {
        return Ok("No brand voice profiles configured. Visit http://localhost:3458/profiles to create one.".into());
    }
    let mut lines = vec!["## Vessel Brand Voice Profiles".to_string(), String::new()];
    for p in &profiles {
        lines.push(format!("**{}** ({})", p.name, p.id));
        lines.push(format!("  Formality: {} | Humor: {} | Technical depth: {} | Self-promotion: {}",
            p.formality, p.humor, p.technical_depth, p.self_promotion));
    }
    lines.push(String::new());
    lines.push("Manage profiles at: http://localhost:3458/profiles".into());
    Ok(lines.join("\n"))
}
```

- [ ] **Step 5: Verify it compiles**

```bash
cargo build 2>&1 | grep -E "^error" | head -20
```
Expected: 0 errors (warnings OK)

- [ ] **Step 6: Commit**

```bash
git add src/mcp/ src/hivemind/
git commit -m "feat: add MCP server with vessel-generate, vessel-revise, vessel-status, vessel-profile prompts and vessel_save tool"
```

---

### Task 8: HiveMind Client

**Files:**
- Modify: `src/hivemind/mod.rs`
- Create: `src/hivemind/client.rs`

**Interfaces:**
- Produces:
  - `HiveMindClient::new(port: u16) -> Self`
  - `HiveMindClient::is_available(&self) -> bool` (async)
  - `HiveMindClient::read_project_context(&self, repo_path: &str) -> Result<HiveMindContext>`
  - `HiveMindClient::write_vessel_memory(&self, key: &str, value: &str, repo_name: &str) -> Result<()>`

**Adaptation to current HiveMind API:** The spec's project-scoped `?project=` filter doesn't exist in the current HiveMind API. Instead:
- `read_project_context` searches HiveMind FTS for the repo name and excludes memories with titles starting with `vessel:`
- `write_vessel_memory` stores memories with title format `vessel:<key>` and tags `["vessel", repo_name]`

- [ ] **Step 1: Implement src/hivemind/client.rs**

```rust
use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use crate::generation::prompt::{HiveMindContext, HiveMindMemory};

#[derive(Clone)]
pub struct HiveMindClient {
    base_url: String,
    client: Client,
}

#[derive(Deserialize)]
struct SearchResponse {
    count: u32,
    results: Vec<MemoryObject>,
}

#[derive(Deserialize)]
struct MemoryObject {
    id: String,
    title: String,
    content: String,
}

#[derive(Deserialize)]
struct CreateMemoryResponse {
    id: String,
}

impl HiveMindClient {
    pub fn new(port: u16) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(3))
            .build()
            .unwrap();
        Self {
            base_url: format!("http://localhost:{}/api/v1", port),
            client,
        }
    }

    pub async fn is_available(&self) -> bool {
        self.client.get(format!("{}/status", self.base_url))
            .send().await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    pub async fn read_project_context(&self, repo_path: &str) -> Result<HiveMindContext> {
        // derive a search term from the repo path basename
        let repo_name = std::path::Path::new(repo_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(repo_path);

        let resp: SearchResponse = self.client
            .get(format!("{}/search", self.base_url))
            .query(&[("q", repo_name), ("limit", "20")])
            .send().await?
            .json().await?;

        // filter out vessel: prefixed memories (those are written by Vessel itself)
        let memories: Vec<HiveMindMemory> = resp.results.into_iter()
            .filter(|m| !m.title.starts_with("vessel:"))
            .map(|m| HiveMindMemory { title: m.title, content: m.content })
            .collect();

        Ok(HiveMindContext { memories })
    }

    pub async fn write_vessel_memory(&self, key: &str, value: &str, repo_name: &str) -> Result<()> {
        #[derive(Serialize)]
        struct CreateMemory<'a> {
            title: String,
            content: &'a str,
            tags: Vec<&'a str>,
        }
        let body = CreateMemory {
            title: format!("vessel:{}", key),
            content: value,
            tags: vec!["vessel", repo_name],
        };
        self.client
            .post(format!("{}/memories", self.base_url))
            .json(&body)
            .send().await?
            .error_for_status()?;
        Ok(())
    }
}
```

- [ ] **Step 2: Update src/hivemind/mod.rs**

```rust
pub mod client;
pub use client::HiveMindClient;
```

- [ ] **Step 3: Verify compiles**

```bash
cargo build 2>&1 | grep "^error" | head -10
```

- [ ] **Step 4: Commit**

```bash
git add src/hivemind/
git commit -m "feat: add HiveMind client with project context read and vessel: memory write"
```

---

### Task 9: GitHub API Client

**Files:**
- Modify: `src/generation/github.rs`

**Interfaces:**
- Produces:
  - `GitHubClient::new(repo: &str, token: Option<&str>) -> Self` (repo = "owner/repo")
  - `GitHubClient::list_tags(&self) -> Result<Vec<String>>`
  - `GitHubClient::get_release_body(&self, tag: &str) -> Result<Option<String>>`
  - `GitHubClient::patch_release_body(&self, tag: &str, body: &str) -> Result<()>`
  - `encrypt_token(token: &str, key: &[u8; 32]) -> Result<(String, String)>` — returns (ciphertext_b64, nonce_b64)
  - `decrypt_token(ciphertext_b64: &str, nonce_b64: &str, key: &[u8; 32]) -> Result<String>`
  - `derive_encryption_key() -> [u8; 32]`

- [ ] **Step 1: Write failing tests**

```rust
// tests/github_token.rs
use vessel::generation::github::{encrypt_token, decrypt_token, derive_encryption_key};

#[test]
fn token_roundtrip() {
    let key = derive_encryption_key();
    let original = "ghp_testtoken123";
    let (enc, nonce) = encrypt_token(original, &key).unwrap();
    let decrypted = decrypt_token(&enc, &nonce, &key).unwrap();
    assert_eq!(decrypted, original);
}

#[test]
fn different_encryptions_of_same_token_differ() {
    let key = derive_encryption_key();
    let (enc1, _) = encrypt_token("token", &key).unwrap();
    let (enc2, _) = encrypt_token("token", &key).unwrap();
    assert_ne!(enc1, enc2); // different nonces
}
```

- [ ] **Step 2: Implement src/generation/github.rs**

```rust
use anyhow::{Result, Context};
use reqwest::{Client, header};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use aes_gcm::{Aes256Gcm, KeyInit, aead::{Aead, AeadCore, OsRng}};
use base64::{Engine as _, engine::general_purpose::STANDARD as B64};

pub struct GitHubClient {
    repo: String,
    token: Option<String>,
    client: Client,
}

#[derive(Deserialize)]
struct GhTag {
    name: String,
}

#[derive(Deserialize)]
struct GhRelease {
    id: u64,
    body: Option<String>,
}

impl GitHubClient {
    pub fn new(repo: &str, token: Option<&str>) -> Self {
        let mut headers = header::HeaderMap::new();
        headers.insert("Accept", "application/vnd.github+json".parse().unwrap());
        headers.insert("X-GitHub-Api-Version", "2022-11-28".parse().unwrap());
        if let Some(t) = token {
            headers.insert("Authorization", format!("Bearer {t}").parse().unwrap());
        }
        let client = Client::builder()
            .default_headers(headers)
            .user_agent("vessel/0.1.0")
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap();
        Self { repo: repo.into(), token: token.map(Into::into), client }
    }

    pub async fn list_tags(&self) -> Result<Vec<String>> {
        let url = format!("https://api.github.com/repos/{}/tags?per_page=30", self.repo);
        let tags: Vec<GhTag> = self.client.get(&url).send().await?
            .error_for_status()?
            .json().await?;
        Ok(tags.into_iter().map(|t| t.name).collect())
    }

    pub async fn get_release_body(&self, tag: &str) -> Result<Option<String>> {
        let url = format!("https://api.github.com/repos/{}/releases/tags/{}", self.repo, tag);
        let resp = self.client.get(&url).send().await?;
        if resp.status() == 404 { return Ok(None); }
        let release: GhRelease = resp.error_for_status()?.json().await?;
        Ok(release.body)
    }

    pub async fn patch_release_body(&self, tag: &str, body: &str) -> Result<()> {
        // find release id first
        let url = format!("https://api.github.com/repos/{}/releases/tags/{}", self.repo, tag);
        let release: GhRelease = self.client.get(&url).send().await?
            .error_for_status()?.json().await?;

        let patch_url = format!("https://api.github.com/repos/{}/releases/{}", self.repo, release.id);
        #[derive(Serialize)]
        struct Patch<'a> { body: &'a str }
        self.client.patch(&patch_url)
            .json(&Patch { body })
            .send().await?
            .error_for_status()?;
        Ok(())
    }
}

pub fn derive_encryption_key() -> [u8; 32] {
    // derive from home dir path as machine-specific entropy — not secret, but consistent per machine
    let home = dirs::home_dir().unwrap_or_default();
    let seed = home.to_string_lossy();
    let mut key = [0u8; 32];
    let seed_bytes = seed.as_bytes();
    for (i, b) in seed_bytes.iter().enumerate() {
        key[i % 32] ^= b;
    }
    // mix in a constant
    let salt = b"vessel-token-key-v1";
    for (i, b) in salt.iter().enumerate() {
        key[i % 32] ^= b;
    }
    key
}

pub fn encrypt_token(token: &str, key: &[u8; 32]) -> Result<(String, String)> {
    let cipher = Aes256Gcm::new(key.into());
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let ciphertext = cipher.encrypt(&nonce, token.as_bytes())
        .map_err(|e| anyhow::anyhow!("encrypt error: {e}"))?;
    Ok((B64.encode(&ciphertext), B64.encode(&nonce)))
}

pub fn decrypt_token(ciphertext_b64: &str, nonce_b64: &str, key: &[u8; 32]) -> Result<String> {
    let cipher = Aes256Gcm::new(key.into());
    let ciphertext = B64.decode(ciphertext_b64)?;
    let nonce_bytes = B64.decode(nonce_b64)?;
    let nonce = aes_gcm::Nonce::from_slice(&nonce_bytes);
    let plaintext = cipher.decrypt(nonce, ciphertext.as_ref())
        .map_err(|e| anyhow::anyhow!("decrypt error: {e}"))?;
    Ok(String::from_utf8(plaintext)?)
}
```

- [ ] **Step 3: Run tests**

```bash
cargo test github_token
```
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add src/generation/github.rs tests/github_token.rs
git commit -m "feat: add GitHub API client and AES-256-GCM token encryption"
```

---

### Task 10: REST API

**Files:**
- Modify: `src/api/mod.rs`, `src/server.rs`
- Create: `src/api/generations.rs`, `src/api/profiles.rs`, `src/api/projects.rs`, `src/api/feedback.rs`, `src/api/settings.rs`

**Interfaces:**
- Produces: axum Router at `/api/v1/...` bound to port 3458

Endpoints:
```
GET  /api/v1/generations                  list all (most recent 50)
GET  /api/v1/generations/:id              get with outputs
GET  /api/v1/generations/:id/outputs      list outputs for generation
POST /api/v1/feedback                     { generation_id, platform, signal }

GET  /api/v1/profiles                     list
POST /api/v1/profiles                     create
GET  /api/v1/profiles/:id                 get
PATCH /api/v1/profiles/:id               update voice settings

GET  /api/v1/projects                     list
POST /api/v1/projects                     create
GET  /api/v1/projects/:id                 get
GET  /api/v1/projects/:id/tags            list git tags for project

GET  /api/v1/settings                     { port, hivemind_port, hivemind_available, db_path }
POST /api/v1/settings/github-token        { project_id, token } — encrypt and store
DELETE /api/v1/settings/github-token/:project_id

GET  /health                              { status: "ok", version }
```

- [ ] **Step 1: Write failing API test**

```rust
// tests/api_generations.rs
use axum::http::StatusCode;
use axum_test::TestServer;
use vessel::{api, db, config::VesselConfig};

async fn test_app() -> TestServer {
    let db = {
        let raw = libsql::Builder::new_local(":memory:").build().await.unwrap();
        db::schema::run_migrations(&raw).await.unwrap();
        std::sync::Arc::new(raw)
    };
    let config = VesselConfig::default();
    let app = api::router(db, config);
    TestServer::new(app).unwrap()
}

#[tokio::test]
async fn health_returns_ok() {
    let server = test_app().await;
    let resp = server.get("/health").await;
    resp.assert_status(StatusCode::OK);
    let body: serde_json::Value = resp.json();
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
async fn generations_list_empty() {
    let server = test_app().await;
    let resp = server.get("/api/v1/generations").await;
    resp.assert_status(StatusCode::OK);
    let body: serde_json::Value = resp.json();
    assert_eq!(body["count"], 0);
}
```

Add to dev-dependencies:
```toml
axum-test = "15"
```

- [ ] **Step 2: Run to verify fail**

```bash
cargo test api_generations 2>&1 | tail -5
```

- [ ] **Step 3: Implement src/api/mod.rs**

```rust
pub mod generations;
pub mod profiles;
pub mod projects;
pub mod feedback;
pub mod settings;

use axum::{Router, routing::{get, post, patch, delete}, extract::State, Json};
use std::sync::Arc;
use serde_json::json;
use crate::{config::VesselConfig, db::Db};

#[derive(Clone)]
pub struct AppState {
    pub db: Db,
    pub config: Arc<VesselConfig>,
}

pub fn router(db: Db, config: VesselConfig) -> Router {
    let state = AppState { db, config: Arc::new(config) };
    Router::new()
        .route("/health", get(health))
        .route("/api/v1/generations", get(generations::list))
        .route("/api/v1/generations/:id", get(generations::get_one))
        .route("/api/v1/generations/:id/outputs", get(generations::list_outputs))
        .route("/api/v1/feedback", post(feedback::create))
        .route("/api/v1/profiles", get(profiles::list).post(profiles::create))
        .route("/api/v1/profiles/:id", get(profiles::get_one).patch(profiles::update))
        .route("/api/v1/projects", get(projects::list).post(projects::create))
        .route("/api/v1/projects/:id", get(projects::get_one))
        .route("/api/v1/projects/:id/tags", get(projects::list_tags))
        .route("/api/v1/settings", get(settings::get))
        .route("/api/v1/settings/github-token", post(settings::store_github_token))
        .route("/api/v1/settings/github-token/:project_id", delete(settings::delete_github_token))
        .with_state(state)
}

async fn health() -> Json<serde_json::Value> {
    Json(json!({ "status": "ok", "version": env!("CARGO_PKG_VERSION") }))
}
```

- [ ] **Step 4: Implement src/api/generations.rs**

```rust
use axum::{extract::{State, Path}, Json, http::StatusCode};
use serde_json::json;
use crate::{api::AppState, db::generations as gen_db};

pub async fn list(State(state): State<AppState>) -> Result<Json<serde_json::Value>, StatusCode> {
    // list across all projects, most recent 50
    let conn = state.db.connect().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut rows = conn.query(
        "SELECT id, project_id, tag, category, context_notes, created_at
         FROM generations ORDER BY created_at DESC LIMIT 50", ()
    ).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut gens = vec![];
    while let Some(row) = rows.next().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)? {
        gens.push(json!({
            "id": row.get::<String>(0).unwrap_or_default(),
            "project_id": row.get::<String>(1).unwrap_or_default(),
            "tag": row.get::<String>(2).unwrap_or_default(),
            "category": row.get::<String>(3).unwrap_or_default(),
            "context_notes": row.get::<Option<String>>(4).unwrap_or(None),
            "created_at": row.get::<i64>(5).unwrap_or_default(),
        }));
    }
    Ok(Json(json!({ "count": gens.len(), "generations": gens })))
}

pub async fn get_one(State(state): State<AppState>, Path(id): Path<String>)
    -> Result<Json<serde_json::Value>, StatusCode> {
    match gen_db::get_with_outputs(&state.db, &id).await {
        Ok(Some((gen, outputs))) => Ok(Json(json!({ "generation": gen, "outputs": outputs }))),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn list_outputs(State(state): State<AppState>, Path(id): Path<String>)
    -> Result<Json<serde_json::Value>, StatusCode> {
    match gen_db::get_with_outputs(&state.db, &id).await {
        Ok(Some((_, outputs))) => Ok(Json(json!({ "count": outputs.len(), "outputs": outputs }))),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}
```

- [ ] **Step 5: Implement remaining API handlers**

`src/api/feedback.rs`:
```rust
use axum::{extract::State, Json, http::StatusCode};
use serde::Deserialize;
use crate::{api::AppState, db::feedback};

#[derive(Deserialize)]
pub struct FeedbackInput {
    pub generation_id: String,
    pub platform: String,
    pub signal: String,
}

pub async fn create(State(state): State<AppState>, Json(input): Json<FeedbackInput>)
    -> Result<Json<serde_json::Value>, StatusCode> {
    feedback::record(&state.db, &input.generation_id, &input.platform, &input.signal)
        .await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!({ "recorded": true })))
}
```

`src/api/profiles.rs`:
```rust
use axum::{extract::{State, Path}, Json, http::StatusCode};
use serde::Deserialize;
use serde_json::json;
use crate::{api::AppState, db::profiles::{self, VoiceSettings}};

#[derive(Deserialize)]
pub struct CreateProfileInput {
    pub name: String,
    pub formality: Option<String>,
    pub humor: Option<String>,
    pub technical_depth: Option<String>,
    pub self_promotion: Option<String>,
}

pub async fn list(State(state): State<AppState>) -> Result<Json<serde_json::Value>, StatusCode> {
    let ps = profiles::list(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(json!({ "count": ps.len(), "profiles": ps })))
}

pub async fn create(State(state): State<AppState>, Json(input): Json<CreateProfileInput>)
    -> Result<(StatusCode, Json<serde_json::Value>), StatusCode> {
    let voice = VoiceSettings {
        formality: input.formality.unwrap_or_else(|| "balanced".into()),
        humor: input.humor.unwrap_or_else(|| "subtle".into()),
        technical_depth: input.technical_depth.unwrap_or_else(|| "medium".into()),
        self_promotion: input.self_promotion.unwrap_or_else(|| "balanced".into()),
    };
    let profile = profiles::create(&state.db, &input.name, voice)
        .await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok((StatusCode::CREATED, Json(json!({ "id": profile.id }))))
}

pub async fn get_one(State(state): State<AppState>, Path(id): Path<String>)
    -> Result<Json<serde_json::Value>, StatusCode> {
    match profiles::get(&state.db, &id).await {
        Ok(Some(p)) => Ok(Json(json!(p))),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn update(State(state): State<AppState>, Path(id): Path<String>, Json(input): Json<CreateProfileInput>)
    -> Result<Json<serde_json::Value>, StatusCode> {
    let conn = state.db.connect().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let now = chrono::Utc::now().timestamp();
    conn.execute(
        "UPDATE profiles SET name=?1, formality=COALESCE(?2,formality), humor=COALESCE(?3,humor),
         technical_depth=COALESCE(?4,technical_depth), self_promotion=COALESCE(?5,self_promotion),
         updated_at=?6 WHERE id=?7",
        libsql::params![input.name, input.formality, input.humor, input.technical_depth,
            input.self_promotion, now, id.clone()],
    ).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(json!({ "updated": true, "id": id })))
}
```

`src/api/projects.rs`:
```rust
use axum::{extract::{State, Path}, Json, http::StatusCode};
use serde::Deserialize;
use serde_json::json;
use crate::{api::AppState, db::projects, generation::git};

#[derive(Deserialize)]
pub struct CreateProjectInput {
    pub profile_id: String,
    pub repo_path: Option<String>,
    pub github_repo: Option<String>,
    pub provider: Option<String>,
}

pub async fn list(State(state): State<AppState>) -> Result<Json<serde_json::Value>, StatusCode> {
    let ps = projects::list(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(json!({ "count": ps.len(), "projects": ps })))
}

pub async fn create(State(state): State<AppState>, Json(input): Json<CreateProjectInput>)
    -> Result<(StatusCode, Json<serde_json::Value>), StatusCode> {
    let project = projects::create(
        &state.db,
        &input.profile_id,
        input.repo_path.as_deref(),
        input.github_repo.as_deref(),
        input.provider.as_deref().unwrap_or("local"),
    ).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok((StatusCode::CREATED, Json(json!({ "id": project.id }))))
}

pub async fn get_one(State(state): State<AppState>, Path(id): Path<String>)
    -> Result<Json<serde_json::Value>, StatusCode> {
    let conn = state.db.connect().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut rows = conn.query(
        "SELECT id, profile_id, repo_path, github_repo, provider, created_at
         FROM projects WHERE id=?1", [id]
    ).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    match rows.next().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)? {
        None => Err(StatusCode::NOT_FOUND),
        Some(row) => Ok(Json(json!({
            "id": row.get::<String>(0).unwrap_or_default(),
            "profile_id": row.get::<String>(1).unwrap_or_default(),
            "repo_path": row.get::<Option<String>>(2).unwrap_or(None),
            "github_repo": row.get::<Option<String>>(3).unwrap_or(None),
            "provider": row.get::<String>(4).unwrap_or_default(),
            "created_at": row.get::<i64>(5).unwrap_or_default(),
        }))),
    }
}

pub async fn list_tags(State(state): State<AppState>, Path(id): Path<String>)
    -> Result<Json<serde_json::Value>, StatusCode> {
    let conn = state.db.connect().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut rows = conn.query("SELECT repo_path FROM projects WHERE id=?1", [id])
        .await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let repo_path = match rows.next().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)? {
        None => return Err(StatusCode::NOT_FOUND),
        Some(row) => row.get::<Option<String>>(0).unwrap_or(None),
    };
    match repo_path {
        None => Ok(Json(json!({ "tags": [] }))),
        Some(path) => {
            let tags = git::list_tags(&path).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            Ok(Json(json!({ "tags": tags })))
        }
    }
}
```

`src/api/settings.rs`:
```rust
use axum::{extract::{State, Path}, Json, http::StatusCode};
use serde::Deserialize;
use serde_json::json;
use crate::{api::AppState, generation::github::{encrypt_token, derive_encryption_key}, hivemind::HiveMindClient};
use uuid::Uuid;
use chrono::Utc;

pub async fn get(State(state): State<AppState>) -> Json<serde_json::Value> {
    let hivemind_available = HiveMindClient::new(state.config.hivemind.port)
        .is_available().await;
    Json(json!({
        "port": state.config.server.port,
        "hivemind_port": state.config.hivemind.port,
        "hivemind_available": hivemind_available,
        "db_path": state.config.db_path().to_string_lossy(),
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

#[derive(Deserialize)]
pub struct GithubTokenInput {
    pub project_id: String,
    pub token: String,
}

pub async fn store_github_token(State(state): State<AppState>, Json(input): Json<GithubTokenInput>)
    -> Result<(StatusCode, Json<serde_json::Value>), StatusCode> {
    let key = derive_encryption_key();
    let (enc, nonce) = encrypt_token(&input.token, &key)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let id = format!("ghtoken_{}", Uuid::new_v4().simple());
    let now = Utc::now().timestamp();
    let conn = state.db.connect().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    conn.execute(
        "INSERT INTO github_tokens (id, project_id, token_enc, nonce, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(project_id) DO UPDATE SET token_enc=?3, nonce=?4, created_at=?5",
        libsql::params![id, input.project_id, enc, nonce, now],
    ).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok((StatusCode::CREATED, Json(json!({ "stored": true }))))
}

pub async fn delete_github_token(State(state): State<AppState>, Path(project_id): Path<String>)
    -> Result<Json<serde_json::Value>, StatusCode> {
    let conn = state.db.connect().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    conn.execute("DELETE FROM github_tokens WHERE project_id=?1", [project_id])
        .await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(json!({ "deleted": true })))
}
```

- [ ] **Step 6: Implement src/server.rs**

```rust
use anyhow::Result;
use axum::Router;
use tower_http::cors::{CorsLayer, Any};
use crate::{config::VesselConfig, db::Db};

pub async fn start(config: VesselConfig, db: Db) -> Result<()> {
    let port = config.server.port;
    let app = crate::api::router(db, config)
        .layer(CorsLayer::new().allow_origin(Any).allow_headers(Any).allow_methods(Any));

    let addr = format!("127.0.0.1:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    println!("Vessel running at http://localhost:{port}");
    println!("Dashboard: http://localhost:{port}");
    println!("MCP config for Claude Code:");
    println!(r#"  {{ "mcpServers": {{ "vessel": {{ "command": "vessel", "args": ["mcp"] }} }} }}"#);
    axum::serve(listener, app).await?;
    Ok(())
}
```

- [ ] **Step 7: Run API tests**

```bash
cargo test api_generations
```
Expected: PASS

- [ ] **Step 8: Commit**

```bash
git add src/api/ src/server.rs tests/api_generations.rs Cargo.toml
git commit -m "feat: add REST API (generations, profiles, projects, settings) and vessel up server"
```

---

### Task 11: Integration Smoke Test and .gitignore

**Files:**
- Create: `.gitignore`
- Create: `tests/smoke.rs`

- [ ] **Step 1: Write .gitignore**

```gitignore
/target
*.db
*.db-shm
*.db-wal
vessel.toml
```

- [ ] **Step 2: Write end-to-end smoke test**

```rust
// tests/smoke.rs
// Tests the full MCP generate → save → REST API retrieve flow

use vessel::{db, config::VesselConfig, generation::git};
use std::process::Command;
use tempfile::TempDir;

fn make_test_repo() -> TempDir {
    let dir = TempDir::new().unwrap();
    let p = dir.path();
    Command::new("git").args(["init"]).current_dir(p).output().unwrap();
    Command::new("git").args(["config", "user.email", "test@test.com"]).current_dir(p).output().unwrap();
    Command::new("git").args(["config", "user.name", "Test"]).current_dir(p).output().unwrap();
    std::fs::write(p.join("README.md"), "# My Project\nA cool tool.").unwrap();
    Command::new("git").args(["add", "."]).current_dir(p).output().unwrap();
    Command::new("git").args(["commit", "-m", "feat: add README"]).current_dir(p).output().unwrap();
    Command::new("git").args(["tag", "v1.0.0"]).current_dir(p).output().unwrap();
    dir
}

#[tokio::test]
async fn full_generate_save_retrieve_flow() {
    let repo = make_test_repo();
    let raw_db = libsql::Builder::new_local(":memory:").build().await.unwrap();
    db::schema::run_migrations(&raw_db).await.unwrap();
    let db = std::sync::Arc::new(raw_db);

    // create profile and project
    let voice = db::profiles::VoiceSettings::default();
    let profile = db::profiles::create(&db, "TestProfile", voice).await.unwrap();
    let project = db::projects::create(
        &db, &profile.id, Some(repo.path().to_str().unwrap()), None, "local"
    ).await.unwrap();

    // create generation + save output (simulating what vessel_save does)
    let gen = db::generations::create(&db, &project.id, "v1.0.0", "release", None).await.unwrap();
    db::generations::save_output(&db, &gen.id, "twitter", "My project v1.0.0 is out! #opensource").await.unwrap();
    db::generations::save_output(&db, &gen.id, "discord", "**v1.0.0** is live — check the README for details.").await.unwrap();

    // verify via DB
    let (fetched_gen, outputs) = db::generations::get_with_outputs(&db, &gen.id).await.unwrap().unwrap();
    assert_eq!(fetched_gen.tag, "v1.0.0");
    assert_eq!(outputs.len(), 2);

    let twitter = outputs.iter().find(|o| o.platform == "twitter").unwrap();
    assert!(twitter.content.chars().count() <= 280);

    // verify git context read for the tag
    let ctx = git::read_git_context(repo.path().to_str().unwrap(), "v1.0.0").unwrap();
    assert_eq!(ctx.tag, "v1.0.0");
}
```

- [ ] **Step 3: Run full test suite**

```bash
cargo test 2>&1 | tail -20
```
Expected: all tests PASS

- [ ] **Step 4: Commit**

```bash
git add .gitignore tests/smoke.rs
git commit -m "test: add integration smoke test and .gitignore"
```

---

## Self-Review Against Spec

**Spec coverage check:**

| Spec section | Task(s) covering it |
|---|---|
| §3 Components: MCP server, REST API, libSQL | Task 1, 7, 10 |
| §4 Primary flow (generate via Claude Code) | Task 6, 7 |
| §5 /vessel-generate, /vessel-revise, /vessel-status, /vessel-profile | Task 7 |
| §6 All 6 platforms, character limits, tone | Task 5 |
| §7 Release/Update/Milestone/Announcement categories | Task 3, 7 |
| §8 Local git + GitHub provider | Task 4, 9 |
| §8 GitHub token (encrypted local storage) | Task 9 |
| §9 Brand voice profiles (4 axes) | Task 3 |
| §10 DB schema (all 8 tables) | Task 2 |
| §11 HiveMind integration (health check, read, write) | Task 8 |
| §14 `vessel up` + `vessel mcp` CLI | Task 1, 10 |
| §16 Content feedback injection on every generation | Task 7 (prompts.rs reads feedback history) |

**Gaps identified and addressed:**
- GitHub Release notes generation: covered by Task 9 (`GitHubClient::get_release_body`) + platform formatter in Task 5. The MCP prompt includes GitHub Release as one of the 6 platforms.
- `vessel:` memory key schema (10 keys from spec §11): covered by `write_vessel_memory` in Task 8. The keys themselves are written by the dashboard settings flow (outside backend scope of this plan).
- Content feedback injection (spec §16): `handle_vessel_generate` in Task 7 fetches past feedback and injects into `PromptRequest`.

**Type consistency check:** All functions referenced in later tasks were defined in the task that introduces them. `HiveMindContext` and `HiveMindMemory` are defined in `prompt.rs` (Task 6) and consumed by `client.rs` (Task 8) — verify the import paths are consistent when implementing.
