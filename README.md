<div align="center">

# Tables

**A fast, native database client for modern developers.**

Connect to PostgreSQL, SQLite, MySQL, MongoDB, and Redis from a single, beautiful desktop app — with an AI agent built right in.

[![Build](https://github.com/been-there-done-that/tables/actions/workflows/build.yml/badge.svg)](https://github.com/been-there-done-that/tables/actions/workflows/build.yml)
[![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Windows%20%7C%20Linux-blue)](#)
[![Version](https://img.shields.io/badge/version-0.0.2-green)](#)
[![License](https://img.shields.io/badge/license-MIT-lightgrey)](#)

</div>

---

## What is Tables?

Tables is a cross-platform desktop database management app built with **Tauri 2** and **SvelteKit**. Think DataGrip — but lighter, faster, and with an AI agent that understands your schema.

- **Native performance** via Rust backend — no Electron overhead
- **SQL editor** powered by Monaco with real-time diagnostics and auto-complete
- **Schema explorer** with inline table/column previews
- **AI assistant** that can query your database, read schemas, and help write SQL
- **Multi-provider AI** — Claude, Codex, Gemini, Cursor, and more via a pluggable harness
- **Query history** with per-connection logs and copy-to-clipboard
- **Auto-updater** — silent background checks and one-click installs

---

## Supported Databases

| Database | Status |
|----------|--------|
| PostgreSQL | Full support — streaming results, query cancellation, server-side cursors |
| SQLite | Full support |
| MySQL | Supported |
| MongoDB | Supported |
| Redis | Supported |

---

## Features

### SQL Editor
- Monaco-based editor with SQL syntax highlighting
- **Real-time diagnostics** — syntax errors, unknown tables, dangerous queries (DROP/DELETE/UPDATE without WHERE), powered by the PostgreSQL C parser (`pg_query`)
- **Per-statement run buttons** — run one query at a time without selecting text
- **EXPLAIN & Analysis panel** — visualize query execution plans
- Format SQL, undo/redo, multi-tab editing

### Schema Explorer
- Browse databases, schemas, tables, views, indexes, and foreign keys
- Click any table to preview data instantly
- Inline DDL viewer

### AI Agent
- Conversational agent that understands your live schema
- Tools: `run_query`, `list_tables`, `get_schema`, `explain_query`, file operations
- Thread persistence — picks up where you left off
- Multi-provider: configure Claude, OpenRouter, Codex, Gemini, Cursor, or any ACP-compatible provider
- Approval shield — confirm before the agent runs write queries

### Security
- Passwords stored via OS keyring + AES-GCM encryption — never in plaintext
- Credential isolation from connection metadata

---

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Desktop shell | [Tauri 2](https://tauri.app) (Rust) |
| Frontend | [SvelteKit](https://kit.svelte.dev) + Svelte 5 runes |
| Editor | [Monaco Editor](https://microsoft.github.io/monaco-editor/) |
| SQL parsing | [pg_query](https://github.com/pganalyze/pg_query) (PostgreSQL C parser) + tree-sitter |
| Async runtime | Tokio |
| DB drivers | tokio-postgres, rusqlite, mysql, mongodb, redis |
| AI harness | Custom multi-provider harness (Bun) |
| Styling | Tailwind CSS + shadcn-svelte |

---

## Getting Started

### Prerequisites

- [Rust](https://rustup.rs) (stable)
- [Node.js](https://nodejs.org) 22+
- [pnpm](https://pnpm.io) 10+
- [Bun](https://bun.sh) (for the AI harness)

### Development

```bash
# Install dependencies
pnpm install

# Run in dev mode (Vite + Rust hot reload)
pnpm tauri dev

# Frontend only
pnpm dev
```

### Build

```bash
# Production desktop app
pnpm tauri build

# Frontend only
pnpm build
```

### Tests

```bash
# Rust backend tests
cd src-tauri && cargo test

# With output
cd src-tauri && cargo test -- --nocapture

# Type check frontend
pnpm check
```

---

## Project Structure

```
tables/
├── src/                        # SvelteKit frontend
│   ├── lib/
│   │   ├── components/         # UI components (editor, explorer, agent, ...)
│   │   ├── stores/             # Svelte 5 rune-based state
│   │   ├── agent/              # AI agent system (tools, runner, providers)
│   │   └── monaco/             # Editor extensions (highlighting, headers)
│   └── routes/                 # Pages (main, datasource, settings, feedback)
├── src-tauri/                  # Rust backend
│   ├── src/
│   │   ├── adapters/           # DB adapters (postgres, sqlite, ...)
│   │   ├── commands/           # Tauri IPC commands
│   │   ├── completion/         # SQL completion + diagnostics engine
│   │   └── crates/sql-scope/  # Internal SQL semantic analysis crate
│   └── binaries/               # Built harness binaries (gitignored)
└── packages/harness/           # Multi-provider AI harness (TypeScript/Bun)
```

---

## Architecture

```
Frontend (SvelteKit / Svelte 5)
        ↕  Tauri IPC (invoke / emit)
Backend (Rust / Tokio)
        ↕  DatabaseAdapter trait
DB Drivers (tokio-postgres · rusqlite · mysql · mongodb · redis)
        ↕  AI Harness (Bun subprocess)
AI Providers (Claude · Codex · Gemini · Cursor · OpenRouter · ...)
```

The `sql-scope` internal crate wraps the PostgreSQL C parser (`pg_query`) to provide accurate statement splitting, scope analysis, and semantic diagnostics — no regex heuristics.

---

## Releases

Releases are built for macOS (Apple Silicon + Intel), Windows, and Linux via GitHub Actions. See the [Releases](../../releases) page to download the latest version.

---

## Contributing

1. Fork the repo
2. Create a feature branch
3. Run `cargo test` and `pnpm check` before opening a PR
4. Open a PR against `main`

---

<div align="center">
  <sub>Built with Tauri + SvelteKit + Rust &nbsp;·&nbsp; Made for developers who care about their tools</sub>
</div>
