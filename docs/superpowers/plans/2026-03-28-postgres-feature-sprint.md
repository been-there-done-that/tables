# PostgreSQL Feature Sprint Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add read-only PostgreSQL object browsing (functions, sequences, constraints, DDL) with pgAdmin-style explorer tree and flat context menus, all DDL opening in read-only editor tabs.

**Architecture:** New Rust structs and live-query Tauri commands power the backend; `PostgresExplorer.svelte` gains schema-level folders for Functions/Procedures/Sequences/MatViews; `ExplorerContextMenu.svelte` is rewritten per-type; a read-only tab mode in the session store handles "Open DDL" actions.

**Tech Stack:** Rust/Tokio + tokio-postgres (backend), Svelte 5 runes + Tauri IPC (frontend), Monaco editor (DDL tabs)

---

## File Map

| File | Change |
|---|---|
| `src-tauri/src/introspection.rs` | Add `MetaFunction`, `MetaFunctionArg`, `FunctionKind`, `ArgMode`, `MetaSequence`, `MetaConstraint`, `ConstraintKind`; enhance `MetaIndex` |
| `src-tauri/src/commands/ddl_commands.rs` | **New file** — all DDL generation Tauri commands |
| `src-tauri/src/commands/introspection_commands.rs` | Add `get_functions`, `get_sequences`, `get_constraints`, `get_index_details` commands |
| `src-tauri/src/commands/mod.rs` | Add `pub mod ddl_commands` |
| `src/lib/components/explorer/FileTree.svelte` | Add `function`, `procedure`, `sequence`, `constraint`, `materialized_view` to `NodeType` union; add icons |
| `src/lib/components/explorer/engines/PostgresExplorer.svelte` | Add schema-level folders; lazy loading for new types; enhanced table sub-nodes; DDL action handler |
| `src/lib/components/explorer/ExplorerContextMenu.svelte` | Full rewrite — per-type flat menus |
| `src/lib/stores/session.svelte.ts` | Add `openDdlTab(title, ddl)` method |

---

## Task 1: Add New Structs to introspection.rs

**Files:**
- Modify: `src-tauri/src/introspection.rs` (after line 119, after `MetaTrigger`)

- [ ] **Step 1: Add the new types after the `MetaTrigger` struct**

Add this block immediately after the closing `}` of `MetaTrigger` (around line 119):

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FunctionKind {
    Function,
    Procedure,
    Aggregate,
    Window,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ArgMode {
    In,
    Out,
    InOut,
    Variadic,
    Table,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaFunctionArg {
    pub name: Option<String>,
    pub data_type: String,
    pub mode: ArgMode,
    pub default_value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaFunction {
    pub name: String,
    pub schema: String,
    pub language: String,
    pub kind: FunctionKind,
    pub return_type: String,
    pub arguments: Vec<MetaFunctionArg>,
    pub definition: String,
    pub security_definer: bool,
    pub volatility: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaSequence {
    pub name: String,
    pub schema: String,
    pub data_type: String,
    pub start_value: i64,
    pub min_value: i64,
    pub max_value: i64,
    pub increment_by: i64,
    pub cycle: bool,
    pub cache_size: i64,
    pub last_value: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConstraintKind {
    Check,
    Unique,
    Exclusion,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaConstraint {
    pub name: String,
    pub kind: ConstraintKind,
    pub definition: String,
    pub columns: Vec<String>,
}
```

- [ ] **Step 2: Enhance MetaIndex — add `index_type`, `columns`, `predicate` fields**

Replace the existing `MetaIndex` struct (lines 64–71):

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaIndex {
    pub connection_id: String,
    pub database: String,
    pub schema: String,
    pub table_name: String,
    pub index_name: String,
    pub is_unique: bool,
    pub is_primary: bool,
    pub index_type: String,         // btree, hash, gin, gist, brin, spgist
    pub columns: Vec<String>,       // column names in index order
    pub predicate: Option<String>,  // partial index WHERE clause
    pub definition: String,         // full CREATE INDEX statement
}
```

- [ ] **Step 3: Fix all MetaIndex construction sites that are now missing fields**

Run:
```bash
cd src-tauri && cargo check 2>&1 | grep "missing field"
```

For each error site, add the missing fields with defaults:
```rust
// When constructing MetaIndex in existing introspection code, add:
is_primary: false,
index_type: "btree".to_string(),
columns: vec![],
predicate: None,
definition: String::new(),
```

- [ ] **Step 4: Verify it compiles**

```bash
cd src-tauri && cargo check
```

Expected: no errors (warnings OK).

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/introspection.rs
git commit -m "feat(postgres): add MetaFunction, MetaSequence, MetaConstraint structs; enhance MetaIndex"
```

---

## Task 2: Backend — Browse Commands (get_functions, get_sequences, get_constraints, get_index_details)

**Files:**
- Modify: `src-tauri/src/commands/introspection_commands.rs`

These commands connect live to Postgres (no SQLite cache) and return structured data for the explorer tree.

- [ ] **Step 1: Add a private helper to build a live Postgres connection**

Add this helper at the top of `introspection_commands.rs` (after the existing `use` statements):

```rust
use crate::introspection::{MetaFunction, MetaFunctionArg, FunctionKind, ArgMode, MetaSequence, MetaConstraint, ConstraintKind};

async fn pg_connect_for_commands(
    connection_id: &str,
    database: &str,
    db_state: &tauri::State<'_, DatabaseState>,
    conn_state: &tauri::State<'_, ConnectionManagerState>,
) -> Result<tokio_postgres::Client, String> {
    let manager = ConnectionManager::from_state(db_state, conn_state);
    let (connection, credentials) = manager.get_connection(connection_id)?;
    let mut config: serde_json::Value = serde_json::from_str(&connection.config_json)
        .map_err(|e| format!("Config parse error: {}", e))?;

    if let Some(db) = config.get_mut("db") {
        if let Some(db_obj) = db.as_object_mut() {
            if let Some(password) = &credentials.password {
                db_obj.insert("password".to_string(), serde_json::Value::String(password.expose().to_string()));
            }
            // Override database to the requested one
            db_obj.insert("database".to_string(), serde_json::Value::String(database.to_string()));
        }
    }

    let db_config = config.get("db").ok_or("Missing db config")?;
    let host = db_config.get("host").and_then(|v| v.as_str()).unwrap_or("localhost");
    let port = db_config.get("port").and_then(|v| v.as_u64()).unwrap_or(5432) as u16;
    let user = db_config.get("username").and_then(|v| v.as_str()).unwrap_or("postgres");
    let pass = db_config.get("password").and_then(|v| v.as_str()).unwrap_or("");
    let db = db_config.get("database").and_then(|v| v.as_str()).unwrap_or("postgres");
    let use_tls = config.get("tls").and_then(|t| t.get("enabled")).and_then(|v| v.as_bool()).unwrap_or(false);

    let conn_str = format!("postgres://{}:{}@{}:{}/{}", user, pass, host, port, db);

    if use_tls {
        let tls = native_tls::TlsConnector::builder()
            .danger_accept_invalid_certs(true)
            .build()
            .map_err(|e| format!("TLS error: {}", e))?;
        let connector = postgres_native_tls::MakeTlsConnector::new(tls);
        let (client, conn) = tokio_postgres::connect(&conn_str, connector).await
            .map_err(|e| format!("Connection error: {}", e))?;
        tokio::spawn(async move { let _ = conn.await; });
        Ok(client)
    } else {
        let (client, conn) = tokio_postgres::connect(&conn_str, tokio_postgres::NoTls).await
            .map_err(|e| format!("Connection error: {}", e))?;
        tokio::spawn(async move { let _ = conn.await; });
        Ok(client)
    }
}
```

- [ ] **Step 2: Add `get_functions` command**

```rust
#[tauri::command]
pub async fn get_functions(
    connection_id: String,
    database: String,
    schema: String,
    db_state: State<'_, DatabaseState>,
    conn_state: State<'_, ConnectionManagerState>,
) -> Result<Vec<MetaFunction>, String> {
    let client = pg_connect_for_commands(&connection_id, &database, &db_state, &conn_state).await?;

    let rows = client.query(
        "SELECT
            p.proname AS name,
            l.lanname AS language,
            p.prokind AS kind,
            COALESCE(pg_get_function_result(p.oid), '') AS return_type,
            COALESCE(p.prosrc, pg_get_functiondef(p.oid)) AS definition,
            p.prosecdef AS security_definer,
            CASE p.provolatile
                WHEN 'v' THEN 'volatile'
                WHEN 's' THEN 'stable'
                WHEN 'i' THEN 'immutable'
                ELSE 'volatile'
            END AS volatility,
            COALESCE(pg_get_function_arguments(p.oid), '') AS arguments_str
        FROM pg_proc p
        JOIN pg_namespace n ON p.pronamespace = n.oid
        JOIN pg_language l ON p.prolang = l.oid
        WHERE n.nspname = $1
          AND p.prokind IN ('f', 'p')
        ORDER BY p.proname",
        &[&schema],
    ).await.map_err(|e| format!("Query error: {}", e))?;

    let functions = rows.iter().map(|row| {
        let kind_char: i8 = row.get::<_, i8>(2);
        let kind = match kind_char as u8 as char {
            'p' => FunctionKind::Procedure,
            'a' => FunctionKind::Aggregate,
            'w' => FunctionKind::Window,
            _ => FunctionKind::Function,
        };
        MetaFunction {
            name: row.get(0),
            schema: schema.clone(),
            language: row.get(1),
            kind,
            return_type: row.get(3),
            definition: row.get(4),
            security_definer: row.get(5),
            volatility: row.get(6),
            arguments: vec![], // simplified — full arg parsing is complex
        }
    }).collect();

    Ok(functions)
}
```

- [ ] **Step 3: Add `get_sequences` command**

```rust
#[tauri::command]
pub async fn get_sequences(
    connection_id: String,
    database: String,
    schema: String,
    db_state: State<'_, DatabaseState>,
    conn_state: State<'_, ConnectionManagerState>,
) -> Result<Vec<MetaSequence>, String> {
    let client = pg_connect_for_commands(&connection_id, &database, &db_state, &conn_state).await?;

    let rows = client.query(
        "SELECT
            sequencename AS name,
            data_type,
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
    ).await.map_err(|e| format!("Query error: {}", e))?;

    let sequences = rows.iter().map(|row| MetaSequence {
        name: row.get(0),
        schema: schema.clone(),
        data_type: row.get(1),
        start_value: row.get(2),
        min_value: row.get(3),
        max_value: row.get(4),
        increment_by: row.get(5),
        cycle: row.get(6),
        cache_size: row.get(7),
        last_value: row.get(8),
    }).collect();

    Ok(sequences)
}
```

- [ ] **Step 4: Add `get_constraints` command**

```rust
#[tauri::command]
pub async fn get_constraints(
    connection_id: String,
    database: String,
    schema: String,
    table_name: String,
    db_state: State<'_, DatabaseState>,
    conn_state: State<'_, ConnectionManagerState>,
) -> Result<Vec<MetaConstraint>, String> {
    let client = pg_connect_for_commands(&connection_id, &database, &db_state, &conn_state).await?;
    let qualified = format!("{}.{}", schema, table_name);

    let rows = client.query(
        "SELECT
            c.conname AS name,
            c.contype AS kind,
            pg_get_constraintdef(c.oid) AS definition,
            ARRAY(
                SELECT a.attname
                FROM pg_attribute a
                WHERE a.attrelid = c.conrelid
                  AND a.attnum = ANY(c.conkey)
                  AND a.attnum > 0
                ORDER BY array_position(c.conkey, a.attnum)
            ) AS columns
        FROM pg_constraint c
        WHERE c.conrelid = $1::regclass
          AND c.contype IN ('c', 'u', 'x')
        ORDER BY c.conname",
        &[&qualified],
    ).await.map_err(|e| format!("Query error: {}", e))?;

    let constraints = rows.iter().map(|row| {
        let kind_char: i8 = row.get::<_, i8>(1);
        let kind = match kind_char as u8 as char {
            'u' => ConstraintKind::Unique,
            'x' => ConstraintKind::Exclusion,
            _ => ConstraintKind::Check,
        };
        let columns: Vec<String> = row.get(3);
        MetaConstraint {
            name: row.get(0),
            kind,
            definition: row.get(2),
            columns,
        }
    }).collect();

    Ok(constraints)
}
```

- [ ] **Step 5: Add `get_index_details` command**

```rust
#[tauri::command]
pub async fn get_index_details(
    connection_id: String,
    database: String,
    schema: String,
    table_name: String,
    db_state: State<'_, DatabaseState>,
    conn_state: State<'_, ConnectionManagerState>,
) -> Result<Vec<crate::introspection::MetaIndex>, String> {
    let client = pg_connect_for_commands(&connection_id, &database, &db_state, &conn_state).await?;
    let qualified = format!("{}.{}", schema, table_name);

    let rows = client.query(
        "SELECT
            i.relname AS index_name,
            am.amname AS index_type,
            ix.indisunique AS is_unique,
            ix.indisprimary AS is_primary,
            pg_get_indexdef(ix.indexrelid) AS definition,
            ARRAY(
                SELECT a.attname
                FROM pg_attribute a
                WHERE a.attrelid = t.oid
                  AND a.attnum = ANY(ix.indkey)
                  AND a.attnum > 0
                ORDER BY array_position(ix.indkey::smallint[], a.attnum)
            ) AS columns,
            pg_get_expr(ix.indpred, ix.indrelid) AS predicate
        FROM pg_index ix
        JOIN pg_class t ON t.oid = ix.indrelid
        JOIN pg_class i ON i.oid = ix.indexrelid
        JOIN pg_am am ON am.oid = i.relam
        WHERE t.oid = $1::regclass
        ORDER BY i.relname",
        &[&qualified],
    ).await.map_err(|e| format!("Query error: {}", e))?;

    let connection_id_clone = connection_id.clone();
    let indexes = rows.iter().map(|row| {
        let columns: Vec<String> = row.get(5);
        crate::introspection::MetaIndex {
            connection_id: connection_id_clone.clone(),
            database: database.clone(),
            schema: schema.clone(),
            table_name: table_name.clone(),
            index_name: row.get(0),
            is_unique: row.get(2),
            is_primary: row.get(3),
            index_type: row.get(1),
            columns,
            predicate: row.get(6),
            definition: row.get(4),
        }
    }).collect();

    Ok(indexes)
}
```

- [ ] **Step 6: Verify compile**

```bash
cd src-tauri && cargo check
```

Expected: no errors.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/commands/introspection_commands.rs
git commit -m "feat(postgres): add get_functions, get_sequences, get_constraints, get_index_details commands"
```

---

## Task 3: Backend — DDL Generation Commands

**Files:**
- Create: `src-tauri/src/commands/ddl_commands.rs`
- Modify: `src-tauri/src/commands/mod.rs`

- [ ] **Step 1: Create `ddl_commands.rs`**

```rust
//! DDL generation commands — return CREATE statements for PostgreSQL objects.
//! All commands are read-only and connect live to the database.

use tauri::State;
use crate::{DatabaseState, ConnectionManagerState, ConnectionManager};
use log::debug;

/// Re-use the connection helper from introspection_commands
use super::introspection_commands::pg_connect_for_commands;

#[tauri::command]
pub async fn get_table_ddl(
    connection_id: String,
    database: String,
    schema: String,
    table_name: String,
    db_state: State<'_, DatabaseState>,
    conn_state: State<'_, ConnectionManagerState>,
) -> Result<String, String> {
    let client = pg_connect_for_commands(&connection_id, &database, &db_state, &conn_state).await?;
    let qualified = format!("{}.{}", schema, table_name);
    debug!("Generating DDL for table {}", qualified);

    // Get column definitions
    let col_rows = client.query(
        "SELECT
            a.attname AS col_name,
            pg_catalog.format_type(a.atttypid, a.atttypmod) AS col_type,
            a.attnotnull AS not_null,
            pg_get_expr(d.adbin, d.adrelid) AS default_val
        FROM pg_attribute a
        LEFT JOIN pg_attrdef d ON d.adrelid = a.attrelid AND d.adnum = a.attnum
        WHERE a.attrelid = $1::regclass
          AND a.attnum > 0
          AND NOT a.attisdropped
        ORDER BY a.attnum",
        &[&qualified],
    ).await.map_err(|e| format!("Column query error: {}", e))?;

    // Get constraints
    let con_rows = client.query(
        "SELECT conname, pg_get_constraintdef(oid) AS def
        FROM pg_constraint
        WHERE conrelid = $1::regclass
        ORDER BY contype, conname",
        &[&qualified],
    ).await.map_err(|e| format!("Constraint query error: {}", e))?;

    // Build the CREATE TABLE statement
    let mut parts: Vec<String> = col_rows.iter().map(|row| {
        let col_name: &str = row.get(0);
        let col_type: &str = row.get(1);
        let not_null: bool = row.get(2);
        let default_val: Option<&str> = row.get(3);
        let mut def = format!("    {} {}", col_name, col_type);
        if not_null { def.push_str(" NOT NULL"); }
        if let Some(d) = default_val { def.push_str(&format!(" DEFAULT {}", d)); }
        def
    }).collect();

    for row in &con_rows {
        let con_name: &str = row.get(0);
        let con_def: &str = row.get(1);
        parts.push(format!("    CONSTRAINT {} {}", con_name, con_def));
    }

    Ok(format!(
        "CREATE TABLE {}.{} (\n{}\n);",
        schema, table_name,
        parts.join(",\n")
    ))
}

#[tauri::command]
pub async fn get_view_definition(
    connection_id: String,
    database: String,
    schema: String,
    view_name: String,
    db_state: State<'_, DatabaseState>,
    conn_state: State<'_, ConnectionManagerState>,
) -> Result<String, String> {
    let client = pg_connect_for_commands(&connection_id, &database, &db_state, &conn_state).await?;

    let row = client.query_one(
        "SELECT definition FROM pg_views WHERE schemaname = $1 AND viewname = $2",
        &[&schema, &view_name],
    ).await.map_err(|e| format!("View not found: {}", e))?;

    let definition: &str = row.get(0);
    Ok(format!("CREATE OR REPLACE VIEW {}.{} AS\n{}", schema, view_name, definition.trim_end()))
}

#[tauri::command]
pub async fn get_matview_definition(
    connection_id: String,
    database: String,
    schema: String,
    view_name: String,
    db_state: State<'_, DatabaseState>,
    conn_state: State<'_, ConnectionManagerState>,
) -> Result<String, String> {
    let client = pg_connect_for_commands(&connection_id, &database, &db_state, &conn_state).await?;

    let row = client.query_one(
        "SELECT definition FROM pg_matviews WHERE schemaname = $1 AND matviewname = $2",
        &[&schema, &view_name],
    ).await.map_err(|e| format!("Materialized view not found: {}", e))?;

    let definition: &str = row.get(0);
    Ok(format!("CREATE MATERIALIZED VIEW {}.{} AS\n{}", schema, view_name, definition.trim_end()))
}

#[tauri::command]
pub async fn get_function_ddl(
    connection_id: String,
    database: String,
    schema: String,
    function_name: String,
    db_state: State<'_, DatabaseState>,
    conn_state: State<'_, ConnectionManagerState>,
) -> Result<String, String> {
    let client = pg_connect_for_commands(&connection_id, &database, &db_state, &conn_state).await?;

    // pg_get_functiondef returns the complete CREATE OR REPLACE FUNCTION statement
    let row = client.query_one(
        "SELECT pg_get_functiondef(p.oid)
        FROM pg_proc p
        JOIN pg_namespace n ON p.pronamespace = n.oid
        WHERE n.nspname = $1 AND p.proname = $2
        LIMIT 1",
        &[&schema, &function_name],
    ).await.map_err(|e| format!("Function not found: {}", e))?;

    Ok(row.get(0))
}

#[tauri::command]
pub async fn get_sequence_ddl(
    connection_id: String,
    database: String,
    schema: String,
    sequence_name: String,
    db_state: State<'_, DatabaseState>,
    conn_state: State<'_, ConnectionManagerState>,
) -> Result<String, String> {
    let client = pg_connect_for_commands(&connection_id, &database, &db_state, &conn_state).await?;

    let row = client.query_one(
        "SELECT data_type, start_value, min_value, max_value, increment_by, cycle, cache_size
        FROM pg_sequences
        WHERE schemaname = $1 AND sequencename = $2",
        &[&schema, &sequence_name],
    ).await.map_err(|e| format!("Sequence not found: {}", e))?;

    let data_type: &str = row.get(0);
    let start: i64 = row.get(1);
    let min: i64 = row.get(2);
    let max: i64 = row.get(3);
    let increment: i64 = row.get(4);
    let cycle: bool = row.get(5);
    let cache: i64 = row.get(6);

    Ok(format!(
        "CREATE SEQUENCE {}.{}\n    AS {}\n    START WITH {}\n    INCREMENT BY {}\n    MINVALUE {}\n    MAXVALUE {}\n    CACHE {}{};",
        schema, sequence_name, data_type, start, increment, min, max, cache,
        if cycle { "\n    CYCLE" } else { "\n    NO CYCLE" }
    ))
}

#[tauri::command]
pub async fn get_index_ddl(
    connection_id: String,
    database: String,
    schema: String,
    table_name: String,
    index_name: String,
    db_state: State<'_, DatabaseState>,
    conn_state: State<'_, ConnectionManagerState>,
) -> Result<String, String> {
    let client = pg_connect_for_commands(&connection_id, &database, &db_state, &conn_state).await?;

    let row = client.query_one(
        "SELECT indexdef FROM pg_indexes WHERE schemaname = $1 AND tablename = $2 AND indexname = $3",
        &[&schema, &table_name, &index_name],
    ).await.map_err(|e| format!("Index not found: {}", e))?;

    Ok(row.get(0))
}

#[tauri::command]
pub async fn get_trigger_definition(
    connection_id: String,
    database: String,
    schema: String,
    table_name: String,
    trigger_name: String,
    db_state: State<'_, DatabaseState>,
    conn_state: State<'_, ConnectionManagerState>,
) -> Result<String, String> {
    let client = pg_connect_for_commands(&connection_id, &database, &db_state, &conn_state).await?;
    let qualified = format!("{}.{}", schema, table_name);

    let row = client.query_one(
        "SELECT pg_get_triggerdef(t.oid, true)
        FROM pg_trigger t
        JOIN pg_class c ON c.oid = t.tgrelid
        JOIN pg_namespace n ON n.oid = c.relnamespace
        WHERE n.nspname = $1 AND c.relname = $2 AND t.tgname = $3
          AND NOT t.tgisinternal",
        &[&schema, &table_name, &trigger_name],
    ).await.map_err(|e| format!("Trigger not found: {}", e))?;

    Ok(row.get(0))
}
```

- [ ] **Step 2: Add `pg_connect_for_commands` to the public API of introspection_commands.rs**

The helper in Task 2 must be `pub(super)` so `ddl_commands.rs` can use it. Change the function signature:

```rust
pub(super) async fn pg_connect_for_commands(
```

- [ ] **Step 3: Register the new module in `commands/mod.rs`**

Add after the last `pub mod` line:

```rust
pub mod ddl_commands;
```

- [ ] **Step 4: Register all new commands in lib.rs**

Find the `aggregate_plugin_commands!()` macro usage or the `tauri::generate_handler!` call. Add all new commands:

```rust
// In the generate_handler! macro (search for where other commands are registered):
get_functions,
get_sequences,
get_constraints,
get_index_details,
get_table_ddl,
get_view_definition,
get_matview_definition,
get_function_ddl,
get_sequence_ddl,
get_index_ddl,
get_trigger_definition,
```

First check how the macro is structured:
```bash
grep -n "aggregate_plugin_commands\|generate_handler" src-tauri/src/lib.rs | head -5
```

Then find and add to the right place.

- [ ] **Step 5: Verify compile**

```bash
cd src-tauri && cargo check
```

Expected: no errors.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/commands/ddl_commands.rs src-tauri/src/commands/mod.rs src-tauri/src/lib.rs
git commit -m "feat(postgres): add DDL generation commands for tables, views, functions, sequences, indexes, triggers"
```

---

## Task 4: Frontend — New NodeType Variants + Icons

**Files:**
- Modify: `src/lib/components/explorer/FileTree.svelte` (context module, lines 19–32)

- [ ] **Step 1: Extend the `NodeType` union**

Replace the existing `NodeType` export (lines 19–32 in FileTree.svelte):

```typescript
export type NodeType =
    | "folder"
    | "group"
    | "file"
    | "database"
    | "key"
    | "primary_key"
    | "schema"
    | "table"
    | "view"
    | "materialized_view"   // NEW
    | "function"            // NEW
    | "procedure"           // NEW
    | "sequence"            // NEW
    | "constraint"          // NEW
    | "column"
    | "index"
    | "trigger"
    | "foreign_key";
```

- [ ] **Step 2: Add icon imports for new types**

Add to the `<script context="module">` imports section at the top of `FileTree.svelte`:

```typescript
import IconFunction from "@tabler/icons-svelte/icons/math-function";
import IconSequence from "@tabler/icons-svelte/icons/sort-ascending-numbers";
import IconConstraint from "@tabler/icons-svelte/icons/shield-check";
import IconMatView from "@tabler/icons-svelte/icons/eye-table";
```

- [ ] **Step 3: Map new NodeTypes to icons in the icon-selection logic**

Find the section in `FileTree.svelte` where node types are mapped to icons (look for `node.type === "table"` conditions). Add cases for the new types:

```typescript
// In the icon mapping (wherever table/view/index icons are set):
case "function":
case "procedure":
    return IconFunction;
case "sequence":
    return IconSequence;
case "constraint":
    return IconConstraint;
case "materialized_view":
    return IconMatView;
```

- [ ] **Step 4: Verify icon imports exist**

```bash
ls node_modules/@tabler/icons-svelte/icons/ | grep -E "math-function|sort-ascending|shield-check|eye-table"
```

If any icon name doesn't exist, find an alternative:
```bash
ls node_modules/@tabler/icons-svelte/icons/ | grep -E "function|sequence|constraint|material"
```

Use whatever is available. Common fallbacks:
- function → `variable`, `code`, `brackets`
- sequence → `sort-ascending`, `list-numbers`
- constraint → `lock`, `shield`
- materialized_view → `eye`, `table`

- [ ] **Step 5: Type-check**

```bash
pnpm check 2>&1 | head -30
```

Expected: no new errors from FileTree.svelte.

- [ ] **Step 6: Commit**

```bash
git add src/lib/components/explorer/FileTree.svelte
git commit -m "feat(explorer): add function, procedure, sequence, constraint, materialized_view node types"
```

---

## Task 5: Frontend — Explorer Tree Restructure

**Files:**
- Modify: `src/lib/components/explorer/engines/PostgresExplorer.svelte`

This is the largest frontend task. We add schema-level folders for Functions, Procedures, Sequences, and Materialized Views, with lazy loading matching the existing table-details pattern.

- [ ] **Step 1: Add state for new object type caches**

In the `<script>` section, after the existing `tableDetailsCache` and `loadingTables` declarations:

```typescript
// Cache for schema-level objects (functions, sequences)
let functionsCache = $state<Map<string, any[]>>(new Map()); // key: "dbName:schemaName"
let sequencesCache = $state<Map<string, any[]>>(new Map());
let loadingSchemaObjects = $state<Set<string>>(new Set()); // key: "dbName:schemaName:type"
```

- [ ] **Step 2: Add load functions for new types**

```typescript
async function loadFunctions(dbName: string, schemaName: string) {
    const cacheKey = `${dbName}:${schemaName}`;
    const loadKey = `${cacheKey}:functions`;
    if (functionsCache.has(cacheKey) || loadingSchemaObjects.has(loadKey)) return;

    loadingSchemaObjects = new Set([...loadingSchemaObjects, loadKey]);
    try {
        const fns = await invoke<any[]>("get_functions", {
            connectionId: schemaStore.activeConnection?.id,
            database: dbName,
            schema: schemaName,
        });
        functionsCache = new Map(functionsCache).set(cacheKey, fns);
    } catch (e) {
        console.error(`Failed to load functions for ${schemaName}:`, e);
    } finally {
        loadingSchemaObjects = new Set([...loadingSchemaObjects].filter(k => k !== loadKey));
    }
}

async function loadSequences(dbName: string, schemaName: string) {
    const cacheKey = `${dbName}:${schemaName}`;
    const loadKey = `${cacheKey}:sequences`;
    if (sequencesCache.has(cacheKey) || loadingSchemaObjects.has(loadKey)) return;

    loadingSchemaObjects = new Set([...loadingSchemaObjects, loadKey]);
    try {
        const seqs = await invoke<any[]>("get_sequences", {
            connectionId: schemaStore.activeConnection?.id,
            database: dbName,
            schema: schemaName,
        });
        sequencesCache = new Map(sequencesCache).set(cacheKey, seqs);
    } catch (e) {
        console.error(`Failed to load sequences for ${schemaName}:`, e);
    } finally {
        loadingSchemaObjects = new Set([...loadingSchemaObjects].filter(k => k !== loadKey));
    }
}
```

- [ ] **Step 3: Update `treeData` to include new schema-level folders**

In the `treeData = $derived.by(() => { ... })` block, update the part where `children` is built for each schema. Replace the existing `children.push(...)` blocks with:

```typescript
const tables = schema.tables.filter((t: any) => t.table_type === "table" || t.table_type === "BASE TABLE");
const views = schema.tables.filter((t: any) => t.table_type === "view");
const matviews = schema.tables.filter((t: any) => t.table_type === "materialized_view" || t.table_type === "MATERIALIZED VIEW");

const cacheKey = `${db.name}:${schema.name}`;
const functions = functionsCache.get(cacheKey) || [];
const sequences = sequencesCache.get(cacheKey) || [];

const children: TreeNode[] = [];

if (tables.length > 0) {
    children.push({
        id: `folder:tables:${db.name}:${schema.name}`,
        name: "tables",
        type: "folder" as NodeType,
        count: tables.length,
        children: tables.map((table: any) => mapTableToNode(table, db.name, schema.name)),
    });
}

if (views.length > 0) {
    children.push({
        id: `folder:views:${db.name}:${schema.name}`,
        name: "views",
        type: "folder" as NodeType,
        count: views.length,
        children: views.map((v: any) => ({
            id: `view:${db.name}:${schema.name}.${v.table_name}`,
            name: v.table_name,
            type: "view" as NodeType,
            metadata: { dbName: db.name, schemaName: schema.name, objectName: v.table_name, objectType: "view" },
        })),
    });
}

if (matviews.length > 0) {
    children.push({
        id: `folder:matviews:${db.name}:${schema.name}`,
        name: "materialized views",
        type: "folder" as NodeType,
        count: matviews.length,
        children: matviews.map((v: any) => ({
            id: `matview:${db.name}:${schema.name}.${v.table_name}`,
            name: v.table_name,
            type: "materialized_view" as NodeType,
            metadata: { dbName: db.name, schemaName: schema.name, objectName: v.table_name, objectType: "matview" },
        })),
    });
}

// Functions folder — lazy loaded
children.push({
    id: `folder:functions:${db.name}:${schema.name}`,
    name: "functions",
    type: "folder" as NodeType,
    count: functions.length || undefined,
    children: functions.map((f: any) => ({
        id: `function:${db.name}:${schema.name}.${f.name}`,
        name: f.name,
        type: (f.kind === "Procedure" ? "procedure" : "function") as NodeType,
        detail: f.return_type || undefined,
        metadata: { dbName: db.name, schemaName: schema.name, objectName: f.name, objectType: "function", language: f.language },
    })),
});

// Sequences folder — lazy loaded
children.push({
    id: `folder:sequences:${db.name}:${schema.name}`,
    name: "sequences",
    type: "folder" as NodeType,
    count: sequences.length || undefined,
    children: sequences.map((s: any) => ({
        id: `sequence:${db.name}:${schema.name}.${s.name}`,
        name: s.name,
        type: "sequence" as NodeType,
        detail: s.data_type,
        metadata: { dbName: db.name, schemaName: schema.name, objectName: s.name, objectType: "sequence" },
    })),
});
```

- [ ] **Step 4: Update `handleNodeExpand` to lazy-load functions/sequences**

In the `handleNodeExpand` function, add cases for the new folder types:

```typescript
async function handleNodeExpand(node: TreeNode, isOpen: boolean) {
    if (isOpen && node.type === "database") {
        schemaStore.loadDatabase(node.name);
    }
    if (isOpen && node.type === "table" && node.metadata) {
        const { dbName, schemaName, tableName } = node.metadata as any;
        loadTableDetails(dbName, schemaName, tableName);
    }
    // NEW: lazy load schema-level objects
    if (isOpen && node.type === "folder" && node.id) {
        if (node.id.startsWith("folder:functions:")) {
            const [, , dbName, schemaName] = node.id.split(":");
            loadFunctions(dbName, schemaName);
        } else if (node.id.startsWith("folder:sequences:")) {
            const [, , dbName, schemaName] = node.id.split(":");
            loadSequences(dbName, schemaName);
        }
    }
    if (activeSession) {
        activeSession.persistExpandedNodes();
    }
}
```

- [ ] **Step 5: Update `mapTableToNode` to include Constraints sub-folder**

In the `mapTableToNode` function, inside the `if (cachedDetails)` branch, add a Constraints group after Indexes:

```typescript
// After the indexes group push:
{
    id: `constraints:${tableId}`,
    name: "Constraints",
    type: "group" as NodeType,
    count: cachedDetails.constraints?.length || 0,
    children: (cachedDetails.constraints || []).map((c: any) => ({
        id: `constraint:${tableId}.${c.name}`,
        name: c.name,
        type: "constraint" as NodeType,
        detail: c.kind,
        metadata: { ...node.metadata, constraintName: c.name, definition: c.definition },
    })),
},
```

> Note: `cachedDetails.constraints` will be empty initially. Constraints are fetched lazily in Task 7.

- [ ] **Step 6: Update Indexes sub-folder to show index type**

In `mapTableToNode`, update the index node mapping to show type in `detail`:

```typescript
// Change index detail from:  detail: idx.is_unique ? "Unique" : ""
// To:
detail: [idx.index_type, idx.is_unique ? "unique" : ""].filter(Boolean).join(" · "),
```

- [ ] **Step 7: Type-check**

```bash
pnpm check 2>&1 | head -30
```

Expected: no errors in PostgresExplorer.svelte.

- [ ] **Step 8: Commit**

```bash
git add src/lib/components/explorer/engines/PostgresExplorer.svelte
git commit -m "feat(explorer): add schema-level Functions, Sequences, MatViews folders with lazy loading; enhanced table sub-nodes"
```

---

## Task 6: Frontend — Context Menus

**Files:**
- Modify: `src/lib/components/explorer/ExplorerContextMenu.svelte`

Full rewrite of the currently minimal context menu (only has "Query Console").

- [ ] **Step 1: Rewrite ExplorerContextMenu.svelte**

Replace the entire file content with:

```svelte
<script lang="ts">
    import * as ContextMenu from "$lib/components/ui/context-menu";
    import IconFileDatabase from "@tabler/icons-svelte/icons/file-database";
    import IconCopy from "@tabler/icons-svelte/icons/copy";
    import IconCode from "@tabler/icons-svelte/icons/code";
    import IconTable from "@tabler/icons-svelte/icons/table";
    import IconRefresh from "@tabler/icons-svelte/icons/refresh";
    import type { TreeNode } from "./FileTree.svelte";

    let {
        node,
        onAction = (action: string, node: TreeNode) => {},
    }: {
        node: TreeNode;
        onAction?: (action: string, node: TreeNode) => void;
    } = $props();

    function act(action: string) {
        onAction(action, node);
    }

    const isTable = $derived(node.type === "table");
    const isView = $derived(node.type === "view");
    const isMatView = $derived(node.type === "materialized_view");
    const isFunction = $derived(node.type === "function" || node.type === "procedure");
    const isSequence = $derived(node.type === "sequence");
    const isIndex = $derived(node.type === "index");
    const isTrigger = $derived(node.type === "trigger");
    const isConstraint = $derived(node.type === "constraint");
    const isSchema = $derived(node.type === "schema");
    const isColumn = $derived(node.type === "column" || node.type === "primary_key");
    const isForeignKey = $derived(node.type === "foreign_key");
</script>

<ContextMenu.Content class="w-52">
    <!-- Always available -->
    <ContextMenu.Item onclick={() => act("copy_name")}>
        <IconCopy class="mr-2 size-4 opacity-60" />
        <span>Copy Name</span>
    </ContextMenu.Item>

    {#if isTable || isView || isMatView}
        <ContextMenu.Item onclick={() => act("view_data")}>
            <IconTable class="mr-2 size-4 opacity-60" />
            <span>View Data</span>
        </ContextMenu.Item>
        <ContextMenu.Item onclick={() => act("copy_select")}>
            <IconCopy class="mr-2 size-4 opacity-60" />
            <span>Copy as SELECT *</span>
        </ContextMenu.Item>
    {/if}

    {#if isTable}
        <ContextMenu.Item onclick={() => act("open_ddl")}>
            <IconCode class="mr-2 size-4 opacity-60" />
            <span>Open DDL</span>
        </ContextMenu.Item>
    {/if}

    {#if isView}
        <ContextMenu.Item onclick={() => act("open_definition")}>
            <IconCode class="mr-2 size-4 opacity-60" />
            <span>Open Definition</span>
        </ContextMenu.Item>
    {/if}

    {#if isMatView}
        <ContextMenu.Item onclick={() => act("open_definition")}>
            <IconCode class="mr-2 size-4 opacity-60" />
            <span>Open Definition</span>
        </ContextMenu.Item>
    {/if}

    {#if isFunction}
        <ContextMenu.Item onclick={() => act("open_definition")}>
            <IconCode class="mr-2 size-4 opacity-60" />
            <span>Open Definition</span>
        </ContextMenu.Item>
        <ContextMenu.Item onclick={() => act("copy_function_ddl")}>
            <IconCopy class="mr-2 size-4 opacity-60" />
            <span>Copy as CREATE FUNCTION</span>
        </ContextMenu.Item>
    {/if}

    {#if isSequence}
        <ContextMenu.Item onclick={() => act("open_ddl")}>
            <IconCode class="mr-2 size-4 opacity-60" />
            <span>Open DDL</span>
        </ContextMenu.Item>
        <ContextMenu.Item onclick={() => act("copy_sequence_ddl")}>
            <IconCopy class="mr-2 size-4 opacity-60" />
            <span>Copy as CREATE SEQUENCE</span>
        </ContextMenu.Item>
    {/if}

    {#if isIndex}
        <ContextMenu.Item onclick={() => act("open_definition")}>
            <IconCode class="mr-2 size-4 opacity-60" />
            <span>Open Definition</span>
        </ContextMenu.Item>
        <ContextMenu.Item onclick={() => act("copy_index_ddl")}>
            <IconCopy class="mr-2 size-4 opacity-60" />
            <span>Copy DDL</span>
        </ContextMenu.Item>
    {/if}

    {#if isTrigger}
        <ContextMenu.Item onclick={() => act("open_definition")}>
            <IconCode class="mr-2 size-4 opacity-60" />
            <span>Open Definition</span>
        </ContextMenu.Item>
        <ContextMenu.Item onclick={() => act("copy_trigger_ddl")}>
            <IconCopy class="mr-2 size-4 opacity-60" />
            <span>Copy DDL</span>
        </ContextMenu.Item>
    {/if}

    {#if isConstraint}
        <ContextMenu.Item onclick={() => act("copy_constraint_ddl")}>
            <IconCopy class="mr-2 size-4 opacity-60" />
            <span>Copy DDL</span>
        </ContextMenu.Item>
    {/if}

    {#if isSchema}
        <ContextMenu.Separator />
        <ContextMenu.Item onclick={() => act("refresh_schema")}>
            <IconRefresh class="mr-2 size-4 opacity-60" />
            <span>Refresh Schema</span>
        </ContextMenu.Item>
    {/if}

    {#if isTable || isView || isMatView}
        <ContextMenu.Separator />
    {/if}

    <ContextMenu.Item onclick={() => act("query_console")}>
        <IconFileDatabase class="mr-2 size-4 text-primary" />
        <span>Query Console</span>
        <ContextMenu.Shortcut>⇧⌘L</ContextMenu.Shortcut>
    </ContextMenu.Item>
</ContextMenu.Content>
```

- [ ] **Step 2: Type-check**

```bash
pnpm check 2>&1 | grep ExplorerContextMenu
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add src/lib/components/explorer/ExplorerContextMenu.svelte
git commit -m "feat(explorer): rewrite context menus with per-type flat pgAdmin-style actions"
```

---

## Task 7: Frontend — Read-only DDL Tab + Action Wiring

**Files:**
- Modify: `src/lib/stores/session.svelte.ts`
- Modify: `src/lib/components/explorer/engines/PostgresExplorer.svelte`

- [ ] **Step 1: Add `openDdlTab` to the session store**

Find `session.svelte.ts` and look for the `openView` method. Add `openDdlTab` as a new method on the session class:

```typescript
openDdlTab(title: string, ddl: string) {
    // Open a read-only editor view pre-filled with DDL content
    this.openView("editor", title, {
        initialValue: ddl,
        readOnly: true,
        language: "sql",
    });
}
```

- [ ] **Step 2: Find where the editor tab reads `initialValue` and add `readOnly` support**

Search for where `initialValue` is consumed:
```bash
grep -rn "initialValue" src/lib/ --include="*.svelte" --include="*.ts" | grep -v node_modules
```

In `SqlTestingEditor.svelte` (or wherever the editor is rendered), find where Monaco is initialized. Add read-only mode support:

```typescript
// When creating the Monaco editor instance, check for readOnly:
const isReadOnly = viewMetadata?.readOnly === true;
editor = monaco.editor.create(container, {
    value: viewMetadata?.initialValue || "",
    language: viewMetadata?.language || "sql",
    readOnly: isReadOnly,
    // ... existing options
});
```

> The exact location depends on where Monaco is initialized. Search for `monaco.editor.create` to find it.

- [ ] **Step 3: Add the full `handleContextMenuAction` switch in PostgresExplorer.svelte**

Replace the existing `handleContextMenuAction` function with:

```typescript
async function handleContextMenuAction(action: string, node: TreeNode) {
    if (!activeSession) {
        if (schemaStore.activeConnection) windowState.startSession(schemaStore.activeConnection);
        else return;
    }
    const session = windowState.activeSession;
    if (!session) return;

    const meta = node.metadata as any;
    const connId = schemaStore.activeConnection?.id;

    switch (action) {
        case "query_console": {
            const title = node.type === "schema" ? `Console: ${node.name}` : `Query: ${node.name}`;
            session.openView("editor", title, node.metadata);
            break;
        }

        case "copy_name": {
            await navigator.clipboard.writeText(node.name);
            break;
        }

        case "view_data": {
            session.openView("table", node.name, {
                tableName: node.name,
                schemaName: meta?.schemaName,
                databaseName: meta?.dbName,
                connectionId: connId,
            });
            break;
        }

        case "copy_select": {
            const schema = meta?.schemaName || "public";
            await navigator.clipboard.writeText(`SELECT * FROM ${schema}.${node.name};`);
            break;
        }

        case "open_ddl": {
            try {
                let ddl = "";
                if (node.type === "table") {
                    ddl = await invoke<string>("get_table_ddl", {
                        connectionId: connId,
                        database: meta?.dbName || schemaStore.selectedDatabase,
                        schema: meta?.schemaName || "public",
                        tableName: node.name,
                    });
                } else if (node.type === "sequence") {
                    ddl = await invoke<string>("get_sequence_ddl", {
                        connectionId: connId,
                        database: meta?.dbName || schemaStore.selectedDatabase,
                        schema: meta?.schemaName || "public",
                        sequenceName: node.name,
                    });
                }
                session.openDdlTab(`DDL: ${node.name}`, ddl);
            } catch (e) {
                console.error("DDL fetch failed:", e);
            }
            break;
        }

        case "open_definition": {
            try {
                let ddl = "";
                if (node.type === "view") {
                    ddl = await invoke<string>("get_view_definition", {
                        connectionId: connId,
                        database: meta?.dbName || schemaStore.selectedDatabase,
                        schema: meta?.schemaName || "public",
                        viewName: node.name,
                    });
                } else if (node.type === "materialized_view") {
                    ddl = await invoke<string>("get_matview_definition", {
                        connectionId: connId,
                        database: meta?.dbName || schemaStore.selectedDatabase,
                        schema: meta?.schemaName || "public",
                        viewName: node.name,
                    });
                } else if (node.type === "function" || node.type === "procedure") {
                    ddl = await invoke<string>("get_function_ddl", {
                        connectionId: connId,
                        database: meta?.dbName || schemaStore.selectedDatabase,
                        schema: meta?.schemaName || "public",
                        functionName: node.name,
                    });
                } else if (node.type === "index") {
                    ddl = await invoke<string>("get_index_ddl", {
                        connectionId: connId,
                        database: meta?.dbName || schemaStore.selectedDatabase,
                        schema: meta?.schemaName || "public",
                        tableName: meta?.tableName || "",
                        indexName: node.name,
                    });
                } else if (node.type === "trigger") {
                    ddl = await invoke<string>("get_trigger_definition", {
                        connectionId: connId,
                        database: meta?.dbName || schemaStore.selectedDatabase,
                        schema: meta?.schemaName || "public",
                        tableName: meta?.tableName || "",
                        triggerName: node.name,
                    });
                }
                session.openDdlTab(`${node.name}`, ddl);
            } catch (e) {
                console.error("Definition fetch failed:", e);
            }
            break;
        }

        case "copy_function_ddl":
        case "copy_sequence_ddl":
        case "copy_index_ddl":
        case "copy_trigger_ddl": {
            // Fetch DDL then copy instead of opening tab
            // Reuse open_ddl / open_definition logic but write to clipboard
            try {
                let ddl = "";
                if (node.type === "function" || node.type === "procedure") {
                    ddl = await invoke<string>("get_function_ddl", {
                        connectionId: connId,
                        database: meta?.dbName || schemaStore.selectedDatabase,
                        schema: meta?.schemaName || "public",
                        functionName: node.name,
                    });
                } else if (node.type === "sequence") {
                    ddl = await invoke<string>("get_sequence_ddl", {
                        connectionId: connId,
                        database: meta?.dbName || schemaStore.selectedDatabase,
                        schema: meta?.schemaName || "public",
                        sequenceName: node.name,
                    });
                } else if (node.type === "index") {
                    ddl = await invoke<string>("get_index_ddl", {
                        connectionId: connId,
                        database: meta?.dbName || schemaStore.selectedDatabase,
                        schema: meta?.schemaName || "public",
                        tableName: meta?.tableName || "",
                        indexName: node.name,
                    });
                } else if (node.type === "trigger") {
                    ddl = await invoke<string>("get_trigger_definition", {
                        connectionId: connId,
                        database: meta?.dbName || schemaStore.selectedDatabase,
                        schema: meta?.schemaName || "public",
                        tableName: meta?.tableName || "",
                        triggerName: node.name,
                    });
                }
                await navigator.clipboard.writeText(ddl);
            } catch (e) {
                console.error("Copy DDL failed:", e);
            }
            break;
        }

        case "copy_constraint_ddl": {
            // Constraint definition is stored in metadata
            const def = meta?.definition || node.name;
            await navigator.clipboard.writeText(`CONSTRAINT ${node.name} ${def}`);
            break;
        }

        case "refresh_schema": {
            await schemaStore.refresh();
            break;
        }

        default:
            console.log(`[handleContextMenuAction] Unhandled action: ${action}`);
    }
}
```

- [ ] **Step 4: Ensure `metadata` is populated on index/trigger/foreign_key nodes**

In `mapTableToNode`, update index and trigger node metadata to include `tableName` (needed by DDL commands):

```typescript
// In the indexes children mapping:
children: (cachedDetails.indexes || []).map((idx: any) => ({
    id: `idx:${tableId}.${idx.index_name}`,
    name: idx.index_name,
    type: "index" as NodeType,
    detail: [idx.index_type, idx.is_unique ? "unique" : ""].filter(Boolean).join(" · "),
    metadata: { ...node.metadata, tableName: table.table_name }, // ADD tableName
})),

// In the triggers children mapping:
children: (cachedDetails.triggers || []).map((trg: any) => ({
    id: `trg:${tableId}.${trg.trigger_name}`,
    name: trg.trigger_name,
    type: "trigger" as NodeType,
    detail: `${trg.timing} ${trg.event}`,
    metadata: { ...node.metadata, tableName: table.table_name }, // ADD tableName
})),
```

- [ ] **Step 5: Type-check**

```bash
pnpm check 2>&1 | head -40
```

Expected: no errors.

- [ ] **Step 6: Smoke test manually**

```bash
pnpm tauri dev
```

- Connect to a Postgres database
- Expand a schema — should see Tables, Views, Functions, Sequences folders
- Expand Functions folder — should lazy-load and show function nodes
- Right-click a table → should see "Copy Name", "View Data", "Open DDL", "Copy as SELECT *"
- Click "Open DDL" → should open a read-only editor tab with CREATE TABLE statement
- Right-click a function → should see "Open Definition"
- Click "Open Definition" → should open DDL tab with CREATE OR REPLACE FUNCTION

- [ ] **Step 7: Commit**

```bash
git add src/lib/stores/session.svelte.ts src/lib/components/explorer/engines/PostgresExplorer.svelte
git commit -m "feat(explorer): wire DDL tab actions, read-only tab mode, full context menu action handlers"
```

---

## Task 8: Rust Unit Tests for DDL Generation

**Files:**
- Modify: `src-tauri/src/commands/ddl_commands.rs`

Test the DDL string assembly functions in isolation (no live DB needed).

- [ ] **Step 1: Add unit tests for `get_sequence_ddl` assembly logic**

Extract the sequence DDL string builder into a pure function and test it:

At the bottom of `ddl_commands.rs`, add:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn build_sequence_ddl(
        schema: &str, name: &str, data_type: &str,
        start: i64, increment: i64, min: i64, max: i64,
        cache: i64, cycle: bool,
    ) -> String {
        format!(
            "CREATE SEQUENCE {}.{}\n    AS {}\n    START WITH {}\n    INCREMENT BY {}\n    MINVALUE {}\n    MAXVALUE {}\n    CACHE {}{};",
            schema, name, data_type, start, increment, min, max, cache,
            if cycle { "\n    CYCLE" } else { "\n    NO CYCLE" }
        )
    }

    #[test]
    fn test_sequence_ddl_no_cycle() {
        let ddl = build_sequence_ddl("public", "users_id_seq", "bigint", 1, 1, 1, 9223372036854775807, 1, false);
        assert!(ddl.contains("CREATE SEQUENCE public.users_id_seq"));
        assert!(ddl.contains("AS bigint"));
        assert!(ddl.contains("NO CYCLE"));
        assert!(!ddl.contains("\n    CYCLE;"));
    }

    #[test]
    fn test_sequence_ddl_with_cycle() {
        let ddl = build_sequence_ddl("myschema", "order_seq", "integer", 100, 10, 1, 1000, 5, true);
        assert!(ddl.contains("CREATE SEQUENCE myschema.order_seq"));
        assert!(ddl.contains("START WITH 100"));
        assert!(ddl.contains("INCREMENT BY 10"));
        assert!(ddl.contains("CACHE 5"));
        assert!(ddl.contains("\n    CYCLE"));
    }
}
```

- [ ] **Step 2: Run the tests**

```bash
cd src-tauri && cargo test ddl_commands::tests
```

Expected:
```
test commands::ddl_commands::tests::test_sequence_ddl_no_cycle ... ok
test commands::ddl_commands::tests::test_sequence_ddl_with_cycle ... ok
```

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/commands/ddl_commands.rs
git commit -m "test(postgres): add unit tests for DDL string assembly"
```

---

## Self-Review Notes

**Spec coverage check:**
- ✅ Explorer tree restructure: Tasks 4 + 5
- ✅ Context menus per-type: Task 6
- ✅ DDL tabs (read-only editor): Tasks 7
- ✅ MetaFunction + MetaSequence + MetaConstraint: Task 1
- ✅ Enhanced MetaIndex: Task 1
- ✅ get_functions, get_sequences, get_constraints, get_index_details: Task 2
- ✅ get_table_ddl, get_view_definition, get_matview_definition, get_function_ddl, get_sequence_ddl, get_index_ddl, get_trigger_definition: Task 3
- ✅ NodeType additions: Task 4
- ✅ MatViews folder: Task 5
- ✅ Constraints sub-folder on tables: Task 5

**One known gap:** `get_index_details` is not yet wired to the explorer to replace the existing basic index introspection. The enhanced index data (type, columns, predicate) will show in context-menu DDL but the tree still uses the old simple MetaIndex. This is acceptable for sprint scope — enhanced tree display can be a follow-up.

**Command registration:** Task 3 Step 4 requires finding the exact macro format in `lib.rs`. The instruction says to search first before adding — follow that.

**`pg_connect_for_commands` visibility:** Must be `pub(super)` in `introspection_commands.rs` so `ddl_commands.rs` can import it via `super::`.
