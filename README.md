# Vessel

Developer release announcement tool. Vessel reads your local git (or GitHub) release context, assembles it into a prompt for Claude, and lets Claude generate platform-tailored announcement copy — which you then save back to Vessel for review in the dashboard.

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

This registers the `vessel mcp` server automatically — the `vessel` binary just needs to be on your `PATH` (see Installation below).

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

| Prompt | Arguments | Description |
|---|---|---|
| `/vessel-generate` | `repo_path` (optional, defaults to cwd), `tag` (optional, defaults to latest git tag), `category` (optional, defaults to `release`), `context_notes` (optional) | Assembles git/GitHub context, brand voice profile, and past feedback into a prompt. Claude generates content for all 6 platforms, then calls the `vessel_save` tool with the results. |
| `/vessel-status` | none | Shows recent generations per project and a link to the dashboard. |
| `/vessel-revise` | `generation_id` (required), `notes` (required) | Returns the current content for a generation plus revision notes, instructing Claude to revise and call `vessel_save` again. |
| `/vessel-profile` | none | Lists configured brand voice profiles (formality, humor, technical depth, self-promotion). |

## MCP tools

| Tool | Input | Description |
|---|---|---|
| `vessel_save` | `generation_id: string`, `outputs: [{ platform: string, content: string }]` | Persists Claude-generated platform content to local storage. Called after a `/vessel-generate` or `/vessel-revise` prompt. |

## Platforms

`twitter` (280 chars), `linkedin` (3000 chars), `bluesky` (300 chars), `mastodon` (500 chars), `discord` (no limit), `github_release` (no limit).

## Storage

libSQL database at `~/.vessel/vessel.db` by default. Config file at `~/.config/vessel/vessel.toml`.
