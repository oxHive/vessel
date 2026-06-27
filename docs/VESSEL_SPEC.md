# Vessel — Product & Technical Specification

*An Oxhive product*
*Status: Pre-development — specification only*
*Last updated: June 2026*

---

## 1. Product Overview

Vessel is a developer-focused release announcement tool that translates technical releases into platform-optimized social content. It is a standalone product under the Oxhive umbrella with its own identity, distribution, and install story.

**The core job to be done:** A developer has shipped something real. They cannot translate it into language that attracts users. Vessel does that translation — from git tag and changelog to ready-to-copy content across every platform the developer cares about.

**Target user:** Solo open source developers, indie game developers, and small teams without a dedicated social media or marketing function. Developers who ship consistently but communicate inconsistently.

**Primary interaction surface:** Claude Code via MCP slash commands. The web dashboard handles the visual, archival, and review layer.

---

## 2. Positioning

### Standalone with HiveMind as premium experience

Vessel works without HiveMind. A developer who has never heard of HiveMind can install Vessel and get immediate value.

HiveMind integration is the premium experience layer that unlocks:
- Persistent brand voice memory across sessions
- Cross-session content continuity ("same concept as last week")
- Project context awareness drawn from real coding session history
- Richer, more informed content generation without the user re-explaining their project

The value ladder is explicit and honest: standalone Vessel is useful, HiveMind-integrated Vessel is meaningfully better.

### Oxhive product family

Vessel is an Oxhive product. It shares the Oxhive design language, install pattern, and cross-product linking convention with HiveMind. It is not a plugin or extension of HiveMind — it is a sibling product that integrates with HiveMind when present.

---

## 3. Architecture

### Local-first

Vessel is a local tool. There is no hosted backend, no SaaS layer, no Oxhive server handling user data. Everything runs on the user's machine.

### Components

```
vessel (Rust binary)
├── MCP server          — Claude Code integration, slash commands, content generation trigger
├── REST API            — Dashboard data layer
├── Static file server  — Serves Vue 3 dashboard at localhost:PORT
└── libSQL              — Local storage for all Vessel-owned data
```

### Data flow

```
Claude Code
    │
    │  MCP slash command
    ▼
Vessel MCP Server
    │
    │  Generates content using user's Claude session (no Oxhive API cost)
    │  Reads project context from libSQL (or HiveMind if installed)
    ▼
libSQL (local)
    │
    ▼
Vessel REST API
    │
    ▼
Vue 3 Dashboard (localhost)
    │
    ▼
User reviews, revises, copies content
```

### Technology stack

| Layer | Technology |
|---|---|
| Binary language | Rust |
| MCP server | rmcp SDK |
| Database | libSQL |
| Dashboard frontend | Vue 3 with Composition API |
| Config format | TOML |

### Binary

- Crate name: `oxvessel`
- Binary name: `vessel`
- Repository: `github.com/oxhive/vessel`
- Published on: crates.io

### Ports

- Dashboard + REST API: `localhost:3458` (one port below HiveMind to maintain family convention)
- MCP server: stdio transport (same as HiveMind)

---

## 4. User Workflow

### Primary flow — Claude Code first

```
1. Developer tags a release (git tag or GitHub release)
2. Developer opens Claude Code
3. Developer runs /vessel-generate slash command
4. Vessel reads git tag, diff, and changelog from local repo or GitHub API
5. Vessel reads brand voice profile and content history from libSQL
6. Vessel reads project context from HiveMind if installed (optional)
7. Content is generated using the user's Claude session
8. Result is written to libSQL
9. Developer opens Vessel dashboard at localhost:3458
10. Developer reviews generated content per platform
11. Developer adds revision notes if needed, runs /vessel-revise
12. Developer copies final content per platform
```

### Secondary flow — Dashboard initiated

```
1. Developer opens Vessel dashboard
2. Selects New Post
3. Selects category (Release, Update, Milestone, etc.)
4. Selects project/repo
5. Dashboard lists available git tags via backend
6. Developer selects tag
7. Developer adds optional context notes
8. Dashboard displays generated slash command
9. Developer copies and runs in Claude Code
10. Result appears in dashboard automatically
11. Developer reviews, revises, copies
```

### GitHub Release optional flow

```
If GitHub token is configured:
    Vessel fetches existing release body via GitHub API
    Generates enriched release notes in GitHub markdown format
    Developer approves in dashboard
    Vessel patches only the body field via GitHub API
    Assets, binaries, and CI-managed fields are untouched

If no GitHub token:
    Vessel generates release notes in dashboard
    Developer copies and pastes manually into GitHub Release editor
```

---

## 5. MCP Slash Commands

| Command | Purpose |
|---|---|
| `/vessel-generate` | Generate content for a release. Prompts for repo and tag if not provided. |
| `/vessel-revise` | Revise previously generated content based on notes. |
| `/vessel-status` | Show recent generations and pending reviews in dashboard. |
| `/vessel-profile` | View or switch active brand voice profile. |

---

## 6. Platform Support (v1)

Vessel generates platform-optimized content for all of the following at launch. Direct posting and scheduling are explicitly deferred — copy/paste is the only publish mechanism in v1.

### Supported platforms

| Platform | Content culture | Technical constraints |
|---|---|---|
| X (Twitter) | Punchy, opinionated, thread-friendly. Character economy is everything. | 280 chars per tweet, thread support |
| LinkedIn | Narrative and professional. "I built X because Y" performs. Longer is acceptable. | 3000 chars, no thread concept |
| Bluesky | Early-adopter developer culture. Technical credibility matters. Close to early Twitter. | 300 chars |
| Mastodon | Community-first. Self-promotion needs to be softer and contextual. | 500 chars (configurable per instance) |
| Discord | Conversational and immediate. Announcement, not a post. | No hard limit, but brevity expected |
| GitHub Releases | Structured markdown. Changelog format. Permanent documentation. | GitHub Flavored Markdown |

### Platform content contracts

For each platform Vessel handles:
- Character limits and content structure
- Platform-native tone defaults
- Hashtag conventions
- Thread or multi-part formatting where applicable
- Image dimension specifications and recommendations (for platforms supporting images)

### Tone override

Users can override platform tone defaults via brand voice profile settings. Override is applied on top of platform defaults, not instead of them — the platform's structural constraints (character limits, format) always apply.

---

## 7. Content Categories (v1)

| Category | Description |
|---|---|
| Release | New version tag. Pulls from git tag, diff, and changelog. |
| Update | Patch or minor update. Focused on what changed. |
| Milestone | Non-release achievement. Star count, contributor count, first production user, etc. |
| Announcement | General project news not tied to a release. |

---

## 8. Git Provider Support (v1)

| Provider | Support |
|---|---|
| Local git repo | Full support. Reads tags, diffs, and commit history locally. |
| GitHub | Full support. Reads tags, releases, and repo metadata via GitHub API. Optional personal access token for release body patching. |
| GitLab | Deferred to v1.1 |
| Gitea | Deferred to v1.1 |

### GitHub token

- Scope required: `repo` (or fine-grained token with release write permission on specific repos)
- Stored locally in libSQL, never transmitted to Oxhive servers
- Optional — all GitHub features except release body patching work without a token for public repos
- Token management UI in Vessel dashboard settings

---

## 9. Brand Voice Profiles

### Profile concept

A profile is a publishing identity. One Vessel installation supports multiple profiles simultaneously. Each project is mapped to a profile. When a user selects a repo, Vessel loads the correct profile automatically — no manual switching mid-flow.

Example profiles for a single user:
- Personal (personal GitHub, personal projects)
- Oxhive (organization account, Oxhive products)
- Client A (separate project identity)

### Profile schema

Each profile stores:

**Identity**
- Profile name
- Associated GitHub organizations or accounts (for automatic repo-to-profile mapping)
- Platform handles per platform (e.g. @username on X, company page URL on LinkedIn)
- Default hashtags per platform

**Brand voice settings**

Structured axes, not freeform text. Set once, applied consistently.

| Axis | Options |
|---|---|
| Formality | Casual / Balanced / Professional |
| Humor tolerance | None / Subtle / Present |
| Technical depth | Low / Medium / High |
| Self-promotion comfort | Understated / Balanced / Direct |

**Platform preferences**
- Which platforms this profile generates content for
- Per-platform enable/disable toggle

---

## 10. Vessel-Owned Storage

Vessel owns all content and brand voice data in its own libSQL instance. This data does not live in HiveMind. The two databases are separate.

### Schema (high level)

**profiles**
- id, name, formality, humor, technical_depth, self_promotion, created_at, updated_at

**profile_platforms**
- id, profile_id, platform, enabled, handles, hashtags

**projects**
- id, profile_id, repo_path, github_repo, provider (local/github), created_at

**generations**
- id, project_id, tag, category, context_notes, created_at

**generation_outputs**
- id, generation_id, platform, content, revision_number, created_at

**revision_notes**
- id, generation_id, notes, created_at

**content_feedback**
- id, generation_id, platform, signal (liked/disliked/reused), created_at

**github_tokens**
- id, project_id, token (stored encrypted), created_at

---

## 11. HiveMind Integration

The integration operates across three layers: detection, reading, and writing. It is fully passive from the user's perspective — no configuration required. If HiveMind is running, integration is active. If not, Vessel falls back to standalone silently.

---

### Layer 1 — Detection

When Vessel starts a generation, the first thing it does is a health check against HiveMind's REST API port.

```
GET http://localhost:3457/health
```

HiveMind exposes its REST API on port `3457` when started via `hivemind up`. A `200` response means HiveMind is running and integration is active. Any other result — connection refused, timeout, error — triggers silent fallback to standalone mode. Generation never blocks on this check.

```
On generation start:
    GET http://localhost:3457/health
    If 200  → HiveMind active, use integration path
    If error → HiveMind not running, use standalone path
    Never block generation on HiveMind unavailability
```

---

### Layer 2 — Reading from HiveMind

When HiveMind is detected and the user has selected a project, Vessel queries HiveMind's REST API for memories associated with that project — excluding memories Vessel itself wrote.

```
GET http://localhost:3457/api/v1/memories?project={project_id}&exclude_prefix=vessel:
```

The `exclude_prefix=vessel:` filter prevents circular injection — Vessel never reads back its own content strategy signals as if they were organic project context.

What Vessel gets back is the organic project memory HiveMind captured during real Claude Code sessions: what the project is, the tech stack, the problem it solves, past architectural decisions, the target user. None of this was entered manually. It accumulated naturally as the developer worked.

Vessel injects this context into the generation prompt before anything else. So when Claude generates a LinkedIn post for a new release, it already knows the project's stack, target user, and key differentiators — without the developer re-explaining any of it. This is the core value of the HiveMind integration.

---

### Layer 3 — Writing to HiveMind

When a user configures content strategy signals in Vessel — audience, tonality, positioning, content pillars — Vessel writes these back to HiveMind as workspace layer memories using the `vessel:` prefix.

```
POST http://localhost:3457/api/v1/memories
{
  "title": "vessel:audience",
  "content": "early-stage startup CTOs, not individual contributors",
  "project_id": "{project_id}",
  "layer": "workspace"
}
```

These live in HiveMind's workspace memory layer alongside technical memories, cleanly namespaced by prefix. In HiveMind's dashboard a developer can filter by `vessel:` to see only content strategy signals. HiveMind's own generation logic ignores them — they are not technical context.

The reason these live in HiveMind rather than only in Vessel's own database is continuity. If the developer is in a Claude Code session and asks "who is this project for?", HiveMind can surface `vessel:audience` as part of the answer. Content strategy and technical strategy live in the same memory layer, which reflects reality — they are not separate concerns.

---

### `vessel:` memory key schema (v1)

Keys are versioned. Any rename or removal requires a migration — orphaned keys are never left in HiveMind silently.

| Key | Description | Example value |
|---|---|---|
| `vessel:audience` | Who the project is for | "early-stage startup CTOs, not individual contributors" |
| `vessel:positioning` | How to frame the project's value | "emphasize reliability and auditability over speed" |
| `vessel:category` | Type of project | "CLI tool", "Rust library", "indie game", "framework" |
| `vessel:tonality` | Persistent tone characteristics for this project | "avoid humor, audience is enterprise-focused" |
| `vessel:vocabulary` | Words and phrases to use or avoid | "call it a memory layer, not a database; never say revolutionary" |
| `vessel:persona` | How the project presents itself | "developer speaking personally" or "project as entity" |
| `vessel:content-pillars` | 2-3 recurring themes to communicate around | "privacy, developer experience, simplicity" |
| `vessel:avoid` | Topics, angles, or framings to never use | "don't compare to competitor X; avoid enterprise framing" |
| `vessel:stage` | Project lifecycle stage | "early alpha", "growing OSS project", "stable v1" |
| `vessel:community` | Where the project's audience lives | "primarily Mastodon and Discord, not LinkedIn" |

Keys deferred to v1.1:

| Key | Description |
|---|---|
| `vessel:hooks` | Reusable narrative angles that have resonated with this project's audience |

**What Vessel never touches in HiveMind:**
- Technical memories (stack, architecture, preferences, past decisions)
- Personal layer memories
- Any memory not prefixed with `vessel:`

---

### The two databases and what each owns

| Concern | Owned by | Why |
|---|---|---|
| Project identity — what the project is, stack, architecture, decisions | HiveMind libSQL | Captured organically through Claude Code sessions |
| Content strategy annotations — audience, tonality, positioning | HiveMind libSQL (via `vessel:` prefix) | Belongs to project identity, accessible from Claude Code sessions |
| Content production history — past generations, revisions, feedback | Vessel libSQL | Vessel-specific operational data, not relevant to HiveMind |
| Brand voice profiles — formality, humor, platform preferences | Vessel libSQL | Publishing identity, not project identity |
| Content feedback signals — liked, reused, disliked | Vessel libSQL | Vessel generation context only |

The split is: **HiveMind = project identity and context. Vessel = content production history.**

---

### When HiveMind is not installed

Vessel generates content using only what the user has configured directly in Vessel — the brand voice profile, content history from its own database, and git context from the repo. It works, but the user must manually fill in fields like audience and positioning in Vessel's own settings rather than having them flow automatically from accumulated HiveMind memory.

This is why HiveMind integration is the premium experience — not because features are locked, but because generation quality is meaningfully higher when Vessel has access to months of organic project context rather than a few manually entered fields.

---

## 12. Oxhive Cross-Product Linking

Vessel and HiveMind link to each other contextually when both are installed and running. This is the established Oxhive product family convention — all future Oxhive products follow the same pattern.

### Behavior

Links only render after a successful health check against the target product's localhost port. A failed health check silently hides the link. No broken states, no error messages, no user-visible failures.

### HiveMind → Vessel

On any HiveMind project memory view where `vessel:` prefixed memories exist, a contextual link appears: **"View content history in Vessel"** — opens Vessel dashboard scoped to that project with project context pre-applied.

### Vessel → HiveMind

On any Vessel project content or brand voice view, a contextual link appears: **"View project memory in HiveMind"** — opens HiveMind memory view filtered to that project.

### Implementation

Deep links are localhost URLs with query parameters carrying project context.

```
HiveMind → Vessel:
localhost:3458/project?id={project_id}&source=hivemind

Vessel → HiveMind:
localhost:3457/memories?project={project_id}&source=vessel
```

No shared session. No shared auth. No embedded iframes. Just URLs with context.

---

## 13. Dashboard (Vue 3)

### Design language

Dark-first aesthetic, consistent with HiveMind. Vessel's accent color is distinct from HiveMind's teal (personal) and purple (workspace) to establish its own identity within the Oxhive family. Recommended: amber/gold — signals publishing, broadcast, attention.

### Views

**Dashboard home**
- Recent generations list with status (draft, reviewed, copied)
- Active profile indicator
- Quick action: New Post

**New Post flow**
- Category selection
- Project/repo selection
- Git tag selection (fetched from backend)
- Optional context notes input
- Generated slash command display with copy button

**Generation review**
- Per-platform content cards
- Character count indicator per platform
- Copy button per platform
- Revision notes input
- Re-generate trigger

**Projects**
- List of configured projects
- Profile mapping per project
- GitHub token status per project

**Profiles**
- Profile list
- Brand voice settings per profile
- Platform preferences per profile
- Content history per profile

**Settings**
- GitHub token management
- HiveMind integration status
- Port configuration

---

## 14. Install & Distribution

### Install experience

Single binary download. Same pattern as HiveMind. User downloads one file, runs it, dashboard opens in browser.

```bash
# Install via cargo
cargo install oxvessel

# Start Vessel
vessel up

# Dashboard opens at localhost:3458
# MCP config printed to terminal for Claude Code setup
```

### Claude Code MCP config

```json
{
  "mcpServers": {
    "vessel": {
      "command": "vessel",
      "args": ["mcp"]
    }
  }
}
```

### Config file

`vessel.toml` in user config directory.

```toml
[server]
port = 3458

[storage]
path = "~/.vessel/vessel.db"

[hivemind]
port = 3457  # Health check target
```

---

## 15. v1 Scope Boundaries

### In scope for v1

- All six output platforms with platform-aware formatting
- GitHub Releases generation and optional direct patch
- Local git repo and GitHub as input sources
- Multiple brand voice profiles with project mapping
- Tone override via structured brand voice axes
- Content history and feedback signals
- Revision loop via slash command
- HiveMind integration (read project context, write `vessel:` memories)
- Cross-product deep linking with HiveMind
- Optional GitHub personal access token for release patching

### Explicitly deferred

| Feature | Target |
|---|---|
| Direct social media posting | v2 |
| Post scheduling | v2 |
| Social media platform OAuth | v2 |
| GitLab support | v1.1 |
| Gitea support | v1.1 |
| Plugin marketplace | Post-adoption |
| HiveMind Gateway (paid tier) | Post v0.1.0 |
| Image generation for posts | v2 |
| Analytics or engagement tracking | Post-adoption |

---

## 16. Resolved Decisions

All open questions resolved. No blockers for development start.

| # | Question | Resolution |
|---|---|---|
| 1 | Pre-release drafting | Out of scope. Tag-first workflow only. |
| 2 | Vessel port convention | `3458` confirmed. |
| 3 | `vessel:` memory schema | Defined in full in section 11. Ten keys for v1, one deferred to v1.1. Migrations required for any key rename or removal. |
| 4 | Content feedback loop depth | Heavy injection by default. Content history is core to generation quality, not optional. |

### Content feedback prompt injection strategy

On every generation, Vessel assembles a content memory block from libSQL before sending to Claude:

```
1. Fetch last N generations for this project and profile
2. Fetch all feedback signals (liked, reused, disliked) for those generations
3. Summarize into a content memory block:
   - Concepts and formats the user has marked positively
   - Tones or angles explicitly disliked or unused
   - Recurring themes that have appeared in approved content
4. Inject this block into the generation prompt before the release context
```

This block is injected by default on every generation. It grows more useful over time as the user builds up a history of feedback signals.

---

## 17. Build Sequence (Recommended)

### Phase 1 — Core MCP loop
- Scaffold Rust project (`oxvessel` crate, `vessel` binary)
- libSQL schema: profiles, projects, generations, generation_outputs
- `/vessel-generate` slash command — local git only, single platform output
- Basic content generation prompt for X
- Result written to libSQL

### Phase 2 — Platform expansion
- Remaining five platform output formats
- Platform content contracts and character limit enforcement
- Brand voice profile schema and tone injection into prompts

### Phase 3 — GitHub integration
- GitHub API: tag and release listing
- GitHub Release notes generation
- Optional token management and release body patching

### Phase 4 — Dashboard
- REST API layer
- Vue 3 dashboard: generation review, profiles, projects, settings
- Slash command display and copy flow
- Revision loop UI

### Phase 5 — HiveMind integration
- HiveMind health check
- Project context read from HiveMind memories
- `vessel:` memory write to HiveMind
- Cross-product deep linking (both directions)

### Phase 6 — Polish
- `/vessel-revise` and `/vessel-status` slash commands
- Content feedback signals (liked, reused)
- GitHub token scoping UI
- Install and onboarding flow

---

*End of spec. Generated from product brainstorm session with Claude — June 2026.*
*All decisions explicitly validated by the product owner. All open questions resolved.*
*Companion product: HiveMind — github.com/oxhive/hivemind*
