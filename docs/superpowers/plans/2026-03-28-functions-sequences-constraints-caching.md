# Functions, Sequences & Constraints SQLite Caching — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move functions, sequences, and constraints from lazy live-Postgres fetching into the SQLite introspection pipeline so the frontend reads them from `schemaStore` like tables.

**Architecture:** New SQLite migration adds `meta_functions`, `meta_sequences`, `meta_constraints` tables. The Rust adapter trait gains three new optional methods (`list_functions`, `list_sequences`, `list_constraints`) implemented by `PostgresAdapter`. The `Introspector` gains save/get methods called by `save_introspected_database` and the `get_schema` read path. The orchestrator's Level 4 loop fetches functions/sequences per schema and constraints per table. Frontend removes `functionsCache`/`sequencesCache` Maps and reads `schema.functions`/`schema.sequences`/`table.constraints` from `schemaStore`.

**Tech Stack:** Rust/Tokio, tokio-postgres, rusqlite (SQLite), Svelte 5 runes, Tauri 2 IPC

---

## File Map

| File | Change |
|------|--------|
| `src-tauri/migrations/010_functions_sequences_constraints.sql` | **Create** |
| `src-tauri/src/introspection.rs` | Add `oid` to `MetaFunction`; `functions`/`sequences` to `MetaSchema`; `constraints` to `MetaTable`; `PrimaryKey`/`ForeignKey` to `ConstraintKind`; new save/get methods; update save/load paths |
| `src-tauri/src/adapter.rs` | Add `supports_functions` to `DatabaseCapabilities`; add 3 new trait methods + Box forwarding |
| `src-tauri/src/adapters/postgres.rs` | Implement `list_functions`, `list_sequences`, `list_constraints` |
| `src-tauri/src/orchestrator.rs` | Level 4: add functions/sequences per schema, constraints per table |
| `src/lib/commands/types.ts` | Add `MetaFunction`, `MetaSequence`, `MetaConstraint`; update `MetaSchema`, `MetaTable` |
| `src/lib/components/explorer/engines/PostgresExplorer.svelte` | Remove lazy caches; read from schemaStore |

---

## Task 1: SQLite Migration

**Files:**
- Create: `src-tauri/migrations/010_functions_sequences_constraints.sql`

- [ ] **Step 1: Write the migration file**

```sql
-- Level 4: Functions (and procedures/aggregates/window functions)
CREATE TABLE IF NOT EXISTS meta_functions (
    function_id      INTEGER PRIMARY KEY AUTOINCREMENT,
    schema_id        INTEGER NOT NULL REFERENCES meta_schemas(schema_id) ON DELETE CASCADE,
    connection_id    TEXT NOT NULL,
    database         TEXT NOT NULL,
    schema           TEXT NOT NULL,
    name             TEXT NOT NULL,
    oid              INTEGER NOT NULL,
    language         TEXT NOT NULL DEFAULT '',
    kind             TEXT NOT NULL CHECK(kind IN ('Function','Procedure','Aggregate','Window')),
    return_type      TEXT NOT NULL DEFAULT '',
    arguments        TEXT NOT NULL DEFAULT '[]',
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

-- Level 4: Constraints
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
    columns       TEXT NOT NULL DEFAULT '[]',
    UNIQUE(table_id, name)
);
CREATE INDEX IF NOT EXISTS idx_meta_constraints_table ON meta_constraints(table_id);
CREATE INDEX IF NOT EXISTS idx_meta_constraints_lookup ON meta_constraints(connection_id, database, schema, table_name);
```

- [ ] **Step 2: Verify migration is picked up by the migration runner**

Open `src-tauri/src/migrations.rs` (or wherever migrations are applied). Confirm it uses `include_str!` glob or numbered-file discovery that will pick up `002_*.sql` automatically. If it uses a hardcoded list, add `include_str!("../../migrations/010_functions_sequences_constraints.sql")` to the list.

Run:
```bash
cd src-tauri && cargo build 2>&1 | head -50
```
Expected: compiles without errors.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/migrations/010_functions_sequences_constraints.sql
git commit -m "feat: add SQLite migration for meta_functions, meta_sequences, meta_constraints"
```

---

## Task 2: Rust Struct Updates

**Files:**
- Modify: `src-tauri/src/introspection.rs`

The structs `MetaFunction`, `MetaSchema`, `MetaTable`, and `ConstraintKind` all need changes. Every place these structs are constructed (in introspection.rs itself, orchestrator.rs, postgres.rs) needs to be updated.

- [ ] **Step 1: Add `oid: i64` to `MetaFunction` and `PrimaryKey`/`ForeignKey` to `ConstraintKind`**

In `introspection.rs`, find `MetaFunction` (line ~152) and add `oid`:

```rust
pub struct MetaFunction {
    pub connection_id: String,
    pub database: String,
    pub name: String,
    pub schema: String,
    pub oid: i64,              // NEW — pg_proc.oid, differentiates overloads
    pub language: String,
    pub kind: FunctionKind,
    pub return_type: String,
    pub arguments: Vec<MetaFunctionArg>,
    pub definition: String,
    pub security_definer: bool,
    pub volatility: String,
}
```

Find `ConstraintKind` (line ~183) and add two new variants:

```rust
pub enum ConstraintKind {
    PrimaryKey,   // NEW
    ForeignKey,   // NEW
    Check,
    Unique,
    Exclusion,
}
```

- [ ] **Step 2: Add `functions` and `sequences` to `MetaSchema`**

Find `MetaSchema` (line ~21) and add two fields:

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

- [ ] **Step 3: Add `constraints` to `MetaTable`**

Find `MetaTable` (line ~29) and add one field:

```rust
pub struct MetaTable {
    pub connection_id: String,
    pub database: String,
    pub schema: String,
    pub table_name: String,
    pub table_type: String,
    pub classification: String,
    pub last_introspected_at: i64,
    pub columns: Vec<MetaColumn>,
    pub foreign_keys: Vec<MetaForeignKey>,
    pub indexes: Vec<MetaIndex>,
    pub triggers: Vec<MetaTrigger>,
    pub constraints: Vec<MetaConstraint>,  // NEW
}
```

- [ ] **Step 4: Fix all struct literal constructions — run cargo build to find them**

```bash
cd src-tauri && cargo build 2>&1 | grep "error\[E"
```

For every compilation error about missing field `functions`, `sequences`, `constraints`, or `oid`, add the appropriate `vec![]` or `0i64` default. The errors will point to exact line numbers.

Common locations to fix:
- `introspection.rs` `get_tables()` ~line 977: `MetaTable { ... triggers: vec![], }` → add `constraints: vec![]`
- `introspection.rs` `get_tables_in_schema()` ~line 1195: same
- `introspection.rs` `get_table_details()` ~line 1241: same
- `introspection.rs` `get_schemas()` ~line 1166: `MetaSchema { ... tables: vec![], }` → add `functions: vec![], sequences: vec![]`
- `orchestrator.rs` ~line 299: `MetaSchema { ... is_introspected: false, tables: vec![] }` → add `functions: vec![], sequences: vec![]`
- `adapters/postgres.rs` `list_schemas()`: same MetaSchema fix
- Any place `MetaFunction` is constructed: add `oid: 0`

- [ ] **Step 5: Confirm build passes**

```bash
cd src-tauri && cargo build 2>&1 | tail -5
```
Expected: `Finished dev [unoptimized + debuginfo] target(s) in ...`

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/introspection.rs
git commit -m "feat: add oid to MetaFunction, functions/sequences to MetaSchema, constraints to MetaTable"
```

---

## Task 3: Adapter Trait Updates

**Files:**
- Modify: `src-tauri/src/adapter.rs`

- [ ] **Step 1: Add `supports_functions` to `DatabaseCapabilities`**

In the `DatabaseCapabilities` struct (around line 123), add after `supports_triggers`:

```rust
/// Whether the engine supports user-defined functions/procedures/sequences
pub supports_functions: bool,
```

In `impl Default for DatabaseCapabilities` (around line 156):
```rust
supports_functions: false,
```

In `DatabaseCapabilities::postgres()` (around line 176):
```rust
supports_functions: true,
```

All other engines (`sqlite()`, `mysql()`, `athena()`, `mongodb()`, `redis()`) get `supports_functions: false` added to their constructor (add after `supports_triggers: ...`).

- [ ] **Step 2: Update the import line at the top of adapter.rs**

Find the existing import (line ~14):
```rust
use crate::introspection::{
    MetaDatabase, MetaSchema, MetaTable, MetaColumn, MetaIndex, MetaForeignKey, MetaTrigger
};
```

Replace with:
```rust
use crate::introspection::{
    MetaDatabase, MetaSchema, MetaTable, MetaColumn, MetaIndex, MetaForeignKey, MetaTrigger,
    MetaFunction, MetaSequence, MetaConstraint,
};
```

- [ ] **Step 3: Add three new trait methods after `list_triggers` (around line 497)**

```rust
// =========================================================================
// Level 4: Functions, Sequences, Constraints
// =========================================================================

/// List all functions/procedures for a schema. Default: not supported.
async fn list_functions(&self, _database: &str, _schema: &str) -> Result<Vec<MetaFunction>, AdapterError> {
    Ok(vec![])
}

/// List all sequences for a schema. Default: not supported.
async fn list_sequences(&self, _database: &str, _schema: &str) -> Result<Vec<MetaSequence>, AdapterError> {
    Ok(vec![])
}

/// List all constraints for a table. Default: not supported.
async fn list_constraints(&self, _table: &TableRef) -> Result<Vec<MetaConstraint>, AdapterError> {
    Ok(vec![])
}
```

- [ ] **Step 4: Add forwarding methods to the `Box<dyn DatabaseAdapter>` impl (around line 564)**

After the `list_triggers` forwarding:

```rust
async fn list_functions(&self, database: &str, schema: &str) -> Result<Vec<MetaFunction>, AdapterError> {
    (**self).list_functions(database, schema).await
}

async fn list_sequences(&self, database: &str, schema: &str) -> Result<Vec<MetaSequence>, AdapterError> {
    (**self).list_sequences(database, schema).await
}

async fn list_constraints(&self, table: &TableRef) -> Result<Vec<MetaConstraint>, AdapterError> {
    (**self).list_constraints(table).await
}
```

- [ ] **Step 5: Build and run existing adapter tests**

```bash
cd src-tauri && cargo test test_capabilities -- --nocapture
```
Expected: all tests pass.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/adapter.rs
git commit -m "feat: add supports_functions flag and list_functions/sequences/constraints trait methods"
```

---

## Task 4: PostgreSQL Adapter Implementations

**Files:**
- Modify: `src-tauri/src/adapters/postgres.rs`

Add the three new methods after `list_triggers_schema` (which ends around line 1000).

- [ ] **Step 1: Add the import for new types at the top of postgres.rs**

Find the existing import from `crate::introspection`:
```rust
use crate::introspection::{MetaDatabase, MetaSchema, MetaTable, MetaColumn, MetaIndex, MetaForeignKey, MetaTrigger, ...};
```
Add `MetaFunction, MetaSequence, MetaConstraint, FunctionKind, ConstraintKind` to it.

- [ ] **Step 2: Implement `list_functions`**

Add inside `impl DatabaseAdapter for PostgresAdapter`:

```rust
async fn list_functions(&self, database: &str, schema: &str) -> Result<Vec<MetaFunction>, AdapterError> {
    self.ensure_connected(database).await?;
    let state_guard = self.state.lock().await;
    let client = &state_guard.as_ref().unwrap().client;

    let rows = client.query(
        "SELECT
             p.oid::bigint,
             p.proname,
             l.lanname,
             CASE p.prokind
                 WHEN 'f' THEN 'Function'
                 WHEN 'p' THEN 'Procedure'
                 WHEN 'a' THEN 'Aggregate'
                 WHEN 'w' THEN 'Window'
                 ELSE 'Function'
             END,
             COALESCE(pg_catalog.pg_get_function_result(p.oid), ''),
             COALESCE(pg_catalog.pg_get_functiondef(p.oid), ''),
             p.prosecdef,
             CASE p.provolatile
                 WHEN 'i' THEN 'immutable'
                 WHEN 's' THEN 'stable'
                 ELSE 'volatile'
             END
         FROM pg_proc p
         JOIN pg_namespace n ON p.pronamespace = n.oid
         JOIN pg_language l ON p.prolang = l.oid
         WHERE n.nspname = $1
         ORDER BY p.proname, p.oid",
        &[&schema],
    ).await.map_err(|e| AdapterError::Query(format!("Failed to list functions in {}: {}", schema, e)))?;

    let mut functions = Vec::new();
    for row in rows {
        let oid: i64 = row.get(0);
        let name: String = row.get(1);
        let language: String = row.get(2);
        let kind_str: String = row.get(3);
        let return_type: String = row.get(4);
        let definition: String = row.get(5);
        let security_definer: bool = row.get(6);
        let volatility: String = row.get(7);

        let kind = match kind_str.as_str() {
            "Procedure" => FunctionKind::Procedure,
            "Aggregate" => FunctionKind::Aggregate,
            "Window" => FunctionKind::Window,
            _ => FunctionKind::Function,
        };

        functions.push(MetaFunction {
            connection_id: String::new(),
            database: database.to_string(),
            schema: schema.to_string(),
            name,
            oid,
            language,
            kind,
            return_type,
            arguments: vec![],
            definition,
            security_definer,
            volatility,
        });
    }
    Ok(functions)
}
```

- [ ] **Step 3: Implement `list_sequences`**

```rust
async fn list_sequences(&self, database: &str, schema: &str) -> Result<Vec<MetaSequence>, AdapterError> {
    self.ensure_connected(database).await?;
    let state_guard = self.state.lock().await;
    let client = &state_guard.as_ref().unwrap().client;

    let rows = client.query(
        "SELECT
             sequencename,
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
         ORDER BY sequencename",
        &[&schema],
    ).await.map_err(|e| AdapterError::Query(format!("Failed to list sequences in {}: {}", schema, e)))?;

    let mut sequences = Vec::new();
    for row in rows {
        sequences.push(MetaSequence {
            connection_id: String::new(),
            database: database.to_string(),
            schema: schema.to_string(),
            name: row.get(0),
            data_type: row.get(1),
            start_value: row.get(2),
            min_value: row.get(3),
            max_value: row.get(4),
            increment_by: row.get(5),
            cycle: row.get(6),
            cache_size: row.get(7),
            last_value: row.get(8),
        });
    }
    Ok(sequences)
}
```

- [ ] **Step 4: Implement `list_constraints`**

```rust
async fn list_constraints(&self, table: &TableRef) -> Result<Vec<MetaConstraint>, AdapterError> {
    self.ensure_connected(&table.database).await?;
    let state_guard = self.state.lock().await;
    let client = &state_guard.as_ref().unwrap().client;

    let rows = client.query(
        "SELECT
             c.conname,
             CASE c.contype
                 WHEN 'p' THEN 'PrimaryKey'
                 WHEN 'f' THEN 'ForeignKey'
                 WHEN 'u' THEN 'Unique'
                 WHEN 'c' THEN 'Check'
                 WHEN 'x' THEN 'Exclusion'
                 ELSE 'Check'
             END,
             COALESCE(pg_get_constraintdef(c.oid), ''),
             COALESCE(
                 (SELECT json_agg(a.attname ORDER BY array_position(c.conkey, a.attnum))
                  FROM pg_attribute a
                  WHERE a.attrelid = c.conrelid AND a.attnum = ANY(c.conkey)),
                 '[]'::json
             )::text
         FROM pg_constraint c
         JOIN pg_class t ON t.oid = c.conrelid
         JOIN pg_namespace n ON n.oid = t.relnamespace
         WHERE n.nspname = $1 AND t.relname = $2
           AND c.contype IN ('p','f','u','c','x')
         ORDER BY c.contype, c.conname",
        &[&table.schema, &table.name],
    ).await.map_err(|e| AdapterError::Query(format!("Failed to list constraints for {}.{}: {}", table.schema, table.name, e)))?;

    let mut constraints = Vec::new();
    for row in rows {
        let name: String = row.get(0);
        let kind_str: String = row.get(1);
        let definition: String = row.get(2);
        let columns_json: String = row.get(3);

        let kind = match kind_str.as_str() {
            "PrimaryKey" => ConstraintKind::PrimaryKey,
            "ForeignKey" => ConstraintKind::ForeignKey,
            "Unique" => ConstraintKind::Unique,
            "Exclusion" => ConstraintKind::Exclusion,
            _ => ConstraintKind::Check,
        };

        let columns: Vec<String> = serde_json::from_str(&columns_json).unwrap_or_default();

        constraints.push(MetaConstraint {
            connection_id: String::new(),
            database: table.database.clone(),
            schema: table.schema.clone(),
            table_name: table.name.clone(),
            name,
            kind,
            definition,
            columns,
        });
    }
    Ok(constraints)
}
```

- [ ] **Step 5: Build**

```bash
cd src-tauri && cargo build 2>&1 | tail -5
```
Expected: `Finished dev [unoptimized + debuginfo]`

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/adapters/postgres.rs
git commit -m "feat: implement list_functions, list_sequences, list_constraints in PostgresAdapter"
```

---

## Task 5: Introspector Save/Get + Schema Load

**Files:**
- Modify: `src-tauri/src/introspection.rs`

This is the biggest task. We add save/get methods, extend `save_introspected_database` and `save_table_full`, and extend `get_schema`'s read path.

- [ ] **Step 1: Add private save helper `save_functions_internal`**

Add after `save_trigger` (around line 968):

```rust
fn save_functions_internal(&self, conn: &SqliteConnection, schema_id: i64, connection_id: &str, database: &str, schema: &str, functions: &[MetaFunction]) -> Result<(), String> {
    if functions.is_empty() { return Ok(()); }
    // Prune stale functions for this schema
    conn.execute("DELETE FROM meta_functions WHERE schema_id = ?1", params![schema_id])
        .map_err(|e| format!("Failed to prune functions for schema_id {}: {}", schema_id, e))?;
    for f in functions {
        let kind_str = match f.kind {
            FunctionKind::Function => "Function",
            FunctionKind::Procedure => "Procedure",
            FunctionKind::Aggregate => "Aggregate",
            FunctionKind::Window => "Window",
        };
        let arguments_json = serde_json::to_string(&f.arguments).unwrap_or_else(|_| "[]".to_string());
        conn.execute(
            "INSERT INTO meta_functions
                 (schema_id, connection_id, database, schema, name, oid, language, kind, return_type, arguments, definition, security_definer, volatility)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
             ON CONFLICT(schema_id, name, oid) DO UPDATE SET
                 language=excluded.language, kind=excluded.kind,
                 return_type=excluded.return_type, arguments=excluded.arguments,
                 definition=excluded.definition, security_definer=excluded.security_definer,
                 volatility=excluded.volatility",
            params![
                schema_id, connection_id, database, schema,
                f.name, f.oid, f.language, kind_str,
                f.return_type, arguments_json, f.definition,
                f.security_definer as i32, f.volatility
            ],
        ).map_err(|e| format!("Failed to save function '{}': {}", f.name, e))?;
    }
    Ok(())
}
```

- [ ] **Step 2: Add private save helper `save_sequences_internal`**

```rust
fn save_sequences_internal(&self, conn: &SqliteConnection, schema_id: i64, connection_id: &str, database: &str, schema: &str, sequences: &[MetaSequence]) -> Result<(), String> {
    if sequences.is_empty() { return Ok(()); }
    conn.execute("DELETE FROM meta_sequences WHERE schema_id = ?1", params![schema_id])
        .map_err(|e| format!("Failed to prune sequences for schema_id {}: {}", schema_id, e))?;
    for s in sequences {
        conn.execute(
            "INSERT INTO meta_sequences
                 (schema_id, connection_id, database, schema, name, data_type, start_value, min_value, max_value, increment_by, cycle, cache_size, last_value)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
             ON CONFLICT(schema_id, name) DO UPDATE SET
                 data_type=excluded.data_type, start_value=excluded.start_value,
                 min_value=excluded.min_value, max_value=excluded.max_value,
                 increment_by=excluded.increment_by, cycle=excluded.cycle,
                 cache_size=excluded.cache_size, last_value=excluded.last_value",
            params![
                schema_id, connection_id, database, schema, s.name,
                s.data_type, s.start_value, s.min_value, s.max_value,
                s.increment_by, s.cycle as i32, s.cache_size, s.last_value
            ],
        ).map_err(|e| format!("Failed to save sequence '{}': {}", s.name, e))?;
    }
    Ok(())
}
```

- [ ] **Step 3: Add private save helper `save_constraints_internal`**

```rust
fn save_constraints_internal(&self, conn: &SqliteConnection, table_id: i64, constraints: &[MetaConstraint]) -> Result<(), String> {
    if constraints.is_empty() { return Ok(()); }
    for c in constraints {
        let kind_str = match c.kind {
            ConstraintKind::PrimaryKey => "PrimaryKey",
            ConstraintKind::ForeignKey => "ForeignKey",
            ConstraintKind::Check => "Check",
            ConstraintKind::Unique => "Unique",
            ConstraintKind::Exclusion => "Exclusion",
        };
        let columns_json = serde_json::to_string(&c.columns).unwrap_or_else(|_| "[]".to_string());
        conn.execute(
            "INSERT INTO meta_constraints
                 (table_id, connection_id, database, schema, table_name, name, kind, definition, columns)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(table_id, name) DO UPDATE SET
                 kind=excluded.kind, definition=excluded.definition, columns=excluded.columns",
            params![
                table_id, c.connection_id, c.database, c.schema, c.table_name,
                c.name, kind_str, c.definition, columns_json
            ],
        ).map_err(|e| format!("Failed to save constraint '{}': {}", c.name, e))?;
    }
    Ok(())
}
```

- [ ] **Step 4: Update `save_table_full` to save constraints**

Find `save_table_full` (around line 753). After the existing `for trigger in &table.triggers` block, add:

```rust
if !table.constraints.is_empty() {
    conn.execute("DELETE FROM meta_constraints WHERE table_id = ?1", params![table_id])
        .map_err(|e| e.to_string())?;
}
for constraint in &table.constraints {
    self.save_constraints_internal(conn, table_id, &[constraint.clone()])?;
}
```

Wait — `save_constraints_internal` takes a slice. Change the loop to a single call:

```rust
self.save_constraints_internal(conn, table_id, &table.constraints)?;
```

(The helper already no-ops if empty.)

- [ ] **Step 5: Update `save_introspected_database` to save functions and sequences**

Find `save_introspected_database` (around line 297). In the schema loop, after `self.save_schema(...)` which returns `schema_id`, add the functions and sequences save calls. The current code is:

```rust
for schema in &database.schemas {
    self.save_schema(&conn, connection_id, &database.name, &schema.name, &schema.schema_type, schema.kind)?;

    for table in &schema.tables {
        // ...fix connection_ids...
        self.save_table_full(&conn, &table_with_id)?;
    }
}
```

Change to:

```rust
for schema in &database.schemas {
    let schema_id = self.save_schema(&conn, connection_id, &database.name, &schema.name, &schema.schema_type, schema.kind)?;

    // Save functions and sequences for this schema
    let fns_with_id: Vec<MetaFunction> = schema.functions.iter().map(|f| {
        let mut f = f.clone();
        f.connection_id = connection_id.to_string();
        f.database = database.name.clone();
        f.schema = schema.name.clone();
        f
    }).collect();
    self.save_functions_internal(&conn, schema_id, connection_id, &database.name, &schema.name, &fns_with_id)?;

    let seqs_with_id: Vec<MetaSequence> = schema.sequences.iter().map(|s| {
        let mut s = s.clone();
        s.connection_id = connection_id.to_string();
        s.database = database.name.clone();
        s.schema = schema.name.clone();
        s
    }).collect();
    self.save_sequences_internal(&conn, schema_id, connection_id, &database.name, &schema.name, &seqs_with_id)?;

    for table in &schema.tables {
        let mut table_with_id = table.clone();
        table_with_id.connection_id = connection_id.to_string();
        table_with_id.database = database.name.clone();
        table_with_id.schema = schema.name.clone();

        for col in &mut table_with_id.columns { col.connection_id = connection_id.to_string(); }
        for fk in &mut table_with_id.foreign_keys { fk.connection_id = connection_id.to_string(); }
        for idx in &mut table_with_id.indexes { idx.connection_id = connection_id.to_string(); }
        for trg in &mut table_with_id.triggers { trg.connection_id = connection_id.to_string(); }
        for c in &mut table_with_id.constraints { c.connection_id = connection_id.to_string(); }

        self.save_table_full(&conn, &table_with_id)?;
    }
}
```

Note: `save_schema` currently returns `Result<i64, String>` — confirm this by checking its signature at line ~802. It does return `i64`. If it returns `()`, change it to return the schema_id. (Looking at the existing code, `save_schema` already returns `Ok(schema_id)` at line ~828. Good.)

- [ ] **Step 6: Add `get_functions` and `get_sequences` read helpers**

Add after the existing `get_triggers` logic (inside `get_tables`, around line 1114) — actually add as new public methods after `get_table_details`:

```rust
fn get_functions_for_schema(&self, conn: &SqliteConnection, connection_id: &str, database: &str, schema: &str) -> Result<Vec<MetaFunction>, String> {
    let mut stmt = conn.prepare(
        "SELECT name, oid, language, kind, return_type, arguments, definition, security_definer, volatility
         FROM meta_functions
         WHERE connection_id = ?1 AND database = ?2 AND schema = ?3
         ORDER BY name, oid"
    ).map_err(|e| e.to_string())?;

    let rows = stmt.query_map(params![connection_id, database, schema], |row| {
        let kind_str: String = row.get(3)?;
        let arguments_json: String = row.get(5)?;
        Ok((
            row.get::<_, String>(0)?,  // name
            row.get::<_, i64>(1)?,     // oid
            row.get::<_, String>(2)?,  // language
            kind_str,
            row.get::<_, String>(4)?,  // return_type
            arguments_json,
            row.get::<_, String>(6)?,  // definition
            row.get::<_, i32>(7)?,     // security_definer
            row.get::<_, String>(8)?,  // volatility
        ))
    }).map_err(|e| e.to_string())?;

    let mut functions = Vec::new();
    for r in rows {
        let (name, oid, language, kind_str, return_type, arguments_json, definition, sec_def, volatility) = r.map_err(|e| e.to_string())?;
        let kind = match kind_str.as_str() {
            "Procedure" => FunctionKind::Procedure,
            "Aggregate" => FunctionKind::Aggregate,
            "Window" => FunctionKind::Window,
            _ => FunctionKind::Function,
        };
        let arguments: Vec<MetaFunctionArg> = serde_json::from_str(&arguments_json).unwrap_or_default();
        functions.push(MetaFunction {
            connection_id: connection_id.to_string(),
            database: database.to_string(),
            schema: schema.to_string(),
            name, oid, language, kind, return_type, arguments, definition,
            security_definer: sec_def != 0,
            volatility,
        });
    }
    Ok(functions)
}

fn get_sequences_for_schema(&self, conn: &SqliteConnection, connection_id: &str, database: &str, schema: &str) -> Result<Vec<MetaSequence>, String> {
    let mut stmt = conn.prepare(
        "SELECT name, data_type, start_value, min_value, max_value, increment_by, cycle, cache_size, last_value
         FROM meta_sequences
         WHERE connection_id = ?1 AND database = ?2 AND schema = ?3
         ORDER BY name"
    ).map_err(|e| e.to_string())?;

    let rows = stmt.query_map(params![connection_id, database, schema], |row| {
        Ok(MetaSequence {
            connection_id: connection_id.to_string(),
            database: database.to_string(),
            schema: schema.to_string(),
            name: row.get(0)?,
            data_type: row.get(1)?,
            start_value: row.get(2)?,
            min_value: row.get(3)?,
            max_value: row.get(4)?,
            increment_by: row.get(5)?,
            cycle: row.get::<_, i32>(6)? != 0,
            cache_size: row.get(7)?,
            last_value: row.get(8)?,
        })
    }).map_err(|e| e.to_string())?;

    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

fn get_constraints_for_table(&self, conn: &SqliteConnection, connection_id: &str, database: &str, schema: &str, table_name: &str) -> Result<Vec<MetaConstraint>, String> {
    let mut stmt = conn.prepare(
        "SELECT name, kind, definition, columns
         FROM meta_constraints
         WHERE connection_id = ?1 AND database = ?2 AND schema = ?3 AND table_name = ?4
         ORDER BY kind, name"
    ).map_err(|e| e.to_string())?;

    let rows = stmt.query_map(params![connection_id, database, schema, table_name], |row| {
        let kind_str: String = row.get(1)?;
        let columns_json: String = row.get(3)?;
        Ok((row.get::<_, String>(0)?, kind_str, row.get::<_, String>(2)?, columns_json))
    }).map_err(|e| e.to_string())?;

    let mut constraints = Vec::new();
    for r in rows {
        let (name, kind_str, definition, columns_json) = r.map_err(|e| e.to_string())?;
        let kind = match kind_str.as_str() {
            "PrimaryKey" => ConstraintKind::PrimaryKey,
            "ForeignKey" => ConstraintKind::ForeignKey,
            "Unique" => ConstraintKind::Unique,
            "Exclusion" => ConstraintKind::Exclusion,
            _ => ConstraintKind::Check,
        };
        let columns: Vec<String> = serde_json::from_str(&columns_json).unwrap_or_default();
        constraints.push(MetaConstraint {
            connection_id: connection_id.to_string(),
            database: database.to_string(),
            schema: schema.to_string(),
            table_name: table_name.to_string(),
            name, kind, definition, columns,
        });
    }
    Ok(constraints)
}
```

- [ ] **Step 7: Update `get_schema` to load functions, sequences, and constraints**

Find `get_schema` (line ~1217). Currently:

```rust
pub fn get_schema(&self, connection_id: &str) -> Result<Vec<MetaDatabase>, String> {
    let mut dbs = self.get_databases(connection_id)?;
    for db in &mut dbs {
        db.schemas = self.get_schemas(connection_id, &db.name)?;
        for schema in &mut db.schemas {
            schema.tables = self.get_tables_in_schema(connection_id, &db.name, &schema.name)?;
        }
        if !db.schemas.is_empty() {
            db.is_introspected = true;
        }
    }
    Ok(dbs)
}
```

Replace with:

```rust
pub fn get_schema(&self, connection_id: &str) -> Result<Vec<MetaDatabase>, String> {
    let conn = self.app_db.lock().unwrap();
    let mut dbs = self.get_databases(connection_id)?;
    for db in &mut dbs {
        db.schemas = self.get_schemas(connection_id, &db.name)?;
        for schema in &mut db.schemas {
            schema.tables = self.get_tables_in_schema_with_conn(&conn, connection_id, &db.name, &schema.name)?;
            schema.functions = self.get_functions_for_schema(&conn, connection_id, &db.name, &schema.name)?;
            schema.sequences = self.get_sequences_for_schema(&conn, connection_id, &db.name, &schema.name)?;
        }
        if !db.schemas.is_empty() {
            db.is_introspected = true;
        }
    }
    Ok(dbs)
}
```

This requires a `get_tables_in_schema_with_conn` variant that accepts an already-locked connection (to avoid double-locking the mutex). Create it:

```rust
fn get_tables_in_schema_with_conn(&self, conn: &SqliteConnection, connection_id: &str, database: &str, schema: &str) -> Result<Vec<MetaTable>, String> {
    // Identical to get_tables_in_schema but uses provided conn.
    // Also loads constraints per table.
    let mut stmt = conn.prepare(
        "SELECT connection_id, database, schema, table_name, type, classification, last_introspected_at
         FROM meta_tables
         WHERE connection_id = ?1 AND database = ?2 AND schema = ?3
         AND table_name NOT LIKE 'pg_toast%'
         ORDER BY table_name"
    ).map_err(|e| e.to_string())?;

    let rows = stmt.query_map(params![connection_id, database, schema], |row| {
        Ok(MetaTable {
            connection_id: row.get(0)?,
            database: row.get(1)?,
            schema: row.get(2)?,
            table_name: row.get(3)?,
            table_type: row.get(4)?,
            classification: row.get(5)?,
            last_introspected_at: row.get(6)?,
            columns: vec![],
            foreign_keys: vec![],
            indexes: vec![],
            triggers: vec![],
            constraints: vec![],
        })
    }).map_err(|e| e.to_string())?;

    let mut tables: Vec<MetaTable> = rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;

    for table in &mut tables {
        table.constraints = self.get_constraints_for_table(conn, connection_id, database, schema, &table.table_name)?;
    }
    Ok(tables)
}
```

- [ ] **Step 8: Write a unit test for the save/load round-trip**

Add to the `#[cfg(test)]` block at the bottom of `introspection.rs`:

```rust
#[test]
fn test_save_and_load_functions_sequences_constraints() {
    use rusqlite::Connection as SqliteConn;
    use std::sync::{Arc, Mutex};

    let conn = SqliteConn::open_in_memory().unwrap();
    // Run migrations inline
    conn.execute_batch(include_str!("../../migrations/001_initial.sql")).unwrap();
    conn.execute_batch(include_str!("../../migrations/010_functions_sequences_constraints.sql")).unwrap();

    let app_db = Arc::new(Mutex::new(conn));
    let introspector = Introspector::new(Arc::clone(&app_db));

    // Seed database + schema + table rows so foreign keys resolve
    {
        let conn = app_db.lock().unwrap();
        conn.execute("INSERT INTO meta_databases (connection_id, name) VALUES ('c1', 'db1')", []).unwrap();
        conn.execute("INSERT INTO meta_schemas (database_id, connection_id, database, name, schema_type) VALUES (1, 'c1', 'db1', 'public', 'user')", []).unwrap();
        conn.execute("INSERT INTO meta_tables (schema_id, connection_id, database, schema, table_name, type, classification, last_introspected_at) VALUES (1, 'c1', 'db1', 'public', 'users', 'table', 'user', 0)", []).unwrap();
    }

    let conn_guard = app_db.lock().unwrap();

    // Test functions
    let funcs = vec![
        MetaFunction {
            connection_id: "c1".to_string(),
            database: "db1".to_string(),
            schema: "public".to_string(),
            name: "get_user".to_string(),
            oid: 12345,
            language: "plpgsql".to_string(),
            kind: FunctionKind::Function,
            return_type: "text".to_string(),
            arguments: vec![],
            definition: "CREATE OR REPLACE FUNCTION ...".to_string(),
            security_definer: false,
            volatility: "volatile".to_string(),
        },
        MetaFunction {
            connection_id: "c1".to_string(),
            database: "db1".to_string(),
            schema: "public".to_string(),
            name: "get_user".to_string(),
            oid: 12346, // overload
            language: "plpgsql".to_string(),
            kind: FunctionKind::Function,
            return_type: "integer".to_string(),
            arguments: vec![],
            definition: "CREATE OR REPLACE FUNCTION ...".to_string(),
            security_definer: false,
            volatility: "immutable".to_string(),
        },
    ];
    introspector.save_functions_internal(&conn_guard, 1, "c1", "db1", "public", &funcs).unwrap();
    let loaded_fns = introspector.get_functions_for_schema(&conn_guard, "c1", "db1", "public").unwrap();
    assert_eq!(loaded_fns.len(), 2, "both overloads stored");
    assert_eq!(loaded_fns[0].name, "get_user");

    // Test sequences
    let seqs = vec![MetaSequence {
        connection_id: "c1".to_string(),
        database: "db1".to_string(),
        schema: "public".to_string(),
        name: "users_id_seq".to_string(),
        data_type: "bigint".to_string(),
        start_value: 1, min_value: 1, max_value: 9223372036854775807,
        increment_by: 1, cycle: false, cache_size: 1, last_value: None,
    }];
    introspector.save_sequences_internal(&conn_guard, 1, "c1", "db1", "public", &seqs).unwrap();
    let loaded_seqs = introspector.get_sequences_for_schema(&conn_guard, "c1", "db1", "public").unwrap();
    assert_eq!(loaded_seqs.len(), 1);
    assert_eq!(loaded_seqs[0].name, "users_id_seq");

    // Test constraints
    let consts = vec![MetaConstraint {
        connection_id: "c1".to_string(),
        database: "db1".to_string(),
        schema: "public".to_string(),
        table_name: "users".to_string(),
        name: "users_pkey".to_string(),
        kind: ConstraintKind::PrimaryKey,
        definition: "PRIMARY KEY (id)".to_string(),
        columns: vec!["id".to_string()],
    }];
    introspector.save_constraints_internal(&conn_guard, 1, &consts).unwrap();
    let loaded_consts = introspector.get_constraints_for_table(&conn_guard, "c1", "db1", "public", "users").unwrap();
    assert_eq!(loaded_consts.len(), 1);
    assert_eq!(loaded_consts[0].name, "users_pkey");
    assert!(matches!(loaded_consts[0].kind, ConstraintKind::PrimaryKey));
}
```

- [ ] **Step 9: Run the test**

```bash
cd src-tauri && cargo test test_save_and_load_functions_sequences_constraints -- --nocapture
```
Expected: `test test_save_and_load_functions_sequences_constraints ... ok`

- [ ] **Step 10: Commit**

```bash
git add src-tauri/src/introspection.rs
git commit -m "feat: add save/get methods for functions, sequences, constraints in Introspector"
```

---

## Task 6: Orchestrator Level 4 Extension

**Files:**
- Modify: `src-tauri/src/orchestrator.rs`

Update imports and both `introspect_database` and `introspect_schema` to fetch and save functions, sequences, constraints.

- [ ] **Step 1: Update imports in orchestrator.rs**

Find the import of introspection types (line ~21):
```rust
use crate::introspection::{
    MetaColumn, MetaDatabase, MetaForeignKey, MetaIndex, MetaSchema, MetaTable, MetaTrigger,
};
```
Replace with:
```rust
use crate::introspection::{
    MetaColumn, MetaDatabase, MetaForeignKey, MetaIndex, MetaSchema, MetaTable, MetaTrigger,
    MetaFunction, MetaSequence, MetaConstraint,
};
```

- [ ] **Step 2: Update `introspect_database` Level 4 loop**

Find `introspect_database` (line ~389). The existing Level 4 block is:

```rust
if caps.supports_foreign_keys || caps.supports_indexes || caps.supports_triggers {
    for schema in &mut db.schemas {
        for table in &mut schema.tables {
            let table_ref = TableRef::new(...);
            if caps.supports_foreign_keys { ... }
            if caps.supports_indexes { ... }
            if caps.supports_triggers { ... }
        }
    }
    self.save_database_to_cache(&db)?;
    self.emit(IntrospectionEvent::LevelComplete { level: 4, ... });
}
```

Replace the condition and extend the body:

```rust
if caps.supports_foreign_keys || caps.supports_indexes || caps.supports_triggers || caps.supports_functions {
    for schema in &mut db.schemas {
        for table in &mut schema.tables {
            let table_ref = TableRef::new(database_name, &table.schema, &table.table_name);

            if caps.supports_foreign_keys {
                let fks = self.adapter.list_foreign_keys(&table_ref).await?;
                table.foreign_keys = fks.into_iter().map(|mut fk| { fk.connection_id = connection_id.clone(); fk }).collect();
            }

            if caps.supports_indexes {
                let indexes = self.adapter.list_indexes(&table_ref).await?;
                table.indexes = indexes.into_iter().map(|mut idx| { idx.connection_id = connection_id.clone(); idx }).collect();
            }

            if caps.supports_triggers {
                let triggers = self.adapter.list_triggers(&table_ref).await?;
                table.triggers = triggers.into_iter().map(|mut trg| { trg.connection_id = connection_id.clone(); trg }).collect();
            }

            if caps.supports_functions {
                let constraints = self.adapter.list_constraints(&table_ref).await.unwrap_or_default();
                table.constraints = constraints.into_iter().map(|mut c| { c.connection_id = connection_id.clone(); c }).collect();
            }
        }

        if caps.supports_functions {
            let functions = self.adapter.list_functions(database_name, &schema.name).await.unwrap_or_default();
            schema.functions = functions.into_iter().map(|mut f| { f.connection_id = connection_id.clone(); f }).collect();

            let sequences = self.adapter.list_sequences(database_name, &schema.name).await.unwrap_or_default();
            schema.sequences = sequences.into_iter().map(|mut s| { s.connection_id = connection_id.clone(); s }).collect();
        }
    }

    self.save_database_to_cache(&db)?;

    self.emit(IntrospectionEvent::LevelComplete {
        level: 4,
        connection_id: connection_id.clone(),
        database: Some(database_name.to_string()),
        schema_count: Some(db.schemas.len()),
        table_count: Some(db.schemas.iter().map(|s| s.tables.len()).sum()),
    });
}
```

- [ ] **Step 3: Update `introspect_schema` metadata section (around line 567)**

Find the section labeled "C. Indexes", "D. Foreign Keys", "E. Triggers". After the existing metadata, add:

```rust
// F. Constraints (per table)
let mut constraints_map: HashMap<String, Vec<MetaConstraint>> = HashMap::new();
if caps.supports_functions {
    for table in &tables {
        let table_ref = TableRef::new(database_name, schema_name, &table.table_name);
        let consts = self.adapter.list_constraints(&table_ref).await.unwrap_or_default();
        if !consts.is_empty() {
            constraints_map.insert(table.table_name.clone(), consts.into_iter().map(|mut c| { c.connection_id = connection_id.clone(); c }).collect());
        }
    }
}

// G. Functions + Sequences (schema-level)
let (schema_functions, schema_sequences) = if caps.supports_functions {
    let fns = self.adapter.list_functions(database_name, schema_name).await.unwrap_or_default();
    let seqs = self.adapter.list_sequences(database_name, schema_name).await.unwrap_or_default();
    (
        fns.into_iter().map(|mut f| { f.connection_id = connection_id.clone(); f }).collect::<Vec<_>>(),
        seqs.into_iter().map(|mut s| { s.connection_id = connection_id.clone(); s }).collect::<Vec<_>>(),
    )
} else {
    (vec![], vec![])
};
```

Then in the "Attach to tables" loop (around line 616), add:
```rust
table.constraints = constraints_map.remove(&table.table_name).unwrap_or_default();
```

And in the final schema assembly (around line 628):
```rust
let mut result_schema = target_schema.clone();
result_schema.tables = tables;
result_schema.is_introspected = true;
result_schema.functions = schema_functions;
result_schema.sequences = schema_sequences;
```

Add the counts to `metadata_count` before consuming the maps:
```rust
let constraints_total: usize = constraints_map.values().map(|v| v.len()).sum();
metadata_count += constraints_total + schema_functions.len() + schema_sequences.len();
```
(Add these lines immediately after building `constraints_map`, `schema_functions`, and `schema_sequences`, before the "Attach to tables" loop consumes `constraints_map`.)

Note: The HashMap type for `constraints_map` requires adding `MetaConstraint` to the use statement at the top of the function. The `MetaFunction`/`MetaSequence` types are already imported.

- [ ] **Step 4: Build**

```bash
cd src-tauri && cargo build 2>&1 | tail -10
```
Expected: compiles.

- [ ] **Step 5: Run existing orchestrator unit tests**

```bash
cd src-tauri && cargo test test_introspector_config test_event_serialization -- --nocapture
```
Expected: pass.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/orchestrator.rs
git commit -m "feat: Level 4 introspection now fetches functions, sequences, constraints"
```

---

## Task 7: Frontend — Types, Store, Explorer Cleanup

**Files:**
- Modify: `src/lib/commands/types.ts`
- Modify: `src/lib/components/explorer/engines/PostgresExplorer.svelte`

- [ ] **Step 1: Add new interfaces to `src/lib/commands/types.ts`**

After the `MetaIndex` interface (around line 395), add:

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

- [ ] **Step 2: Update `MetaSchema` to include `functions` and `sequences`**

Find `MetaSchema` (around line 267):
```typescript
export interface MetaSchema {
  name: string;
  schema_type: "user" | "system";
  tables: MetaTable[];
}
```
Replace with:
```typescript
export interface MetaSchema {
  name: string;
  schema_type: "user" | "system";
  tables: MetaTable[];
  functions: MetaFunction[];
  sequences: MetaSequence[];
}
```

- [ ] **Step 3: Update `MetaTable` to include `constraints`**

Find `MetaTable` (around line 273):
```typescript
export interface MetaTable {
  connection_id: string;
  database: string;
  schema: string;
  table_name: string;
  table_type: string;
  classification: string;
  last_introspected_at: number;
  columns: MetaColumn[];
  foreign_keys: MetaForeignKey[];
  indexes: MetaIndex[];
}
```
Replace with:
```typescript
export interface MetaTable {
  connection_id: string;
  database: string;
  schema: string;
  table_name: string;
  table_type: string;
  classification: string;
  last_introspected_at: number;
  columns: MetaColumn[];
  foreign_keys: MetaForeignKey[];
  indexes: MetaIndex[];
  triggers?: MetaTrigger[];
  constraints: MetaConstraint[];
}
```

Also add the missing `MetaTrigger` interface if it doesn't exist yet:
```typescript
export interface MetaTrigger {
  connection_id: string;
  database: string;
  schema: string;
  table_name: string;
  trigger_name: string;
  event: string;
  timing: string;
}
```

- [ ] **Step 4: Run svelte-check to find type errors**

```bash
cd /path/to/project && pnpm check 2>&1 | head -30
```

Fix any errors. Common ones: missing `functions`/`sequences` on objects that construct `MetaSchema` directly in tests or elsewhere.

- [ ] **Step 5: Update `PostgresExplorer.svelte` — remove lazy caches (lines 30-32 and related)**

**Remove these declarations** at the top of `<script>`:
```typescript
let functionsCache = $state<Map<string, any[]>>(new Map()); // key: "dbName:schemaName"
let sequencesCache = $state<Map<string, any[]>>(new Map());
let loadingSchemaObjects = $state<Set<string>>(new Set()); // key: "dbName:schemaName:type"
```

**Remove the `loadFunctions` function** (lines ~347–365).

**Remove the `loadSequences` function** (lines ~367–385).

- [ ] **Step 6: Update `treeData` derived — read from schemaStore instead of caches**

In `treeData` (around line 52), find:
```typescript
const cacheKey = `${db.name}:${schema.name}`;
const allFunctions = functionsCache.get(cacheKey) || [];
const functions = allFunctions.filter((f: any) => f.kind !== "Procedure");
const procedures = allFunctions.filter((f: any) => f.kind === "Procedure");
const sequences = sequencesCache.get(cacheKey) || [];
```

Replace with:
```typescript
const allFunctions = schema.functions ?? [];
const functions = allFunctions.filter((f) => f.kind !== "Procedure");
const procedures = allFunctions.filter((f) => f.kind === "Procedure");
const sequences = schema.sequences ?? [];
```

- [ ] **Step 7: Update `mapTableToNode` — read constraints from `table.constraints` not `cachedDetails.constraints`**

Find `mapTableToNode` (line ~175). The function signature is `function mapTableToNode(table: any, dbName: string, schemaName: string)`.

Change the signature to use `MetaTable` type (import it at the top):
```typescript
import type { MetaTable, MetaFunction, MetaSequence } from "$lib/commands/types";
```

In the `if (cachedDetails)` block, find:
```typescript
{
    id: `constraints:${tableId}`,
    name: "Constraints",
    type: "group" as NodeType,
    count: cachedDetails.constraints?.length || 0,
    children: (cachedDetails.constraints || []).map((c: any) => ({
```

Replace with (read from `table` directly):
```typescript
{
    id: `constraints:${tableId}`,
    name: "Constraints",
    type: "group" as NodeType,
    count: table.constraints?.length || 0,
    children: (table.constraints ?? []).map((c) => ({
```

Keep all other `cachedDetails.*` reads as-is (columns, indexes, foreign_keys, triggers still come from lazy-loaded cache).

- [ ] **Step 8: Update `handleNodeExpand` — remove lazy load calls for functions/sequences**

Find `handleNodeExpand` (line ~625). Remove the entire block:
```typescript
// NEW: lazy load schema-level objects when folder expanded
if (isOpen && node.type === "folder" && node.id) {
    if (node.id.startsWith("folder:functions:") || node.id.startsWith("folder:procedures:")) {
        const parts = node.id.split(":");
        const dbName = parts[2];
        const schemaName = parts.slice(3).join(":");
        loadFunctions(dbName, schemaName);
    } else if (node.id.startsWith("folder:sequences:")) {
        const parts = node.id.split(":");
        const dbName = parts[2];
        const schemaName = parts.slice(3).join(":");
        loadSequences(dbName, schemaName);
    }
}
```

- [ ] **Step 9: Update `handleContextMenuAction` — remove cache clears on refresh**

Find `case "refresh_schema":` (around line 612):
```typescript
case "refresh_schema": {
    await schemaStore.refresh();
    tableDetailsCache = new Map();
    functionsCache = new Map();
    sequencesCache = new Map();
    break;
}
```

Remove the `functionsCache` and `sequencesCache` lines:
```typescript
case "refresh_schema": {
    await schemaStore.refresh();
    tableDetailsCache = new Map();
    break;
}
```

- [ ] **Step 10: Update `loadTableDetails` — remove `get_constraints` live call**

Find `loadTableDetails` (line ~304). The current `Promise.all`:
```typescript
const [details, constraints] = await Promise.all([
    invoke<any>("get_schema_table_details", { ... }),
    invoke<any[]>("get_constraints", { ... }).catch(() => []),
]);
tableDetailsCache = new Map(tableDetailsCache).set(cacheKey, { ...details, constraints });
```

Replace with (constraints now come from schemaStore, not live Postgres):
```typescript
const details = await invoke<any>("get_schema_table_details", {
    connectionId: schemaStore.activeConnection?.id,
    database: dbName,
    schema: schemaName,
    tableName: tableName,
});
tableDetailsCache = new Map(tableDetailsCache).set(cacheKey, details);
```

- [ ] **Step 11: Run svelte-check again**

```bash
pnpm check 2>&1 | head -30
```
Expected: no errors (or only pre-existing unrelated ones).

- [ ] **Step 12: Commit**

```bash
git add src/lib/commands/types.ts src/lib/components/explorer/engines/PostgresExplorer.svelte
git commit -m "feat: frontend reads functions/sequences/constraints from schemaStore, removes lazy Postgres fetches"
```

---

## End-to-End Verification

After all tasks are committed, verify the full pipeline manually:

1. Start the app: `pnpm tauri dev`
2. Connect to a PostgreSQL database with functions and sequences
3. Open the schema explorer — functions, sequences folders should be populated without any manual expand-to-load
4. Expand a table — Constraints group should show data from the cache, no live Postgres call
5. Trigger a schema refresh — verify functions/sequences reload correctly

For automated integration testing:
```bash
cd src-tauri && cargo test test_pg_sprint_introspection -- --nocapture
```
This already tests the underlying SQL queries. No new integration test required for this change.

---

## Notes for Implementer

- `save_schema` in `introspection.rs` already returns `Result<i64, String>` (the schema_id). Verify this at line ~828 before using it.
- `save_functions_internal` and `save_sequences_internal` are private (`fn`, not `pub fn`) — only called from within `Introspector` methods.
- `get_functions_for_schema`, `get_sequences_for_schema`, `get_constraints_for_table` are also private.
- The existing `get_functions`, `get_sequences`, `get_constraints` Tauri commands in `introspection_commands.rs` are NOT removed — they remain for DDL context menus.
- The `arguments` field is stored as `[]` JSON for all cached functions. This is intentional — full DDL is in `definition` and retrieved by `get_function_ddl` when needed.
- The `Box<dyn DatabaseAdapter>` forwarding methods added in Task 3 ensure dynamic dispatch works correctly for downstream users of the boxed trait.
- **FK dual-store policy:** `meta_foreign_keys` (from `001_initial.sql`) remains the authoritative FK store — it has rich structural data (ref_schema, ref_table, ref_column, seq_no) and is used for the Foreign Keys tree node. `meta_constraints` also stores FK entries (kind='ForeignKey') as returned by `list_constraints`, but only for display in the Constraints folder. These serve different purposes and intentional duplication is fine. Task 4's `list_constraints` SQL includes `contype = 'f'` (ForeignKey) in results; Task 5's `get_constraints_for_table` reads from `meta_constraints` only (not `meta_foreign_keys`).
- **Migration file is `010_` not `002_`:** migrations 002–009 already exist in the repo. The file was named `010_functions_sequences_constraints.sql` to maintain sequence order.
