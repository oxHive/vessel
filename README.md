# Vessel

> 🚧 **Under active development.** APIs and config formats may change between releases.

Developer release announcement tool. Vessel reads your local git (or GitHub) release context, assembles it into a prompt for Claude, and lets Claude generate platform-tailored announcement copy — which you then review, revise, and approve in a local dashboard without leaving the conversation.

## Getting started

### 1. Install

```
cargo install oxvessel
```

This puts the `vessel` binary on your `PATH` (published to crates.io on every tagged release).

### 2. Hook it into Claude Code

```
/plugin marketplace add oxHive/vessel
/plugin install vessel@vessel
```

The plugin registers the `vessel mcp` server for you. (Manual MCP config below if you're not using the plugin.)

### 3. Generate your first announcement

From a repo that has at least one git tag, in Claude Code:

```
/vessel-generate
```

Claude assembles your release context (commits, tag, GitHub release if configured), generates copy for all six platforms, and saves it with the `vessel_save` tool.

### 4. Review in the browser, revise from the browser

After saving, Claude calls `vessel_poll_feedback` and waits. The dashboard opens at `http://localhost:3458` (auto-started if not already running):

- Type a revision note on any platform card — or one note for all platforms — and hit **Send**. Claude receives it in the same conversation, revises, and the card updates live.
- Claude's one-line status replies appear above the cards.
- Click **Done reviewing** when satisfied. Claude stops polling and wraps up.

Feedback is queued in the local database, so nothing is lost if a poll times out or the session restarts.

### 5. Ship it

Copy each platform's final text from the dashboard with one click. Optionally tune your brand voice (formality, humor, technical depth, self-promotion) on the **Profiles** page — future generations use it.

## CLI

```
vessel up     # start the REST API + dashboard on http://localhost:3458
vessel mcp    # start the MCP server over stdio (for Claude Code / MCP clients)
```

## MCP setup

### Option 1: Claude Code plugin (recommended)

Vessel ships as a Claude Code plugin, so the MCP server is installed and configured for you.

```
/plugin marketplace add oxHive/vessel
/plugin install vessel@vessel
```

This registers the `vessel mcp` server automatically — the `vessel` binary just needs to be on your `PATH` (see Getting started above).

### Option 2: Manual MCP config

Add Vessel as an MCP server directly:

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

## MCP prompts

| Prompt             | Arguments                                                                                                                                                       | Description                                                                                                                                                                           |
| ------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `/vessel-generate` | `repo_path` (optional, defaults to cwd), `tag` (optional, defaults to latest git tag), `category` (optional, defaults to `release`), `context_notes` (optional) | Assembles git/GitHub context, brand voice profile, and past feedback into a prompt. Claude generates content for all 6 platforms, then calls the `vessel_save` tool with the results. |
| `/vessel-status`   | none                                                                                                                                                            | Shows recent generations per project and a link to the dashboard.                                                                                                                     |
| `/vessel-revise`   | `generation_id` (required), `notes` (required)                                                                                                                  | Returns the current content for a generation plus revision notes, instructing Claude to revise and call `vessel_save` again.                                                          |
| `/vessel-profile`  | none                                                                                                                                                            | Lists configured brand voice profiles (formality, humor, technical depth, self-promotion).                                                                                            |

## MCP tools

| Tool                   | Input                                                                        | Description                                                                                                                                                                                                                       |
| ---------------------- | ---------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `vessel_save`          | `generation_id: string`, `outputs: [{ platform: string, content: string }]`  | Persists Claude-generated platform content to local storage. Called after a `/vessel-generate` or `/vessel-revise` prompt. Notifies the dashboard so open review pages refresh live.                                              |
| `vessel_poll_feedback` | `generation_id: string`, `agent_reply: string` (optional)                    | Blocks until the user sends revision notes from the dashboard, clicks **Done reviewing**, or ~10 minutes pass (`timeout: true` — call again to keep waiting; queued feedback is never lost). `agent_reply` shows a one-line status in the browser. Auto-starts `vessel up` and opens the review page if the dashboard isn't running. |

## The review loop

```
Claude Code                     vessel up (localhost:3458)         Browser
───────────                     ──────────────────────────         ───────
vessel_save ─────────────────►  store outputs ──── SSE ──────────► cards refresh live
vessel_poll_feedback ────────►  long-poll, blocks
     │                            ◄── revision notes ◄──────────── user types note, Send
     ◄── notes returned ───────  queue drained
revise + vessel_save ────────►  store ──────────── SSE ──────────► updated content appears
     ◄── session_ended ────────  ◄── Done reviewing ◄────────────── review finished
```

The architecture behind this (long-poll return trip, durable queue, SSE push) is documented tool-agnostically in [`docs/AGENT_BROWSER_FEEDBACK_LOOP.md`](docs/AGENT_BROWSER_FEEDBACK_LOOP.md) if you want to replicate it elsewhere.

## Platforms

`twitter` (280 chars), `linkedin` (3000 chars), `bluesky` (300 chars), `mastodon` (500 chars), `discord` (no limit), `github_release` (no limit).

## Storage

libSQL database in the platform data directory by default — `$XDG_DATA_HOME/vessel/vessel.db` (`~/.local/share/vessel/vessel.db`) on Linux, `~/Library/Application Support/vessel/vessel.db` on macOS. Installs upgraded from older versions keep using the legacy `~/.vessel/vessel.db` if it exists (a startup log points to the new location). Override either way with `storage.path` in the config file.

Config file at `~/.config/vessel/vessel.toml` (platform config directory: `$XDG_CONFIG_HOME` on Linux, `~/Library/Application Support` on macOS).
