# PostgreSQL Feature Sprint — Design Spec

**Date:** 2026-03-28
**Branch:** ai
**Scope:** Read-only PostgreSQL object browsing, DDL generation, enriched explorer tree, pgAdmin-style context menus

---

## Overview

Tables currently supports core schema introspection (databases, schemas, tables, views, columns, foreign keys, basic indexes, basic triggers) but is missing the broader PostgreSQL object model that power users depend on daily. This sprint fills that gap across three parallel tracks — no write/mutation operations in scope.

**Decisions made:**
- **Read-only only** — no create/alter/drop dialogs
- **DDL opens in a new editor tab** — read-only, syntax-highlighted, copyable
- **Context menus** — flat, 4–6 items per object type, pgAdmin-style
- **Explorer tree** — schema-level folders (Tables, Views, Materialized Views, Functions, Procedures, Sequences); table-level sub-nodes (Columns, Indexes, Constraints, Triggers, Foreign Keys)

---

## Track 1 — Explorer Tree Restructure

### Schema-level folders

Each schema node expands to typed folders:

| Folder | Objects |
|---|---|
| Tables | Regular + partitioned tables |
| Views | Regular views |
| Materialized Views | Materialized views |
| Functions | `pg_proc` entries with `prokind = 'f'` |
| Procedures | `pg_proc` entries with `prokind = 'p'` |
| Sequences | `pg_sequences` entries |

Each folder shows object count badge. Lazy-loaded on expand.

### Table-level sub-nodes (enhanced)

| Sub-node | What changes |
|---|---|
| Columns | No change |
| Indexes | Add: index type (btree/hash/gin/gist/brin), columns list, partial predicate |
| Constraints | New: CHECK, UNIQUE, EXCLUSION constraints from `pg_constraint` |
| Triggers | Add: source code via `pg_trigger` + `pg_proc` |
| Foreign Keys | No change |

### NodeType additions

New `NodeType` variants needed in `FileTree.svelte`:
- `function`
- `procedure`
- `sequence`
- `constraint` (generic, for CHECK/UNIQUE/EXCLUSION)
- `materialized_view`

---

## Track 2 — Context Menus

Flat menu, separator before destructive/refresh actions. All "Open" actions open a read-only DDL tab.

### Per object type

**Table**
1. Copy Name
2. View Data
3. Open DDL *(opens CREATE TABLE tab)*
4. Copy as SELECT *
5. *(separator)*
6. Refresh

**View / Materialized View**
1. Copy Name
2. View Data
3. Open Definition *(opens CREATE VIEW tab)*
4. Copy as SELECT *
5. *(separator)*
6. Refresh

**Function**
1. Copy Name
2. Open Definition *(opens CREATE OR REPLACE FUNCTION tab)*
3. Copy Call Signature
4. Copy as CREATE FUNCTION

**Procedure**
1. Copy Name
2. Open Definition
3. Copy as CREATE PROCEDURE

**Sequence**
1. Copy Name
2. Open DDL *(opens CREATE SEQUENCE tab)*
3. Copy as CREATE SEQUENCE

**Index**
1. Copy Name
2. Open Definition *(opens CREATE INDEX tab)*
3. Copy DDL

**Trigger**
1. Copy Name
2. Open Definition *(opens trigger function source tab)*
3. Copy DDL

**Constraint**
1. Copy Name
2. Copy DDL *(e.g. `CHECK (amount > 0)`)*

**Schema**
1. Copy Name
2. Open DDL *(CREATE SCHEMA)*
3. *(separator)*
4. Refresh Schema

---

## Track 3 — DDL Tabs + Backend Introspection

### Read-only editor tab behavior

- Tab title format: `⚡ function_name`, `📋 table_name DDL`, `🔄 trigger_name`, etc.
- Tab is marked read-only (Monaco `readOnly: true`)
- Language: `sql` for syntax highlighting
- Standard tab controls (close, pin) work normally
- Tab is reused if same object is already open (focus existing tab)

### New Rust structs

```rust
// src-tauri/src/introspection.rs additions

pub struct MetaFunction {
    pub name: String,
    pub schema: String,
    pub language: String,         // plpgsql, sql, c, etc.
    pub kind: FunctionKind,       // Function | Procedure | Aggregate | Window
    pub return_type: String,
    pub arguments: Vec<MetaFunctionArg>,
    pub definition: String,       // full source
    pub security_definer: bool,
    pub volatility: String,       // volatile | stable | immutable
}

pub struct MetaFunctionArg {
    pub name: Option<String>,
    pub data_type: String,
    pub mode: ArgMode,            // In | Out | InOut | Variadic
    pub default: Option<String>,
}

pub struct MetaSequence {
    pub name: String,
    pub schema: String,
    pub data_type: String,        // bigint | integer | smallint
    pub start: i64,
    pub minimum: i64,
    pub maximum: i64,
    pub increment: i64,
    pub cycle: bool,
    pub cache_size: i64,
    pub last_value: Option<i64>,
}

pub struct MetaConstraint {
    pub name: String,
    pub kind: ConstraintKind,     // Check | Unique | Exclusion | PrimaryKey
    pub definition: String,       // the CHECK (...) or UNIQUE (...) expression
    pub columns: Vec<String>,
}
```

### New Tauri commands

| Command | Returns | Query source |
|---|---|---|
| `get_functions` | `Vec<MetaFunction>` | `pg_proc` + `pg_namespace` |
| `get_sequences` | `Vec<MetaSequence>` | `pg_sequences` |
| `get_view_definition` | `String` | `pg_views.definition` |
| `get_matview_definition` | `String` | `pg_matviews.definition` |
| `get_trigger_definition` | `String` | `pg_trigger` + `pg_proc` |
| `get_table_ddl` | `String` | assembled from `pg_class` + `pg_attribute` + `pg_constraint` + `pg_index` (covers regular + partitioned tables) |
| `get_function_ddl` | `String` | `pg_proc.prosrc` wrapped in full CREATE statement |
| `get_sequence_ddl` | `String` | assembled from `pg_sequences` |
| `get_index_ddl` | `String` | `pg_indexes.indexdef` |
| `get_constraints` | `Vec<MetaConstraint>` | `pg_constraint` |
| `get_index_details` | enhanced `MetaIndex` | `pg_index` + `pg_am` + `pg_attribute` |

### DDL generation approach

- **Tables:** Assemble from `pg_class`, `pg_attribute`, `pg_constraint`, `pg_index` — same approach as beekeeper-studio's `scripts.ts`
- **Views/MatViews:** `pg_views.definition` / `pg_matviews.definition` — Postgres stores the canonical definition
- **Functions/Procedures:** `pg_proc.prosrc` + reconstruct full `CREATE OR REPLACE` header from `pg_proc` metadata
- **Sequences:** Assemble from `pg_sequences` system view
- **Indexes:** `pg_indexes.indexdef` — Postgres stores the full CREATE INDEX statement
- **Triggers:** `pg_get_triggerdef(oid)` built-in function returns the full DDL

### Enhanced MetaIndex

Add to existing `MetaIndex`:
- `index_type: String` — from `pg_am.amname` (btree, hash, gin, gist, brin, spgist)
- `columns: Vec<String>` — from `pg_attribute`
- `predicate: Option<String>` — from `pg_index.indpred` (for partial indexes)
- `is_primary: bool` — already exists
- `definition: String` — full `CREATE INDEX` statement

---

## Data Flow

```
User right-clicks object in FileTree
  → ExplorerContextMenu shows flat menu
  → User clicks "Open DDL" / "Open Definition"
  → Frontend calls invoke('get_table_ddl' | 'get_function_ddl' | ...)
  → Rust queries pg_* system catalogs
  → Returns DDL string
  → Frontend opens new read-only Monaco tab with sql language
  → Tab title shows object type icon + name
```

```
User expands "Functions" folder in schema
  → Frontend calls invoke('get_functions', { connection_id, database, schema })
  → Rust queries pg_proc
  → Returns Vec<MetaFunction>
  → Tree renders function nodes with ⚡ icon
  → Right-click → context menu for functions
```

---

## Files Affected

### Backend
- `src-tauri/src/introspection.rs` — new structs + query functions for functions, sequences, constraints, enhanced indexes
- `src-tauri/src/commands/introspection_commands.rs` — new Tauri commands
- `src-tauri/src/commands/query_commands.rs` — DDL generation commands
- `src-tauri/src/adapter.rs` — capability flags for new features

### Frontend
- `src/lib/components/ExplorerContainer.svelte` — wire up new folder nodes
- `src/lib/components/FileTree.svelte` — new NodeType variants, icons, lazy loading for new types
- `src/lib/components/ExplorerContextMenu.svelte` — per-type menu items
- `src/lib/components/EditorTabs.svelte` — read-only tab mode, reuse-existing-tab logic
- `src/lib/stores/schema.svelte.ts` — store functions, sequences
- `src/lib/stores/session.svelte.ts` — open DDL tab action

---

## Out of Scope (this sprint)

- Create / Alter / Drop for any object type
- Role & permission management
- Row Level Security policies
- Extensions management
- Publications / subscriptions
- EXPLAIN / query plan
- VACUUM / ANALYZE / maintenance
- SSH tunneling
- Full Text Search objects
- Operators, collations, domains, casts
