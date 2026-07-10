# Agent ↔ Browser Feedback Loop Pattern

*Reference architecture for a bidirectional review loop between a CLI agent
(Claude Code or similar) and a local browser UI. Derived from
[lavish-axi](https://github.com/kunchenguid/lavish-axi); written so the pattern
can be replicated in any tool. A Rust (axum) + Vue mapping is included at the
end.*

*Last updated: July 2026*

---

## 1. The Problem

An agent running in a terminal harness (Claude Code) produces something visual
— an HTML artifact, a dashboard, generated content. The human wants to review
it in a real browser: click on elements, annotate, request changes. Then the
agent should receive that feedback **in the same conversation, with full
context intact**, apply it, and the browser should update — repeatedly, until
the human is satisfied.

The hard part is the return trip. Agent harnesses have no inbound callback
mechanism: nothing outside the conversation can inject a message into it.

## 2. The Core Trick

**The agent blocks itself.** There is no callback into the harness. Instead:

1. The agent runs a CLI command (`tool poll <key>`) as an ordinary Bash tool
   call.
2. That command long-polls a local HTTP server and does not return until the
   human sends feedback from the browser.
3. When feedback arrives, the CLI prints it as JSON to stdout and exits.
4. The harness sees a Bash tool result arrive — same conversation, same
   context. The agent reads the feedback and continues working.

From the harness's perspective, nothing special happened: a command ran for a
while and returned output. That is the entire integration surface. It works in
any agent harness that can run shell commands — no MCP required, no hooks
required, no harness-specific API.

## 3. Architecture

Three components:

```
┌────────────┐  spawns/reuses   ┌──────────────────┐   serves    ┌──────────┐
│ CLI        │ ───────────────► │ Local HTTP server │ ──────────► │ Browser  │
│ (agent-    │                  │ (127.0.0.1,       │    SSE      │ (review  │
│  facing)   │ ◄─── long-poll ──│  detached,        │ ◄── POST ── │  UI)     │
└────────────┘     stdout JSON  │  idle-timeout)    │  feedback   └──────────┘
                                └──────────────────┘
```

- **CLI** — the only thing the agent touches. Subcommands: `open`, `poll`,
  `end`, `stop`. Talks to the server over localhost HTTP.
- **Server** — detached background process bound to `127.0.0.1`. Holds session
  state, queues feedback, pushes updates to the browser via SSE. Self-terminates
  after an idle timeout (e.g. 30 min with no connected browser and no waiting
  poll) so it never leaks.
- **Browser UI** — a "chrome" page (chat panel, annotation list, controls)
  wrapping the reviewed content, usually in an iframe with a small injected SDK
  script.

### Session identity

Sessions are keyed by a **stable, human-meaningful identifier the agent already
has** — lavish-axi uses the canonical file path of the artifact. This is
load-bearing: the agent never has to store or pass around an opaque session ID.
`tool poll <same-file>` always finds the right session, even after the poll
command was killed and re-run.

## 4. The Three Legs

### Leg 1 — Agent → Browser (open)

1. Agent produces the reviewable thing (writes an HTML file, creates a record).
2. Agent runs `tool open <key>`:
   - `GET /health` on the known port; if no server, spawn one detached
     (`setsid` / equivalent), persist the port to a state file
     (`~/.tool/state.json`).
   - `POST /api/sessions` — create or reopen the session for this key.
   - Open the browser at `/session/:key`.
3. Server serves the chrome UI; reviewed content loads in an iframe via
   `/artifact/:key/...` with the annotation SDK injected server-side (string
   transform of the HTML before serving).

### Leg 2 — Browser → Agent (the return trip)

1. Agent runs `tool poll <key>` (blocking).
   - CLI issues `GET /api/poll?key=...`. Server holds the connection open.
   - To survive proxies/timeouts, the server streams whitespace heartbeats, or
     the client passes `timeoutMs` and re-polls on timeout.
2. Human annotates in the browser (click element → CSS selector; select text →
   range; type a prompt) and hits **send**.
3. Browser does `POST /api/:key/prompts` with the annotation payload.
4. Server pushes the payload onto the session's feedback queue and notifies
   waiters.
5. The blocked poll wakes, responds with the queued feedback. CLI prints JSON,
   exits. Agent continues in-conversation.

**Durability rule:** feedback is queued server-side, not delivered
point-to-point. If the poll command was killed (harness timeout, user
interrupt), re-running it drains the queue — nothing is lost. This makes the
loop robust against every harness's foreground-command limits.

### Leg 3 — Live sync while the agent works

- **File/content changes → browser:** a watcher (debounced ~100 ms) fires a
  `reload` event on the session's SSE stream (`GET /events/:key`); the browser
  refreshes the iframe. The human watches edits land live.
- **Agent chat → browser:** `POST /api/:key/agent-reply` broadcasts an
  `agent-reply` SSE event; the chrome's chat panel displays it. The poll
  command takes `--agent-reply "<msg>"` so replying and re-listening is one
  step.

### Session end

- Agent-initiated: `tool end <key>` — plain reopen allowed later.
- Human-initiated (button in browser): server marks the session ended; a
  waiting poll returns `{"session_ended": true}`; a later `tool open` **refuses
  to reopen** unless passed `--reopen`. This prevents the agent from nagging a
  human who said "done".

## 5. HTTP Surface (minimal)

| Method | Path | Purpose |
|---|---|---|
| GET | `/health` | Liveness + version (CLI uses to find/reuse server) |
| POST | `/api/sessions` | Create/reopen session, returns browser URL |
| GET | `/api/poll` | **Long-poll for feedback (blocks)** |
| POST | `/api/:key/prompts` | Browser queues feedback for agent |
| POST | `/api/:key/agent-reply` | Agent sends chat text to browser |
| POST | `/api/:key/end` | End session (browser side) |
| GET | `/events/:key` | SSE stream: `reload`, `agent-reply` |
| GET | `/session/:key` | Chrome UI |
| GET | `/artifact/:key/*` | Reviewed content (SDK injected) |
| POST | `/shutdown` | Graceful stop |

## 6. The Skill Is the Glue

The server and CLI are inert without instructions telling the agent to use
them. The final piece is an **Agent Skill** (`SKILL.md`) describing the
workflow:

1. Create the artifact.
2. `tool open <key>` — open the review session.
3. `tool poll <key> --agent-reply "<summary of what to review>"` — block for
   feedback. If the harness limits foreground command duration, run as a
   background task; if killed, re-run — the queue persists.
4. Apply feedback; poll again with `--agent-reply` to keep the loop going.
5. `tool end <key>` when done. If the human ended from the browser, do not
   reopen uninvited.

Without the skill, the agent never knows to block on poll. The skill *is* the
protocol documentation for the agent.

## 7. Rust + Vue Mapping

What each piece becomes in this stack (all dependencies already in vessel):

| Pattern component | Rust/Vue implementation |
|---|---|
| HTTP server | `axum`, bound to `127.0.0.1` |
| CLI subcommands | `clap` derive (`Open`, `Poll`, `End`, `Stop`) |
| Session store | `Arc<RwLock<HashMap<Key, Arc<Session>>>>` |
| Feedback queue + wakeup | `Mutex<VecDeque<Feedback>>` + `tokio::sync::Notify` |
| Long-poll blocking | `tokio::select!` over `notify.notified()` and a timeout |
| SSE | `axum::response::sse` fed by a `tokio::sync::broadcast` channel per session |
| File watcher | `notify` crate, debounced |
| Chrome UI | Vue SPA embedded via `rust-embed` with SPA fallback |
| Browser → server | `fetch` POST; server → browser: `EventSource` |
| Iframe ↔ chrome | `postMessage` from injected SDK script to parent Vue app |
| Detached server spawn | CLI re-execs itself with a `serve` subcommand, detached |

### Key Rust sketches

Session and long-poll:

```rust
struct Session {
    feedback: Mutex<VecDeque<Feedback>>,
    feedback_notify: Notify,
    sse_tx: broadcast::Sender<SseMsg>, // "reload" | "agent-reply"
    ended: AtomicBool,
}

async fn poll(State(store): State<Store>, Query(q): Query<PollQuery>) -> Response {
    let session = store.get(&session_key(&q.key))?;
    let timeout = Duration::from_millis(q.timeout_ms.unwrap_or(600_000));
    loop {
        if let Some(fb) = session.feedback.lock().await.pop_front() {
            return Json(fb).into_response();
        }
        if session.ended.load(Ordering::Relaxed) {
            return Json(json!({ "session_ended": true })).into_response();
        }
        tokio::select! {
            _ = session.feedback_notify.notified() => continue,
            _ = tokio::time::sleep(timeout) =>
                return Json(json!({ "timeout": true })).into_response(),
        }
    }
}

async fn submit_prompt(Path(key): Path<String>, State(store): State<Store>,
                       Json(fb): Json<Feedback>) {
    let s = store.get(&key).unwrap();
    s.feedback.lock().await.push_back(fb);
    s.feedback_notify.notify_waiters();
}
```

SSE endpoint:

```rust
async fn events(Path(key): Path<String>, State(store): State<Store>)
    -> Sse<impl Stream<Item = Result<Event, Infallible>>>
{
    let rx = store.get(&key).unwrap().sse_tx.subscribe();
    Sse::new(BroadcastStream::new(rx).map(|msg| Ok(Event::default()
        .event(msg.kind)
        .data(msg.payload))))
        .keep_alive(KeepAlive::default())
}
```

Vue chrome essentials:

```js
// Live updates
const es = new EventSource(`/events/${key}`)
es.addEventListener('reload', () => { iframe.src = `${base}?v=${Date.now()}` })
es.addEventListener('agent-reply', e => chat.push(JSON.parse(e.data)))

// Annotations from the iframe SDK arrive via postMessage
window.addEventListener('message', e => {
  if (e.data?.type === 'annotation') pending.push(e.data) // selector, text range
})

// Send wakes the agent's blocked poll
await fetch(`/api/${key}/prompts`, {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({ prompts: pending }),
})
```

## 8. CLI vs MCP Transport

The return trip can ride either transport; the blocking semantics are
identical.

- **CLI (`tool poll`)** — portable to any harness that can run shell commands;
  zero install via `npx`-style invocation; survives harness restarts because
  state lives in the server. lavish-axi chose this deliberately.
- **MCP tool (`poll_feedback`)** — cleaner inside Claude Code (typed tool
  result instead of stdout parsing), natural fit if the tool already ships an
  MCP server (vessel does, via `rmcp`). Costs portability: only harnesses with
  the MCP server configured can join the loop.

Shipping both is cheap: both are thin clients over the same `GET /api/poll`.

## 9. Design Rules Worth Keeping

1. **Bind loopback only.** The server is a local trust boundary; never listen
   on `0.0.0.0`.
2. **Queue, don't deliver.** Feedback persists server-side until a poll drains
   it. Killed polls lose nothing.
3. **Key sessions by something the agent already knows** (file path, record
   ID). No opaque handles.
4. **Idle self-termination.** Detached servers must die on their own.
5. **Respect the human's "end".** Browser-side end blocks silent reopen.
6. **Heartbeat long polls.** Stream whitespace or use client timeouts +
   re-poll; assume something will kill an idle connection.
7. **The skill is part of the product.** Undocumented protocol = agent never
   uses it.
