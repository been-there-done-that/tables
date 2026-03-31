# AI Universal Copilot — Design Spec
**Date:** 2026-03-27
**Status:** Approved for implementation

---

## 1. Goal

Transform the AI chat panel from a plain conversational assistant into a Universal Copilot — an autonomous agent that can progressively explore schema, profile data, run readonly queries, and deliver complete answers, all without leaving the app.

### What changes
- The agent gets 14 tool calls that bridge from Claude → Bun harness → Tauri IPC → Rust DB drivers
- Each tool call shows elapsed time in the chat (live timer while running, frozen when done)
- SQL-producing tools show a "Run" button that opens the query in the main editor
- The harness gains an MCP server and a new `/tool-result` HTTP endpoint
- The frontend gains a tool executor that dispatches tool calls to the right Tauri commands

### What does NOT change (v1)
- No writes. No INSERT, UPDATE, DELETE, DDL. Read-only only.
- No inline data grids in chat. Results stay in the normal editor/results panel.
- No multi-connection tool calls. Agent operates on the active connection only.

---

## 2. Architecture

### Overview

```
User types in Agent Panel
  → POST /session/send → Bun Harness (SSE stream back)
      → Claude CLI subprocess (via claude-agent-sdk)
          → decides to call a tool
          → calls MCP server tool handler (in harness)
              → harness emits tool.started SSE event
              → harness stores Promise<result> keyed by toolCallId
                ← frontend receives tool.started
                ← frontend dispatches Tauri IPC command
                ← frontend POSTs result to POST /tool-result/:sessionId/:toolCallId
              → harness resolves Promise
          → MCP handler returns result to Claude CLI
      → Claude sees data, reasons, calls more tools or writes answer
  → text.delta / tool.started / tool.completed / turn.done SSE events stream to frontend
```

### Key principle
The harness never touches the database. It is a pure bridge. The frontend owns all Tauri IPC calls. This means **zero new Rust code** is needed.

### New harness endpoints
| Endpoint | Purpose |
|---|---|
| `POST /session/start` | existing — create session |
| `POST /session/send` | existing — send message, returns SSE |
| `POST /session/stop` | existing — teardown |
| `POST /tool-result/:sessionId/:toolCallId` | **new** — frontend submits tool execution result |

### MCP server (new, inside harness)
- Spun up once per harness process, shared across sessions
- Defines all 14 tool schemas with JSON Schema input validation
- Each tool handler: stores a deferred Promise, emits `tool.started` SSE, awaits resolution
- Resolution arrives via `POST /tool-result` → session looks up Promise by toolCallId → resolves

---

## 3. Tool Definitions

All tools are **read-only**. The frontend enforces this — it only calls read-safe Tauri commands.

### Schema Exploration

| Tool | Input | Tauri command | Returns |
|---|---|---|---|
| `list_tables` | `schema?: string` | `get_tables_in_schema` | Array of `{ table_name, table_type, column_count }` |
| `describe_table` | `table: string, schema?: string` | `get_schema_table_details` | Columns, types, nullable, PKs, defaults |
| `get_indexes` | `table: string, schema?: string` | `get_schema_table_details` | Index name, columns, unique, type |
| `get_foreign_keys` | `table: string, schema?: string` | `get_schema_table_details` | FK name, column, referenced table/column |

### Query Execution

| Tool | Input | Tauri command | Returns |
|---|---|---|---|
| `run_query` | `sql: string, limit?: number (default 100)` | `execute_query` | `{ columns, rows, row_count, truncated }` |
| `sample_table` | `table: string, schema?: string, n?: number (default 20)` | `execute_query` | Random N rows via `TABLESAMPLE` or `ORDER BY random()` |
| `count_rows` | `table: string, schema?: string, where?: string` | `execute_query` | `{ count: number }` |
| `explain_query` | `sql: string, analyze?: boolean (default false)` | `execute_query` | EXPLAIN plan text |

### Data Profiling

| Tool | Input | Tauri command | Returns |
|---|---|---|---|
| `column_stats` | `table: string, column: string, schema?: string` | `execute_query` | `{ null_pct, distinct_count, min, max, avg }` |
| `find_nulls` | `table: string, schema?: string` | `execute_query` (per column) | `{ column, null_count, null_pct }[]` |
| `get_distinct_values` | `table: string, column: string, schema?: string, limit?: number (default 20)` | `execute_query` | `{ value, count }[]` ordered by count desc |
| `check_fk_integrity` | `table: string, schema?: string` | `execute_query` (per FK) | `{ fk_name, orphan_count }[]` |

### Editor Integration

| Tool | Input | Tauri command | Returns |
|---|---|---|---|
| `open_in_editor` | `sql: string, title?: string` | frontend-only: `session.openView()` | `{ success: true }` |
| `get_query_history` | `limit?: number (default 20)` | `fetch_query_logs` | `{ sql, executed_at, duration_ms }[]` |

---

## 4. Harness Changes

### `packages/harness/src/mcp-server.ts` (new file)

- Creates an MCP server using `@modelcontextprotocol/sdk`
- Registers all 14 tool schemas
- Exposes a `pendingTools: Map<string, { resolve, reject }>` that sessions can write to
- Each tool handler:
  1. Generates a `toolCallId`
  2. Emits the `tool.started` event via the active session's `emitFn`
  3. Stores `{ resolve, reject }` in `pendingTools` keyed by `toolCallId`
  4. Returns `await promise` to the MCP caller (Claude CLI blocks here)
  5. Rejects with timeout after 30 seconds

### `packages/harness/src/session.ts` (modified)

- Constructor receives `mcpServer` instance, passes it to `query()` via `options.mcpServers`
- New method: `submitToolResult(toolCallId: string, result: unknown)` — resolves the pending Promise
- New method: `failToolResult(toolCallId: string, error: string)` — rejects the pending Promise

### `packages/harness/src/index.ts` (modified)

- New endpoint: `POST /tool-result/:sessionId/:toolCallId`
  - Looks up session by `sessionId`
  - Calls `session.submitToolResult(toolCallId, body)`
  - Returns `{ ok: true }`

---

## 5. Frontend Changes

### `src/lib/agent/tool-executor.ts` (new file)

Single responsibility: receive a `tool.started` event, execute it, POST the result back.

```
dispatch(event: ToolStartedEvent, context: ToolContext): Promise<void>
  - context = { connectionId, database, schema, port, sessionId }
  - switch on event.toolName → call appropriate Tauri command
  - POST result to /tool-result/:sessionId/:toolCallId
  - on error → POST error payload so agent gets useful feedback
```

Tool context is derived from `schemaStore` at dispatch time. This means tools always run against whatever connection is active when the agent is responding — no stale context.

### `src/lib/components/agent/AgentPanel.svelte` (modified)

- Imports `toolExecutor.dispatch()`
- On `tool.started` SSE event: call `toolExecutor.dispatch()` (fire-and-forget, errors handled inside)

### `src/lib/components/agent/ToolCallCard.svelte` (modified)

**Elapsed time display:**
- On `tool.started`: record `startedAt = Date.now()`; start a `setInterval` that updates a displayed `elapsed` string every 100ms
- On `tool.completed`: clear interval, freeze elapsed display
- Format: `< 1s` → show ms (e.g. `82ms`), `≥ 1s` → show 1 decimal (e.g. `1.4s`)
- Live timer shown in accent color; frozen timer shown in muted/tertiary color

**Run button:**
- Show only on tools: `run_query`, `sample_table`, `count_rows`, `explain_query`
- On click: `session.openView("editor", toolName, { content: sql })` where `sql` comes from the tool's input

**Card states:**
- Running: `IconLoader2` (tabler, spinning via CSS `animate-spin`), live elapsed timer (accent)
- Done: `IconCheck` (tabler, green), frozen time (muted), Run button if applicable
- Error: `IconX` (tabler, red), frozen time (muted), error message summary

**Expandable result:**
- All tool cards are collapsible/expandable — collapsed by default after completion
- Clicking the card header toggles an expanded section showing the raw result JSON
- Expanded section has a fixed max-height (e.g. 200px) with `overflow-y: auto` scroll
- Uses `IconChevronDown` / `IconChevronUp` (tabler) to indicate toggle state

### Turn-level elapsed time

- `AgentPanel` records `turnStartedAt = Date.now()` when user sends a message
- Displays a running total in the composer area while streaming
- On `turn.done`: freezes and shows total (e.g. "turn complete · 8.3s · 3 tools")

---

## 6. SSE Event Protocol Changes

No breaking changes. Two events get richer payloads:

**`tool.started`** (existing, extended):
```ts
{
  type: "tool.started",
  toolCallId: string,   // NEW — needed for /tool-result routing
  toolName: string,
  input: Record<string, unknown>
}
```

**`tool.completed`** (existing, extended):
```ts
{
  type: "tool.completed",
  toolCallId: string,   // NEW
  output: string        // JSON-stringified result summary (for card display)
}
```

---

## 7. System Prompt Update

The system prompt (built in `src/lib/agent/tools.ts`) gets a new tools section explaining what the agent can do:

```
You have tools to explore and query the connected database. Use them proactively:
- Start with list_tables to understand what's available
- Use describe_table before writing queries to know exact column names/types
- Use run_query for any SELECT — you will see the results
- Use sample_table to quickly understand data shape before deeper analysis
- chain tools freely — e.g. list_tables → describe_table → run_query → column_stats

All queries run against: <connection_name> / <database> / <schema>
Current schema is also provided in full below for reference.
```

---

## 8. Build & Deployment

- After harness changes: `cd packages/harness && bun run build` → rebuilds binary
- No new Rust code, no `cargo build` needed
- `@modelcontextprotocol/sdk` is already a transitive dependency of `@anthropic-ai/claude-agent-sdk` — no new package installs needed

---

## 9. Out of Scope (future phases)

- **Phase B**: Writes with confirmation — INSERT, UPDATE, DELETE shown as "pending" tool cards requiring user approval before execution
- **Phase C**: Full DDL — CREATE INDEX, ALTER TABLE with diff-style preview
- **Multi-connection**: `run_query_on_connection` tool with explicit connection parameter
- **Saved snippets**: agent can `save_snippet(sql, name)` to a personal library
- **Chart generation**: agent returns Vega-Lite spec, renders inline sparkline
- **Session history**: persist and reload past agent conversations per connection
