# Functions, Sequences & Constraints SQLite Caching — Design Spec

**Date:** 2026-03-28
**Branch:** ai
**Scope:** Move functions, sequences, and constraints from lazy live-Postgres fetching into the SQLite introspection cache pipeline, matching how tables/columns/indexes/triggers are handled today.

---

## Problem

Functions, sequences, and constraints are currently fetched live from Postgres every time a user expands the corresponding folder in `PostgresExplorer.svelte` (`functionsCache`/`sequencesCache` local Maps + `loadFunctions()`/`loadSequences()` async calls). This causes:
- Repeated Postgres round-trips on every expand
- No offline/cached access
- Inconsistency with every other object type (tables, indexes, triggers all go through SQLite)

## Goal

Functions, sequences, and constraints join Level 4 of the progressive introspection pipeline. After introspection completes, the frontend reads them from `schemaStore.databases` (same as tables). No local caches, no live fetches on expand.

---

## Data Model

### Migration: `src-tauri/migrations/002_functions_sequences_constraints.sql`

```sql
-- Level 4: Functions (and procedures/aggregates/window functions)
CREATE TABLE IF NOT EXISTS meta_functions (
    function_id      INTEGER PRIMARY KEY AUTOINCREMENT,
    schema_id        INTEGER NOT NULL REFERENCES meta_schemas(schema_id) ON DELETE CASCADE,
    connection_id    TEXT NOT NULL,
    database         TEXT NOT NULL,
    schema           TEXT NOT NULL,
    name             TEXT NOT NULL,
    oid              INTEGER NOT NULL,         -- pg_proc.oid — differentiates overloads
    language         TEXT NOT NULL DEFAULT '',
    kind             TEXT NOT NULL CHECK(kind IN ('Function','Procedure','Aggregate','Window')),
    return_type      TEXT NOT NULL DEFAULT '',
    arguments        TEXT NOT NULL DEFAULT '[]',  -- JSON: [{name,data_type,mode,default_value}]
    definition       TEXT NOT NULL DEFAULT '',
    security_definer INTEGER NOT NULL DEFAULT 0,
    volatility       TEXT NOT NULL DEFAULT 'volatile',
    UNIQUE(schema_id, name, oid)
);
CREATE INDEX IF NOT EXISTS idx_meta_functions_schema ON meta_functions(schema_id);
CREATE INDEX IF NOT EXISTS idx_meta_functions_lookup ON meta_functions(connection_id, database, schema);

-- Level 4: Sequences
CREATE TABLE IF NOT EXISTS meta_sequences (
    sequence_id   INTEGER PRIMARY KEY AUTOINCREMENT,
    schema_id     INTEGER NOT NULL REFERENCES meta_schemas(schema_id) ON DELETE CASCADE,
    connection_id TEXT NOT NULL,
    database      TEXT NOT NULL,
    schema        TEXT NOT NULL,
    name          TEXT NOT NULL,
    data_type     TEXT NOT NULL DEFAULT 'bigint',
    start_value   INTEGER NOT NULL DEFAULT 1,
    min_value     INTEGER NOT NULL DEFAULT 1,
    max_value     INTEGER NOT NULL DEFAULT 9223372036854775807,
    increment_by  INTEGER NOT NULL DEFAULT 1,
    cycle         INTEGER NOT NULL DEFAULT 0,
    cache_size    INTEGER NOT NULL DEFAULT 1,
    last_value    INTEGER,
    UNIQUE(schema_id, name)
);
CREATE INDEX IF NOT EXISTS idx_meta_sequences_schema ON meta_sequences(schema_id);
CREATE INDEX IF NOT EXISTS idx_meta_sequences_lookup ON meta_sequences(connection_id, database, schema);

-- Level 4: Constraints (CHECK, UNIQUE, EXCLUSION — not FK/PK which have their own tables)
CREATE TABLE IF NOT EXISTS meta_constraints (
    constraint_id INTEGER PRIMARY KEY AUTOINCREMENT,
    table_id      INTEGER NOT NULL REFERENCES meta_tables(table_id) ON DELETE CASCADE,
    connection_id TEXT NOT NULL,
    database      TEXT NOT NULL,
    schema        TEXT NOT NULL,
    table_name    TEXT NOT NULL,
    name          TEXT NOT NULL,
    kind          TEXT NOT NULL CHECK(kind IN ('PrimaryKey','ForeignKey','Unique','Check','Exclusion')),
    definition    TEXT NOT NULL DEFAULT '',
    columns       TEXT NOT NULL DEFAULT '[]',  -- JSON array of column names
    UNIQUE(table_id, name)
);
CREATE INDEX IF NOT EXISTS idx_meta_constraints_table ON meta_constraints(table_id);
CREATE INDEX IF NOT EXISTS idx_meta_constraints_lookup ON meta_constraints(connection_id, database, schema, table_name);
```

---

## Rust Struct Changes (`src-tauri/src/introspection.rs`)

### Add `oid` to `MetaFunction`

```rust
pub struct MetaFunction {
    pub connection_id: String,
    pub database: String,
    pub name: String,
    pub schema: String,
    pub oid: i64,          // NEW — pg_proc.oid for overload disambiguation
    pub language: String,
    pub kind: FunctionKind,
    pub return_type: String,
    pub arguments: Vec<MetaFunctionArg>,
    pub definition: String,
    pub security_definer: bool,
    pub volatility: String,
}
```

### Add `functions` and `sequences` to `MetaSchema`

```rust
pub struct MetaSchema {
    pub name: String,
    pub schema_type: String,
    pub kind: NamespaceKind,
    pub is_introspected: bool,
    pub tables: Vec<MetaTable>,
    pub functions: Vec<MetaFunction>,   // NEW
    pub sequences: Vec<MetaSequence>,   // NEW
}
```

### Add `constraints` to `MetaTable`

```rust
pub struct MetaTable {
    // ... existing fields ...
    pub columns: Vec<MetaColumn>,
    pub foreign_keys: Vec<MetaForeignKey>,
    pub indexes: Vec<MetaIndex>,
    pub triggers: Vec<MetaTrigger>,
    pub constraints: Vec<MetaConstraint>,  // NEW
}
```

---

## Adapter Trait (`src-tauri/src/adapter.rs`)

Add three new optional methods to `DatabaseAdapter` trait (non-breaking: default impl returns empty `Vec`):

```rust
/// List all functions/procedures/aggregates in a schema.
async fn list_functions(&self, database: &str, schema: &str) -> Result<Vec<MetaFunction>, AdapterError> {
    Ok(vec![])
}

/// List all sequences in a schema.
async fn list_sequences(&self, database: &str, schema: &str) -> Result<Vec<MetaSequence>, AdapterError> {
    Ok(vec![])
}

/// List all constraints for a specific table.
async fn list_constraints(&self, table_ref: &TableRef) -> Result<Vec<MetaConstraint>, AdapterError> {
    Ok(vec![])
}
```

The import line in `adapter.rs` must add `MetaFunction`, `MetaSequence`, `MetaConstraint` to the `use crate::introspection::` import.

---

## PostgreSQL Adapter (`src-tauri/src/adapters/postgres.rs`)

### `list_functions` SQL

```sql
SELECT
    p.oid::bigint,
    p.proname AS name,
    l.lanname AS language,
    CASE p.prokind
        WHEN 'f' THEN 'Function'
        WHEN 'p' THEN 'Procedure'
        WHEN 'a' THEN 'Aggregate'
        WHEN 'w' THEN 'Window'
        ELSE 'Function'
    END AS kind,
    pg_catalog.pg_get_function_result(p.oid) AS return_type,
    pg_catalog.pg_get_function_arguments(p.oid) AS arguments_text,
    COALESCE(pg_catalog.pg_get_functiondef(p.oid), '') AS definition,
    p.prosecdef AS security_definer,
    CASE p.provolatile
        WHEN 'i' THEN 'immutable'
        WHEN 's' THEN 'stable'
        ELSE 'volatile'
    END AS volatility
FROM pg_proc p
JOIN pg_namespace n ON p.pronamespace = n.oid
JOIN pg_language l ON p.prolang = l.oid
WHERE n.nspname = $1
  AND NOT p.proisagg  -- exclude aggregates if using older pg_proc layout
ORDER BY p.proname, p.oid
```

> Note: `arguments_text` from `pg_get_function_arguments` is stored as a plain string in `definition`. The `arguments` field on `MetaFunction` stores parsed `Vec<MetaFunctionArg>`, but for the caching pipeline we can store them as an empty vec and populate `arguments` text in a `raw_arguments: String` field instead — **OR** store arguments as JSON from a separate query to `pg_proc.proargnames` + `pg_proc.proargtypes`. Given the complexity of parsing, store `arguments` as `vec![]` for the caching pipeline (the DDL is captured in `definition` already). The UI only needs `name`, `kind`, `return_type`, `definition` for display.

Simpler approach: keep `arguments: Vec<MetaFunctionArg>` as `vec![]` in the cached path; the existing `get_function_ddl` Tauri command already retrieves full DDL on demand.

### `list_sequences` SQL

```sql
SELECT
    sequencename AS name,
    data_type::text,
    start_value,
    min_value,
    max_value,
    increment_by,
    cycle,
    cache_size,
    last_value
FROM pg_sequences
WHERE schemaname = $1
ORDER BY sequencename
```

### `list_constraints` SQL (per table)

```sql
SELECT
    c.conname AS name,
    CASE c.contype
        WHEN 'p' THEN 'PrimaryKey'
        WHEN 'f' THEN 'ForeignKey'
        WHEN 'u' THEN 'Unique'
        WHEN 'c' THEN 'Check'
        WHEN 'x' THEN 'Exclusion'
        ELSE 'Check'
    END AS kind,
    pg_get_constraintdef(c.oid) AS definition,
    COALESCE(
        (SELECT json_agg(a.attname ORDER BY array_position(c.conkey, a.attnum))
         FROM pg_attribute a
         WHERE a.attrelid = c.conrelid AND a.attnum = ANY(c.conkey)),
        '[]'::json
    )::text AS columns
FROM pg_constraint c
JOIN pg_class t ON t.oid = c.conrelid
JOIN pg_namespace n ON n.oid = t.relnamespace
WHERE n.nspname = $1 AND t.relname = $2
  AND c.contype IN ('p','f','u','c','x')
ORDER BY c.contype, c.conname
```

The `columns` result is a JSON array string; deserialize into `Vec<String>` in Rust.

---

## Introspector Changes (`src-tauri/src/introspection.rs`)

### New save methods

```rust
pub fn save_functions(&self, schema_id: i64, fns: &[MetaFunction]) -> Result<(), String>
pub fn save_sequences(&self, schema_id: i64, seqs: &[MetaSequence]) -> Result<(), String>
pub fn save_constraints(&self, table_id: i64, constraints: &[MetaConstraint]) -> Result<(), String>
```

Each uses `INSERT OR REPLACE` (upsert via `ON CONFLICT DO UPDATE`):
- Functions: conflict on `(schema_id, name, oid)`
- Sequences: conflict on `(schema_id, name)`
- Constraints: conflict on `(table_id, name)`

JSON serialization for `arguments` (functions) and `columns` (constraints) uses `serde_json::to_string`.

### New get methods

```rust
pub fn get_functions(&self, schema_id: i64) -> Result<Vec<MetaFunction>, String>
pub fn get_sequences(&self, schema_id: i64) -> Result<Vec<MetaSequence>, String>
pub fn get_constraints(&self, table_id: i64) -> Result<Vec<MetaConstraint>, String>
```

### Update `save_table_full`

The private `save_table_full` method currently saves columns, FKs, indexes, triggers. Extend it to also save constraints via `self.save_constraints(table_id, &table.constraints)`.

### Update `get_schema` read path

`get_schema` → `get_tables` currently populates `table.foreign_keys`, `table.indexes`, `table.triggers` from SQLite. Extend to also populate:
- `table.constraints` from `meta_constraints` (join via `table_id`)
- `schema.functions` from `meta_functions` (join via `schema_id`)
- `schema.sequences` from `meta_sequences` (join via `schema_id`)

The `get_schema` method is at `introspection.rs:1217`. It calls `get_databases` → `get_schemas` → `get_tables_in_schema`. After assembling each `MetaSchema`, call `get_functions(schema_id)` and `get_sequences(schema_id)` and assign to `schema.functions` and `schema.sequences`. After assembling each `MetaTable` (inside `get_tables_in_schema` or inline in the schema assembly loop), call `get_constraints(table_id)` and assign to `table.constraints`.

### Update `clear_schema_cache`

The existing `clear_schema_cache_internal` clears tables (and cascades to columns, FKs, indexes, triggers). Since `meta_functions` and `meta_sequences` reference `schema_id` with `ON DELETE CASCADE`, they are automatically cleared when the schema row is deleted. No changes needed here.

---

## Orchestrator Changes (`src-tauri/src/orchestrator.rs`)

Level 4 in `introspect_database` and `introspect_schema` both need extending.

### `introspect_database` — Level 4 loop extension

Current loop (per schema, per table): fetches FKs, indexes, triggers.

New additions:
1. **Per schema** (after the per-table loop): fetch functions and sequences
2. **Per table** (inside existing loop): fetch constraints alongside FKs/indexes/triggers

```rust
// After per-table loop for schema:
let functions = self.adapter.list_functions(database_name, &schema.name).await
    .unwrap_or_default();
schema.functions = functions.into_iter().map(|mut f| { f.connection_id = connection_id.clone(); f }).collect();

let sequences = self.adapter.list_sequences(database_name, &schema.name).await
    .unwrap_or_default();
schema.sequences = sequences.into_iter().map(|mut s| { s.connection_id = connection_id.clone(); s }).collect();
```

```rust
// Inside per-table loop:
let constraints = self.adapter.list_constraints(&table_ref).await
    .unwrap_or_default();
table.constraints = constraints.into_iter().map(|mut c| { c.connection_id = connection_id.clone(); c }).collect();
```

### `introspect_schema` — metadata section extension

Same pattern: bulk-fetch functions and sequences for the schema, constraints per table. Append to `metadata_count`.

Add a `supports_functions` flag to `DatabaseCapabilities`:
```rust
pub supports_functions: bool,  // default: false
```
Set to `true` in `DatabaseCapabilities::postgres()`. Gate all three new fetches on this flag.

---

## Frontend Changes

### `src/lib/commands/types.ts`

Add new interfaces and update existing ones:

```typescript
export interface MetaFunctionArg {
  name?: string;
  data_type: string;
  mode: 'In' | 'Out' | 'InOut' | 'Variadic' | 'Table';
  default_value?: string;
}

export interface MetaFunction {
  connection_id: string;
  database: string;
  schema: string;
  name: string;
  oid: number;
  language: string;
  kind: 'Function' | 'Procedure' | 'Aggregate' | 'Window';
  return_type: string;
  arguments: MetaFunctionArg[];
  definition: string;
  security_definer: boolean;
  volatility: string;
}

export interface MetaSequence {
  connection_id: string;
  database: string;
  schema: string;
  name: string;
  data_type: string;
  start_value: number;
  min_value: number;
  max_value: number;
  increment_by: number;
  cycle: boolean;
  cache_size: number;
  last_value?: number;
}

export interface MetaConstraint {
  connection_id: string;
  database: string;
  schema: string;
  table_name: string;
  name: string;
  kind: 'PrimaryKey' | 'ForeignKey' | 'Unique' | 'Check' | 'Exclusion';
  definition: string;
  columns: string[];
}
```

Update `MetaSchema` to include `functions` and `sequences`:
```typescript
export interface MetaSchema {
  name: string;
  schema_type: "user" | "system";
  tables: MetaTable[];
  functions: MetaFunction[];   // NEW
  sequences: MetaSequence[];   // NEW
}
```

Update `MetaTable` to include `constraints`:
```typescript
export interface MetaTable {
  // ... existing fields ...
  constraints: MetaConstraint[];  // NEW
}
```

### `src/lib/components/explorer/engines/PostgresExplorer.svelte`

**Remove:**
- `let functionsCache = $state<Map<string, any[]>>(new Map());`
- `let sequencesCache = $state<Map<string, any[]>>(new Map());`
- `async function loadFunctions(dbName, schemaName) { ... }`
- `async function loadSequences(dbName, schemaName) { ... }`
- The `loadFunctions(...)` and `loadSequences(...)` calls in the folder-expand handler
- The `functionsCache = new Map()` and `sequencesCache = new Map()` in the refresh/clear handler

**Replace reads:**
- Where code was `functionsCache.get(cacheKey) || []`, read `schema.functions ?? []`
- Where code was `sequencesCache.get(cacheKey) || []`, read `schema.sequences ?? []`
- Where constraints were read from live query result, read `table.constraints ?? []`

The `mapSchemaToNode` function reads from the schema object passed from `schemaStore`. The `mapTableToNode` function reads from the table object.

---

## Load Order & Backwards Compatibility

1. Migration `002` runs at app startup via existing `run_migrations()` call — no manual step.
2. On first connection after upgrade: `get_schema` returns empty `functions`/`sequences`/`constraints` arrays (new columns). The UI renders empty folders. Next time the user triggers a refresh or the background introspection runs, the data populates.
3. The existing `get_functions`, `get_sequences`, `get_constraints` Tauri commands in `introspection_commands.rs` remain — they are still used by the DDL context menus (which fetch individual object definitions on demand). **Do not remove them.**

---

## Files Affected

| File | Change |
|------|--------|
| `src-tauri/migrations/002_functions_sequences_constraints.sql` | **Create** — 3 new tables |
| `src-tauri/src/introspection.rs` | Add `oid` to `MetaFunction`; add `functions`/`sequences` to `MetaSchema`; add `constraints` to `MetaTable`; add `save_functions`, `save_sequences`, `save_constraints`, `get_functions`, `get_sequences`, `get_constraints`; update `save_table_full`, `get_schema` load path |
| `src-tauri/src/adapter.rs` | Add `list_functions`, `list_sequences`, `list_constraints` default trait methods; add `supports_functions` to `DatabaseCapabilities`; update `postgres()` constructor |
| `src-tauri/src/adapters/postgres.rs` | Implement `list_functions`, `list_sequences`, `list_constraints` |
| `src-tauri/src/orchestrator.rs` | Level 4: add functions/sequences per schema, constraints per table |
| `src/lib/commands/types.ts` | Add `MetaFunction`, `MetaSequence`, `MetaConstraint` interfaces; update `MetaSchema`, `MetaTable` |
| `src/lib/components/explorer/engines/PostgresExplorer.svelte` | Remove lazy-load caches; read from `schema.functions`, `schema.sequences`, `table.constraints` |

---

## Out of Scope

- SQLite or MySQL adapter implementations of the new trait methods (defaults return `[]`)
- Changing how the DDL Tauri commands work (`get_function_ddl`, etc.)
- `list_constraints_schema` bulk variant (per-table is sufficient for Level 4)
- Partitioned table constraints
- Domain constraints
