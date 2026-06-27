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
