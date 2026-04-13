# Tables — Feature Roadmap

Features to build, roughly in priority order. Updated as things ship or get re-prioritised.

---

## In Progress / Next Up

### Export Results to File
**Status:** Not started
**What:** Save query results to disk — CSV, JSON, JSONL, SQL INSERT format.
**Approach:** Backend-driven. Rust streams rows directly to a file via `dialog::save` + chunked write, so large result sets never load fully into the frontend. Frontend shows a toolbar button → file picker → progress notification.
**Why backend:** Avoids memory ceiling in frontend for large tables; Rust handles encoding and newlines correctly.

### Saved / Favorite Queries (Pinned Query Files)
**Status:** Not started
**What:** User saves a query → stored as a named `.sql` file on disk. The file shows up as a persistently pinned tab in the editor (survives restarts). Sidebar shows a "Saved" section listing all pinned queries.
**Design notes:**
- Pinned tabs are distinct from session tabs — they don't close when the session ends.
- Saved queries stored at `~/.config/tables/saved_queries/` (or user-configurable).
- Clicking a saved query in the sidebar opens/focuses its pinned tab.
- Can be tied to a specific connection or be connection-agnostic.

---

## Backlog

### Table Builder UI (Create / Alter Table)
**Status:** Not started — on roadmap
**What:** Visual GUI to create a new table or alter an existing one — add/remove/rename columns, set types, constraints, indexes, FKs — without writing DDL by hand. Like DataGrip's table editor.
**Scope:** Phase 1 = CREATE TABLE. Phase 2 = ALTER TABLE (add column, drop column, rename).
**Engines:** PostgreSQL + SQLite first.

### Schema Diff
**Status:** Not started
**What:** Compare schema between two databases (e.g. dev vs prod) and show the DDL delta. Useful for catching migration drift.

### ER Diagram
**Status:** Not started
**What:** Auto-generated entity-relationship diagram from introspected FK relationships. Interactive — click a table node to inspect columns.

### Query Performance Insights (pg_stat_statements)
**Status:** Not started
**What:** Separate panel showing historically slow queries from `pg_stat_statements` — total calls, avg duration, cache hit ratio. Different from per-execution EXPLAIN — this is fleet-level slow query tracking.
**Postgres-only.**

### Schema Linter
**Status:** Not started
**What:** Static checks on schema health — missing indexes on FK columns, tables without PKs, RLS not enabled, oversized text columns, etc. Runs on demand or on connection.
**Postgres-only initially.**

---

## Shipped

- SQL editor with Monaco, multi-tab, per-tab session state
- Schema explorer (tables, columns, views, indexes, FK, triggers, functions, sequences, materialized views)
- DDL click-to-open (read-only editor tab for any schema object)
- Virtualized data grid with inline cell editing, add/delete rows, undo/redo, PK-aware deltas
- Query results with copy to clipboard (TSV / CSV / JSON)
- EXPLAIN (ANALYZE, BUFFERS) panel with waterfall chart and issue detection
- Query logs / history (persisted to SQLite, grouped by connection)
- AI agent chat panel with pendingMessage injection from editor
- Auto-complete with tree-sitter SQL parsing, multi-statement isolation, 4-pass scope sanitization
- Connection manager with cloud provider shortcuts (Supabase, Neon, etc.)
- Secure credential storage (OS keyring + AES-GCM)
- PostgreSQL, SQLite, MySQL, MongoDB, Redis adapters
