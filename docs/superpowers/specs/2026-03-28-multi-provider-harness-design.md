# Multi-Provider Harness Design

**Date:** 2026-03-28
**Status:** Approved for implementation

---

## 1. Goal

Extend the Bun HTTP harness (`packages/harness/`) from a Claude-only bridge into a provider-agnostic session layer. Users bring their own installed AI agent tools — Claude CLI, Codex CLI, Gemini CLI, OpenCode server, Cursor — and the harness routes to whichever one is requested. No new subscriptions, no API keys managed by the app: the harness reuses what the user already has.

**Harness-only.** No frontend UI changes, no Rust changes, no new Tauri commands. The HTTP API surface stays backwards-compatible: callers that don't send a `provider` field continue to get Claude.

---

## 2. Architecture Overview

### Integration patterns

There are four distinct ways agent tools expose a programmatic interface:

| Pattern | Providers | How |
|---|---|---|
| **SDK** | Claude, Gemini | TypeScript SDK wraps the CLI subprocess internally (`@anthropic-ai/claude-agent-sdk`, `@google/gemini-cli-sdk`). SDK owns process lifecycle. |
| **JSON-RPC stdio** | Codex | Spawn `codex app-server`, speak JSON-RPC 2.0 over stdin/stdout pipes. Bidirectional: server sends notifications, requests, and responses. |
| **HTTP/SSE** | OpenCode | Run `opencode serve`, talk to it via `@opencode-ai/sdk` REST client. SSE stream for events. |
| **ACP** | Cursor, Windsurf | `@agentclientprotocol/sdk` `ClientSideConnection` — open standard co-developed by JetBrains and Zed. Agent must already be running. |

### File structure (after)

```
packages/harness/src/
  index.ts                        modified: reads `provider` field, calls registry
  types.ts                        NEW: Session interface, HarnessEvent, SessionConfig
  registry.ts                     NEW: maps provider name → factory fn
  adapters/
    sdk-adapter.ts                NEW: base for SDK-based providers (Claude, Gemini)
    jsonrpc-adapter.ts            NEW: base for JSON-RPC stdio providers (Codex)
    http-adapter.ts               NEW: base for HTTP/SSE server providers (OpenCode)
    acp-adapter.ts                NEW: base for ACP providers (Cursor, Windsurf)
  providers/
    claude.ts                     existing session.ts logic, moved here
    codex.ts                      NEW: extends JsonRpcAdapter
    gemini.ts                     NEW: extends SdkAdapter
    opencode.ts                   NEW: extends HttpAdapter
    cursor.ts                     NEW: extends AcpAdapter
```

`session.ts` is deleted after its logic moves to `providers/claude.ts`.

---

## 3. Core Types (`types.ts`)

### `HarnessEvent`

Unchanged from current `session.ts`. Already provider-agnostic.

```ts
export type HarnessEvent =
  | { type: "text.delta"; content: string }
  | { type: "thinking.delta"; content: string }
  | { type: "tool.started"; toolId: string; toolName: string; input: unknown }
  | { type: "tool.completed"; toolId: string; output: string }
  | { type: "tool.input_delta"; toolId: string; toolName: string; partialContent: string }
  | { type: "session.init"; sdkSessionId: string }
  | { type: "turn.done" }
  | { type: "error"; message: string };
```

### `Session` interface

```ts
export interface Session {
  setEmit(fn: (e: HarnessEvent) => void): void;
  emitToolEvent(e: HarnessEvent): void;
  send(text: string): void;
  stop(): void;
}
```

### `SessionConfig`

What `/session/start` and `/session/resume` pass to a provider factory:

```ts
export interface SessionConfig {
  sessionId: string;
  threadId: string;
  systemPrompt: string;
  provider?: string;                        // default "claude"
  providerConfig?: Record<string, unknown>; // provider-specific extras
  model?: string;
  effort?: "auto" | "low" | "medium" | "high" | "max";
  sdkSessionId?: string;                    // resume only
}
```

`providerConfig` is the escape hatch for per-provider settings:
- OpenCode: `{ serverUrl: "http://localhost:4096" }`
- Cursor ACP: `{ acpEndpoint: "ws://localhost:..." }`
- SDK providers: omit entirely

---

## 4. Registry (`registry.ts`)

```ts
import type { Session, SessionConfig } from "./types";
import { ClaudeProvider } from "./providers/claude";
import { CodexProvider } from "./providers/codex";
import { GeminiProvider } from "./providers/gemini";
import { OpenCodeProvider } from "./providers/opencode";
import { CursorProvider } from "./providers/cursor";

type ProviderFactory = (config: SessionConfig) => Session;

const PROVIDERS: Record<string, ProviderFactory> = {
  claude:   (c) => new ClaudeProvider(c),
  codex:    (c) => new CodexProvider(c),
  gemini:   (c) => new GeminiProvider(c),
  opencode: (c) => new OpenCodeProvider(c),
  cursor:   (c) => new CursorProvider(c),
};

export function createSession(config: SessionConfig): Session {
  const name = config.provider ?? "claude";
  const factory = PROVIDERS[name];
  if (!factory) throw new Error(`Unknown provider: "${name}"`);
  return factory(config);
}
```

---

## 5. Adapter Bases

### `SdkAdapter` (Claude, Gemini)

Thin abstract class. Handles the `emitFn` and `emitToolEvent` bookkeeping shared by SDK-based providers. Each provider subclass calls `super()`, sets up its SDK stream in the constructor, and calls `this.consume(stream)`.

Key responsibilities:
- Store and expose `setEmit` / `emitToolEvent`
- `consume(stream)` loop: iterate SDK async iterable, parse events, call `this.emitFn`
- Event normalisation: `stream_event`, `assistant`, `user`, `system`, `result` → `HarnessEvent`
- `stop()`: abort controller

The bulk of current `session.ts` becomes `SdkAdapter`. `ClaudeProvider` adds only Claude-specific SDK options (`pathToClaudeCodeExecutable`, `permissionMode`, `effort`, `persistSession`, `resume`).

### `JsonRpcAdapter` (Codex)

Handles the stdio JSON-RPC 2.0 protocol extracted from omnicode's `CodexAdapter`:

- `spawn(codexBinaryPath, ["app-server"], { stdio: ["pipe","pipe","pipe"] })`
- `readline` on stdout → parse JSONL → route to:
  - `handleNotification(method, params)` — server push events
  - `handleRequest(id, method, params)` → response via stdin
  - `handleResponse(id, result|error)` → resolve pending promise
- `sendRequest(method, params, timeoutMs=20_000)` → returns `Promise<TResponse>`
- Initialization sequence: `initialize` → `initialized` → `account/read` → `thread/start`
- Event normalisation: Codex notification methods → `HarnessEvent`

Key Codex notification → HarnessEvent mappings:
| Codex method | HarnessEvent |
|---|---|
| `item/agentMessage/delta` | `text.delta` |
| `item/thinkingMessage/delta` | `thinking.delta` |
| `item/toolCall/start` | `tool.started` |
| `item/toolCall/complete` | `tool.completed` |
| `thread/turn/complete` | `turn.done` |
| `session/ready` | `session.init` |

`stop()`: send `thread/stop` RPC, then `child.kill()`.

### `HttpAdapter` (OpenCode)

Uses `@opencode-ai/sdk` client:

- Constructor: create `OpenCodeClient({ baseUrl: providerConfig.serverUrl ?? "http://localhost:4096" })`
- `send(text)`: `client.session.message.create(sessionId, { parts: [{ type: "text", text }] })` then open SSE stream
- SSE event normalisation: OpenCode `message.part.updated` → `text.delta`; tool events → `tool.started/completed`; turn end → `turn.done`
- `stop()`: close SSE stream, `client.session.delete(sessionId)`

OpenCode manages its own session state. The harness `sessionId` maps to an OpenCode session ID created on first `send()`.

### `AcpAdapter` (Cursor, Windsurf)

Uses `@agentclientprotocol/sdk` `ClientSideConnection`:

- Constructor: `new ClientSideConnection({ endpoint: providerConfig.acpEndpoint })`
- `send(text)`: `connection.sendMessage({ role: "user", content: text })`
- Event stream: listen on `connection.on("message")` → normalise to `HarnessEvent`
- Tool calls: ACP tool events map to `tool.started/completed`; if agent uses shell tools they still hit the `/db/` bridge as normal
- `stop()`: `connection.close()`

**Prerequisite:** The ACP agent (Cursor, Windsurf) must already be running and listening at `acpEndpoint`. The harness does not launch IDE processes.

---

## 6. `index.ts` Changes

Only three lines change in the session-start handler:

```ts
// Before:
sessions.set(sessionId, new ClaudeSession(systemPrompt, claudePath, model, effort, cwd));

// After:
import { createSession } from "./registry";
// ...
sessions.set(sessionId, createSession({ sessionId, threadId, systemPrompt, provider, providerConfig, model, effort }));
```

All endpoints (`/session/send`, `/db/`, `/tool-result/`, `/session/stop`, `/session/resume`) are unchanged. The `sessions: Map<string, Session>` type changes from `ClaudeSession` to `Session`.

---

## 7. Tool Bridge Compatibility

The `/db/:sessionId/:toolName` HTTP bridge works for all providers that give the agent shell/Bash access. The agent curls `http://localhost:{HARNESS_PORT}/db/{sessionId}/{toolName}` — this works identically regardless of whether Claude, Codex, or Gemini called it.

ACP (Cursor) may handle tool calls differently depending on ACP's tool protocol. The `CursorProvider` will emit `tool.started` events from ACP tool notifications and expect the frontend to POST results to `/tool-result/` as normal. If ACP's tool calling is incompatible with the curl-based bridge, a follow-up phase will add native ACP tool result injection.

---

## 8. Backward Compatibility

- `/session/start` without `provider` → defaults to `"claude"` → identical to current behaviour
- `ClaudeProvider` constructor signature is identical to current `ClaudeSession`
- No frontend changes required
- `HARNESS_PORT` output format unchanged

---

## 9. Dependencies to Add

```json
{
  "@opencode-ai/sdk": "latest",
  "@agentclientprotocol/sdk": "latest"
}
```

`@google/gemini-cli-sdk` added when `GeminiProvider` is implemented (may not be on npm yet — fallback: spawn `gemini` CLI with headless flags and parse stdout).

Codex and Claude: no new dependencies (Codex uses built-in `child_process`/`readline`; Claude uses existing `@anthropic-ai/claude-agent-sdk`).

---

## 10. Out of Scope

- **Provider auto-detection** (`GET /providers`) — follow-on once providers work
- **Frontend provider picker UI** — separate task
- **Windsurf provider** — added once ACP adapter is proven with Cursor
- **Write tools across providers** — the agent tool bridge is read-only today per the AI copilot spec
- **Multi-provider sessions** — one provider per session, always
