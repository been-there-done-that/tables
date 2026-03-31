# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What This Is

**Tables** is a cross-platform desktop database management application (like DataGrip) built with Tauri 2 + SvelteKit. It provides a unified SQL editor and schema explorer for PostgreSQL, SQLite, MySQL, MongoDB, and Redis.

## Commands

### Development
```bash
pnpm tauri dev        # Full dev mode (Vite + Rust hot reload)
pnpm dev              # Frontend only (Vite on http://localhost:1420)
```

### Build
```bash
pnpm tauri build      # Production desktop app bundle
pnpm build            # Frontend build only
```

### Type Checking
```bash
pnpm check            # svelte-check (frontend types)
pnpm check:watch      # Watch mode
```

### Rust Tests
```bash
cd src-tauri && cargo test                    # All backend tests
cd src-tauri && cargo test <test_name>        # Single test
cd src-tauri && cargo test -- --nocapture     # With stdout
```

### Lint / Format
```bash
cd src-tauri && cargo clippy                  # Rust linting
cd src-tauri && cargo fmt                     # Rust formatting
```

## Architecture

### Layers

```
Frontend (SvelteKit/Svelte5)
    ↕ Tauri IPC (invoke / emit)
Backend (Rust/Tokio)
    ↕ DatabaseAdapter trait
Database Drivers (tokio-postgres, rusqlite, mysql, mongodb, redis)
```

### Backend (`src-tauri/src/`)

**Core abstractions:**
- `adapter.rs` — `DatabaseAdapter` trait: the DCI (Database Capability Interface) all engines implement. `DatabaseCapabilities` describes per-engine feature flags. `EngineProfile` enum classifies engines by hierarchy depth (DB0–DB3).
- `adapter_registry.rs` — Creates adapters by connection type.
- `connection_manager.rs` — Caches live adapters per connection ID using double-checked locking. Central hub for query execution and connection lifecycle.
- `orchestrator.rs` — Coordinates multi-step operations (e.g., schema introspection + query).
- `introspection.rs` — Per-engine schema introspection (tables, columns, views, indexes, foreign keys).

**Adapters** (`src-tauri/src/adapters/`):
- `postgres.rs` — PostgreSQL with streaming results, query cancellation, connection pooling, server-side cursors.
- `sqlite.rs` — SQLite adapter.

**Completion engine** (`src-tauri/src/completion/`):
- Uses `tree-sitter-sql` to parse SQL ASTs.
- Diagnostics: duplicate columns, unknown CTEs (with proper CTE scope tracking), parse errors.
- `schema_graph.rs` — petgraph-based scope tracker to prevent false positives.

**Commands** (`src-tauri/src/commands/`):
- Each file exposes Tauri `#[command]` functions. Key modules: `query_commands`, `connection_commands`, `introspection_commands`, `completion_commands`, `editor_commands`, `agent_commands`.

**Security:**
- `credential_manager.rs` + `crypto.rs` — Passwords stored via OS keyring + AES-GCM encryption. Never in plaintext.
- `credentials.rs` — `SecureCredentials` type enforces separation from connection metadata.

**Local storage:**
- SQLite at `~/.config/tables/tables.db` (managed via `migrations.rs`). WAL mode, foreign keys enforced.

### Frontend (`src/`)

**State management** — Svelte 5 runes (`.svelte.ts` files):
- `stores/session.svelte.ts` — Active database session, current view, active editor.
- `stores/schema.svelte.ts` — Cached schema per connection.
- `stores/settings.svelte.ts` — User preferences.
- `stores/window.svelte.ts` — Layout/panel state.

**Key components** (`src/lib/components/`):
- `SqlTestingEditor.svelte` — Monaco editor with SQL syntax + diagnostics.
- `ExplorerContainer.svelte` — Tree explorer for databases/schemas/tables.
- `TablePreview.svelte` — Virtualized data grid.
- `EditorTabs.svelte` — Multi-tab query management.
- `AiAssistantPanel.svelte` + `AgentChat.svelte` — AI agent integration.

**Agent system** (`src/lib/agent/`):
- `agent.svelte.ts` — Agent state.
- `tools.ts` — Tool definitions.
- `tool_runner.ts` — Executes tool calls from AI responses.

**Routes:**
- `/` — Main interface (resizable 3-pane layout: explorer | editor | logs/results).
- `/datasource` — Connection management.
- `/settings` — Theme, editor, AI preferences.

### IPC Pattern

Frontend calls `invoke('command_name', args)` → Tauri dispatches to Rust `#[command]` function → returns serialized result. Long-running queries use Tauri events (`emit`) for streaming.

### Adding a New Database Engine

1. Implement `DatabaseAdapter` trait in `src-tauri/src/adapters/<engine>.rs`.
2. Register in `adapter_registry.rs`.
3. Add `DatabaseCapabilities` describing supported features.
4. Add introspection logic in `introspection.rs`.
5. Add connection form fields in the frontend datasource UI.
