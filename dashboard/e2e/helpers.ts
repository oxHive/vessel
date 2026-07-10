import { DatabaseSync } from 'node:sqlite'
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import crypto from 'node:crypto'

export const PORT = 3459

// One fresh isolated HOME per run. The config module is evaluated first in
// the runner process; workers inherit the env var, so every process agrees
// on the same directory without a globalSetup ordering dependency.
if (!process.env.VESSEL_E2E_HOME) {
  process.env.VESSEL_E2E_HOME = fs.mkdtempSync(path.join(os.tmpdir(), 'vessel-e2e-'))
}
export const E2E_HOME = process.env.VESSEL_E2E_HOME

export const DB_PATH = path.join(E2E_HOME, '.local/share/vessel/vessel.db')

function withDb<T>(fn: (db: DatabaseSync) => T): T {
  // The DB is created (and migrated) by the `vessel up` webServer process;
  // this opens a second connection to the same file, like the Rust e2e tests.
  const db = new DatabaseSync(DB_PATH)
  try {
    return fn(db)
  } finally {
    db.close()
  }
}

const now = () => Math.floor(Date.now() / 1000)
const uid = (prefix: string) => `${prefix}_${crypto.randomUUID().replaceAll('-', '')}`

/** Seed a reviewable generation with one twitter output; returns its id. */
export function seedGeneration(content = 'hello world release'): string {
  return withDb((db) => {
    const ts = now()
    const profileId = uid('prof')
    const projectId = uid('proj')
    const genId = uid('gen')
    db.prepare(
      `INSERT INTO profiles (id, name, formality, humor, technical_depth, self_promotion, created_at, updated_at)
       VALUES (?, 'e2e', 'balanced', 'subtle', 'medium', 'balanced', ?, ?)`,
    ).run(profileId, ts, ts)
    db.prepare(
      `INSERT INTO projects (id, profile_id, repo_path, github_repo, provider, created_at)
       VALUES (?, ?, '/repo', NULL, 'local', ?)`,
    ).run(projectId, profileId, ts)
    db.prepare(
      `INSERT INTO generations (id, project_id, tag, category, context_notes, created_at)
       VALUES (?, ?, 'v1.0.0', 'release', NULL, ?)`,
    ).run(genId, projectId, ts)
    db.prepare(
      `INSERT INTO generation_outputs (id, generation_id, platform, content, revision_number, created_at)
       VALUES (?, ?, 'twitter', ?, 0, ?)`,
    ).run(uid('out'), genId, content, ts)
    return genId
  })
}

/** Simulate the agent's vessel_save DB write: next revision for a platform. */
export function insertRevisedOutput(genId: string, platform: string, content: string): void {
  withDb((db) => {
    const row = db
      .prepare(
        `SELECT COALESCE(MAX(revision_number), -1) + 1 AS rev
         FROM generation_outputs WHERE generation_id = ? AND platform = ?`,
      )
      .get(genId, platform) as { rev: number }
    db.prepare(
      `INSERT INTO generation_outputs (id, generation_id, platform, content, revision_number, created_at)
       VALUES (?, ?, ?, ?, ?, ?)`,
    ).run(uid('out'), genId, platform, content, row.rev, now())
  })
}
