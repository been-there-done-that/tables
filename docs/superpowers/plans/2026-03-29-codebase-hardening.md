# Codebase Hardening Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix all security vulnerabilities, memory leaks, and high-impact correctness/performance bugs identified in the 2026-03-29 multi-domain code review.

**Architecture:** Changes are isolated to specific files; no cross-cutting refactors. Each task is independently testable and committable. Architectural issues (provider abstraction, bulk N+1 introspection, CTE scope resolution, conversation context windowing) are explicitly out of scope and tracked separately.

**Tech Stack:** Rust (Tauri backend), TypeScript/Svelte 5 (frontend), rusqlite, tokio-postgres, native-tls

---

## What This Plan Fixes vs. What It Defers

### In scope (20 issues):
- All 3 critical SQL injection vulnerabilities
- TLS certificate validation disabled by default
- SQLite PRAGMA injection
- Destructive query guard in agent
- CSP configuration
- Memory leaks (event listener lifecycle)
- Adapter cache leak on connection delete
- AbortSignal missing in SSE fetch
- Query result hard cap (OOM prevention)
- `is_indexed` always false in completion engine
- `lock().unwrap()` → `.expect()` in key Rust files
- Duplicate error toast

### Out of scope (require architectural design):
- N+1 → bulk introspection (needs per-adapter bulk SQL redesign)
- Excessive cloning → `Arc<MetaSchema>` (touches 40+ call sites)
- Incremental tree-sitter parsing (requires `DocumentState` redesign)
- CTE scope resolution (requires `sql_scope` crate internals study)
- Conversation context window management
- Provider abstraction (Claude-only hardcoding)
- Master key → OS keyring migration
- Window functions / LATERAL completeness in SQL completion
- Batch orchestrator cache saves

---

## File Map

**Modified:**
- `src/lib/agent/tool-executor.ts` — SQL injection fixes + destructive query guard
- `src-tauri/src/adapters/postgres.rs` — TLS cert validation fix
- `src-tauri/src/adapters/sqlite.rs` — PRAGMA identifier escaping
- `src-tauri/tauri.conf.json` — CSP configuration
- `src-tauri/src/commands/connection_commands.rs` — adapter cache eviction on delete
- `src/lib/stores/schema.svelte.ts` — fix event listener lifecycle
- `src/lib/agent/claude.ts` — add AbortSignal to fetch
- `src-tauri/src/commands/query_commands.rs` — query result hard cap
- `src-tauri/src/commands/completion_commands.rs` — fix `is_indexed` using index data
- `src-tauri/src/adapter_registry.rs` — `.unwrap()` → `.expect()`
- `src-tauri/src/connection_manager.rs` — `.unwrap()` → `.expect()` in key paths

---

## Task 1: Fix SQL Injection in Agent Tool Executor

The `count_rows` tool interpolates a raw WHERE clause from the AI directly into SQL. The `sample_table`, `get_distinct_values`, and `column_stats` tools interpolate numeric/column inputs without validation.

**Files:**
- Modify: `src/lib/agent/tool-executor.ts:115-245`

- [ ] **Step 1: Add input validation helpers at the top of executeTool**

In `src/lib/agent/tool-executor.ts`, add these helpers directly before the `switch` statement (after line 45 `const schema = ...`):

```typescript
    // Validate a numeric limit — clamp to [1, 1000], reject NaN
    function safeLimit(val: unknown, defaultVal: number): number {
        const n = Math.floor(Number(val));
        if (!isFinite(n) || n < 1) return defaultVal;
        return Math.min(n, 1000);
    }

    // Reject strings that contain SQL injection patterns
    function assertNoInjection(label: string, val: unknown): void {
        if (typeof val !== "string") return;
        if (/[;'"\\]|--|\*\/|\/\*/.test(val)) {
            throw new Error(`${label} contains invalid characters`);
        }
    }
```

- [ ] **Step 2: Fix `sample_table` — validate `n`**

Find the `case "sample_table":` block (line ~115). Change:
```typescript
        case "sample_table": {
            const n = (inp.n as number | undefined) ?? 20;
```
To:
```typescript
        case "sample_table": {
            const n = safeLimit(inp.n, 20);
```

- [ ] **Step 3: Fix `count_rows` — remove raw WHERE interpolation**

The entire `where` clause injection is the most dangerous issue. Replace the `count_rows` case (lines ~135-149) with a version that rejects the `where` parameter:

```typescript
        case "count_rows": {
            // NOTE: Raw WHERE clause from AI is a SQL injection risk.
            // We intentionally do not support arbitrary WHERE predicates here.
            // The agent should use run_query with explicit SQL if filtering is needed.
            if (inp.where) {
                return {
                    error: "count_rows does not support WHERE filters to prevent SQL injection. Use run_query with explicit SQL instead.",
                };
            }
            const sql = `SELECT COUNT(*) AS count FROM "${schema}"."${inp.table}"`;
            const result = await invoke<any>("execute_query", {
                connectionId: ctx.connectionId,
                sessionId: "agent",
                database: ctx.database,
                schema,
                query: sql,
                component: "agent",
                limit: 1,
                offset: 0,
            });
            return { count: result?.rows?.[0]?.count ?? result?.rows?.[0]?.[0] ?? 0 };
        }
```

- [ ] **Step 4: Fix `get_distinct_values` — validate `limit`**

Find the `case "get_distinct_values":` block (line ~231). Change:
```typescript
        case "get_distinct_values": {
            const limit = (inp.limit as number | undefined) ?? 20;
```
To:
```typescript
        case "get_distinct_values": {
            const limit = safeLimit(inp.limit, 20);
```

- [ ] **Step 5: Fix `column_stats` — validate column name has no injection**

Find the `case "column_stats":` block (line ~168). After `const col = ...` add the assertion:
```typescript
        case "column_stats": {
            assertNoInjection("column", inp.column);
            const col = `"${inp.column}"`;
```

- [ ] **Step 6: Add destructive query guard in `run_query`**

Find `case "run_query":` (line ~96). After `const sql = inp.sql as string;` add:

```typescript
        case "run_query": {
            const sql = inp.sql as string;
            // Warn the agent about destructive operations — these go through as-is
            // but the AI system prompt already requires user-visible warnings for DDL/DML.
            const upperSql = sql.trimStart().toUpperCase();
            const isDestructive = /^(DROP|TRUNCATE|DELETE\s+FROM|DELETE\s+\w)/i.test(upperSql);
            if (isDestructive) {
                // Surface the query in the editor so the user can see it before it runs,
                // then execute normally (plan-mode approval already gates this tool).
                ctx.openInEditor(sql, "⚠️ Destructive Query");
            }
```
Keep the rest of the `run_query` case unchanged.

- [ ] **Step 7: Commit**

```bash
git add src/lib/agent/tool-executor.ts
git commit -m "fix(security): prevent SQL injection in agent tool executor

- Remove raw WHERE clause from count_rows to eliminate injection risk
- Validate numeric inputs (n, limit) with safeLimit helper
- Add assertNoInjection guard for column_stats column parameter
- Surface destructive queries in editor before execution

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

## Task 2: Fix TLS Certificate Validation in Postgres Adapter

TLS is hard-coded to accept invalid certs when enabled. Default should be secure; opt-out only when explicitly configured.

**Files:**
- Modify: `src-tauri/src/adapters/postgres.rs:23-75, 142-147`

- [ ] **Step 1: Add `allow_invalid_certs` to `PostgresConfig`**

In `src-tauri/src/adapters/postgres.rs`, find the `PostgresConfig` struct (line ~23). Add a new field:

```rust
pub struct PostgresConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub database: Option<String>,
    pub use_tls: bool,
    pub allow_invalid_certs: bool,  // ADD THIS — default false
}
```

- [ ] **Step 2: Update `PostgresConfig::new` to initialize the new field**

In the `new()` constructor (line ~33), add the field:
```rust
    pub fn new(host: impl Into<String>, username: impl Into<String>, password: impl Into<String>) -> Self {
        Self {
            host: host.into(),
            port: 5432,
            username: username.into(),
            password: password.into(),
            database: None,
            use_tls: false,
            allow_invalid_certs: false,  // ADD THIS
        }
    }
```

- [ ] **Step 3: Update `from_config` to read the new field from JSON**

In `from_config` (line ~101), after the `use_tls` line, add:
```rust
        let use_tls = config.get("tls").and_then(|t| t.get("enabled")).and_then(|v| v.as_bool()).unwrap_or(false);
        let allow_invalid_certs = config.get("tls")
            .and_then(|t| t.get("allow_invalid_certs"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let mut pg_config = PostgresConfig::new(host, username, password)
            .with_port(port)
            .with_tls(use_tls);
        // ADD THIS:
        pg_config.allow_invalid_certs = allow_invalid_certs;
```

- [ ] **Step 4: Fix TLS connector to use `allow_invalid_certs` instead of always-true**

Find the TLS connector block (line ~142):
```rust
            let tls_connector = native_tls::TlsConnector::builder()
                .danger_accept_invalid_certs(true)   // REMOVE THIS
                .build()
```
Replace with:
```rust
            let tls_connector = native_tls::TlsConnector::builder()
                .danger_accept_invalid_certs(self.config.allow_invalid_certs)
                .build()
```

- [ ] **Step 5: Verify Rust compiles**

```bash
cd src-tauri && cargo check 2>&1 | head -30
```
Expected: no errors related to PostgresConfig.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/adapters/postgres.rs
git commit -m "fix(security): default TLS to reject invalid certs in Postgres adapter

Removes hardcoded danger_accept_invalid_certs(true). Certificate validation
is now opt-in via tls.allow_invalid_certs in the connection config JSON.

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

## Task 3: Fix SQLite PRAGMA Injection

Table names are interpolated directly into PRAGMA strings without escaping single quotes.

**Files:**
- Modify: `src-tauri/src/adapters/sqlite.rs:482, 514`

- [ ] **Step 1: Fix `list_indexes` PRAGMA**

In `src-tauri/src/adapters/sqlite.rs` at line 482, find:
```rust
            .prepare(&format!("SELECT name, \"unique\" FROM pragma_index_list('{}')", table.name))
```
Replace with:
```rust
            .prepare(&format!(
                "SELECT name, \"unique\" FROM pragma_index_list('{}')",
                table.name.replace('\'', "''")
            ))
```

- [ ] **Step 2: Fix `list_foreign_keys` PRAGMA**

At line 514, find:
```rust
            .prepare(&format!("PRAGMA foreign_key_list('{}')", table.name))
```
Replace with:
```rust
            .prepare(&format!(
                "PRAGMA foreign_key_list('{}')",
                table.name.replace('\'', "''")
            ))
```

- [ ] **Step 3: Verify Rust compiles**

```bash
cd src-tauri && cargo check 2>&1 | grep -E "error|warning: unused"
```
Expected: no new errors.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/adapters/sqlite.rs
git commit -m "fix(security): escape single quotes in SQLite PRAGMA table names

Prevents injection via table names containing single quotes in
pragma_index_list() and pragma_foreign_key_list() calls.

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

## Task 4: Add Content Security Policy

Tauri CSP is `null` — no protection against inline script execution.

**Files:**
- Modify: `src-tauri/tauri.conf.json:25-27`

- [ ] **Step 1: Set a strict CSP**

In `src-tauri/tauri.conf.json`, find:
```json
    "security": {
      "csp": null
    }
```
Replace with:
```json
    "security": {
      "csp": "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; font-src 'self' data:; connect-src 'self' http://127.0.0.1:* ipc: https://ipc.localhost"
    }
```

- [ ] **Step 2: Verify the app still loads in dev mode**

```bash
pnpm tauri dev &
sleep 15
# Check that the app window opens and the main view renders without CSP errors
# Then kill the dev process
kill %1
```
Expected: app renders normally, no "Refused to execute script" errors in webview console.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/tauri.conf.json
git commit -m "fix(security): enable strict Content Security Policy

Replaces null CSP with a policy that restricts scripts to same-origin only,
allows inline styles (Monaco requires this), and permits connections to
localhost (harness) and the Tauri IPC protocol.

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

## Task 5: Evict Adapter Cache on Connection Delete

When a connection is deleted, its cached `DatabaseAdapter` stays in the `RwLock<HashMap>` forever. The Tauri command is already `async`, so we can do the async eviction there.

**Files:**
- Modify: `src-tauri/src/commands/connection_commands.rs:82-94`

- [ ] **Step 1: Add adapter eviction after successful delete**

In `src-tauri/src/commands/connection_commands.rs`, replace the `delete_connection` command (lines 81-94):

```rust
/// Delete a connection
#[tauri::command]
pub async fn delete_connection(
    id: String,
    db_state: State<'_, DatabaseState>,
    conn_state: State<'_, ConnectionManagerState>,
) -> Result<(), String> {
    debug!("Deleting connection '{}'", id);
    let manager = ConnectionManager::from_state(&db_state, &conn_state);
    let result = manager.delete_connection(&id);
    if result.is_ok() {
        // Evict the cached adapter so it doesn't leak memory
        conn_state.adapters.write().await.remove(&id);
        info!("Connection '{}' deleted and adapter cache evicted", id);
    }
    result
}
```

- [ ] **Step 2: Verify Rust compiles**

```bash
cd src-tauri && cargo check 2>&1 | head -20
```
Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/commands/connection_commands.rs
git commit -m "fix(memory): evict adapter cache when connection is deleted

Prevents unbounded growth of the adapter HashMap in long-running sessions
where users create and delete many connections.

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

## Task 6: Fix Event Listener Lifecycle in Schema Store

`listen()` is called inside a `.then()` chain without being awaited. If the component is torn down before the promise resolves, the unlisten function is never stored and the listener leaks.

**Files:**
- Modify: `src/lib/stores/schema.svelte.ts:45-54`

- [ ] **Step 1: Await the listen() call properly**

In `src/lib/stores/schema.svelte.ts`, find the `init()` method around line 30. Look for:

```typescript
        // Listen for active connection changes from backend
        import("@tauri-apps/api/event").then(({ listen }) => {
            listen<Record<string, string>>("active-connections-changed", (event) => {
                this.activeConnectionsMap = event.payload;
            }).then(un => this.unlistenActive = un);
        });
```

Replace with:

```typescript
        // Listen for active connection changes from backend
        // Await the listen() promise so unlistenActive is guaranteed to be set
        // before any teardown can happen
        import("@tauri-apps/api/event").then(async ({ listen }) => {
            this.unlistenActive = await listen<Record<string, string>>(
                "active-connections-changed",
                (event) => {
                    this.activeConnectionsMap = event.payload;
                }
            );
        });
```

- [ ] **Step 2: Verify no TypeScript errors**

```bash
pnpm check 2>&1 | head -30
```
Expected: no new errors.

- [ ] **Step 3: Commit**

```bash
git add src/lib/stores/schema.svelte.ts
git commit -m "fix(memory): await event listener in schema store to prevent leak

The unlisten function was being stored asynchronously via .then(), which meant
it could be missed if the store was torn down before the promise resolved.

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

## Task 7: Add AbortSignal to Harness Fetch in Agent

The `/session/send` fetch in `claude.ts` doesn't pass the abort signal, so when the user stops a session, the SSE stream is cancelled but the underlying HTTP request continues.

**Files:**
- Modify: `src/lib/agent/claude.ts`

- [ ] **Step 1: Find the send() fetch call**

Read `src/lib/agent/claude.ts` and find the `fetch` call that POSTs to `/session/send`. It will look similar to:

```typescript
fetch(`${base}/session/send`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ sessionId, text }),
})
```

- [ ] **Step 2: Add AbortSignal to that fetch**

Add `signal: opts.abortController.signal` to the fetch options:

```typescript
fetch(`${base}/session/send`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ sessionId, text }),
    signal: opts.abortController.signal,
})
```

- [ ] **Step 3: Verify no TypeScript errors**

```bash
pnpm check 2>&1 | grep -E "error TS"
```
Expected: no new TS errors.

- [ ] **Step 4: Commit**

```bash
git add src/lib/agent/claude.ts
git commit -m "fix(memory): pass AbortSignal to session/send fetch in agent

Without this, stopping an agent session cancelled the SSE stream but left
the underlying HTTP POST running in the background.

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

## Task 8: Hard-Cap Query Results to Prevent OOM

`SELECT *` on a million-row table loads everything into memory before serialization. Add a configurable hard cap.

**Files:**
- Modify: `src-tauri/src/commands/query_commands.rs`

- [ ] **Step 1: Read query_commands.rs to find where result rows are collected**

```bash
cd src-tauri && grep -n "rows\|limit\|AdapterQueryResult" src/commands/query_commands.rs | head -40
```

- [ ] **Step 2: Find the execute_query command and locate where it returns results**

Read `src-tauri/src/commands/query_commands.rs` and find the `execute_query` #[tauri::command]. Look for where `AdapterQueryResult` or `rows` is returned to the frontend.

- [ ] **Step 3: Add a hard cap constant and truncation**

At the top of `query_commands.rs` (after the imports), add:
```rust
/// Maximum rows returned to the frontend in a single query.
/// Prevents OOM when users run SELECT * on large tables.
const MAX_RESULT_ROWS: usize = 10_000;
```

Find where the rows Vec is built or the result is returned. Add a truncation check. If the result looks like:
```rust
Ok(QueryResult {
    columns: result.columns,
    rows: result.rows,
    ...
})
```
Add truncation before returning:
```rust
let total_rows = result.rows.len();
let truncated = total_rows > MAX_RESULT_ROWS;
let rows = if truncated {
    result.rows.into_iter().take(MAX_RESULT_ROWS).collect()
} else {
    result.rows
};
Ok(QueryResult {
    columns: result.columns,
    rows,
    truncated,
    total_rows: Some(total_rows),
    ...
})
```
Adjust field names to match the actual struct.

- [ ] **Step 4: Verify Rust compiles**

```bash
cd src-tauri && cargo check 2>&1 | head -30
```
Expected: no errors. If `QueryResult` doesn't have `truncated` / `total_rows` fields, add them with `#[serde(skip_serializing_if = "...")]` as appropriate.

- [ ] **Step 5: Run existing tests**

```bash
cd src-tauri && cargo test query 2>&1 | tail -20
```
Expected: all query-related tests pass.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/commands/query_commands.rs
git commit -m "fix(safety): hard-cap query results at 10k rows to prevent OOM

Large SELECT * queries on multi-million row tables would collect all rows
into memory before serialization. Returns truncated flag when limit is hit.

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

## Task 9: Fix `is_indexed` Always False in Completion Engine

`MetaIndex.columns: Vec<String>` has column names. Build a lookup set while iterating indexes and use it when constructing `ColumnInfo`.

**Files:**
- Modify: `src-tauri/src/commands/completion_commands.rs:174-245`

- [ ] **Step 1: Build an indexed column set per table**

In `src-tauri/src/commands/completion_commands.rs`, find the `schema_graph_from_meta` function (line ~175). Find the inner loop `for table in &schema.tables {`.

Before the `let columns: Vec<ColumnInfo> = ...` block, add:

```rust
            // Build set of column names that appear in at least one index
            let indexed_columns: std::collections::HashSet<&str> = table
                .indexes
                .iter()
                .flat_map(|idx| idx.columns.iter().map(|c| c.as_str()))
                .collect();
```

- [ ] **Step 2: Use the set when building ColumnInfo**

Change the `is_indexed: false` line to:

```rust
            let columns: Vec<ColumnInfo> = table.columns.iter().map(|col| {
                ColumnInfo {
                    name: col.column_name.clone(),
                    data_type: col.raw_type.clone(),
                    is_nullable: col.nullable,
                    is_primary_key: col.is_primary_key,
                    is_indexed: indexed_columns.contains(col.column_name.as_str()),
                }
            }).collect();
```

- [ ] **Step 3: Verify Rust compiles**

```bash
cd src-tauri && cargo check 2>&1 | head -20
```
Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/commands/completion_commands.rs
git commit -m "fix(completion): populate is_indexed using MetaIndex.columns data

is_indexed was always false because the index-to-column join was not implemented.
Now builds a HashSet of indexed column names per table using MetaIndex.columns
and marks columns accordingly, enabling proper ranked completion suggestions.

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

## Task 10: Replace Panic-Prone `.unwrap()` with `.expect()` in Key Rust Files

Lock poisoning from `.unwrap()` cascades failures. `.expect()` with context messages makes crashes debuggable and doesn't change behavior otherwise.

**Files:**
- Modify: `src-tauri/src/adapter_registry.rs`
- Modify: `src-tauri/src/connection_manager.rs` (key paths only)

- [ ] **Step 1: Fix adapter_registry.rs**

```bash
cd src-tauri && grep -n "\.unwrap()" src/adapter_registry.rs
```

For each `ADAPTER_REGISTRY.write().unwrap()` and `ADAPTER_REGISTRY.read().unwrap()`, replace with:
- `ADAPTER_REGISTRY.write().expect("adapter registry write lock poisoned")`
- `ADAPTER_REGISTRY.read().expect("adapter registry read lock poisoned")`

- [ ] **Step 2: Fix connection_manager.rs lock acquisitions**

```bash
cd src-tauri && grep -n "\.lock()\.unwrap\|\.write()\.unwrap\|\.read()\.unwrap" src/connection_manager.rs | head -20
```

For the `db.lock().unwrap()` pattern in `delete_connection` and similar critical paths, replace with `.expect("db lock poisoned")`. Do not change test-only code.

- [ ] **Step 3: Verify Rust compiles**

```bash
cd src-tauri && cargo check 2>&1 | head -20
```
Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/adapter_registry.rs src-tauri/src/connection_manager.rs
git commit -m "fix(reliability): replace unwrap() with expect() on lock acquisitions

Poisoned locks now produce actionable error messages instead of an opaque
thread 'X' panicked at 'called Option::unwrap() on a None value'.

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

## Task 11: Remove Duplicate Error Toast in Schema Store

A duplicate `toast.error()` call causes users to see the same error notification twice.

**Files:**
- Modify: `src/lib/stores/schema.svelte.ts`

- [ ] **Step 1: Find and remove the duplicate**

```bash
grep -n "toast.error" src/lib/stores/schema.svelte.ts
```

Find the two consecutive identical `toast.error(...)` calls around line 327-328. Remove one of them.

- [ ] **Step 2: Verify no TypeScript errors**

```bash
pnpm check 2>&1 | grep -E "error TS"
```
Expected: no new errors.

- [ ] **Step 3: Commit**

```bash
git add src/lib/stores/schema.svelte.ts
git commit -m "fix(ux): remove duplicate error toast on schema refresh failure

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

## Task 12: Final Verification

- [ ] **Step 1: Run all Rust tests**

```bash
cd src-tauri && cargo test 2>&1 | tail -30
```
Expected: all tests pass, no new failures.

- [ ] **Step 2: Run frontend type check**

```bash
pnpm check 2>&1 | tail -20
```
Expected: no type errors.

- [ ] **Step 3: Run Clippy**

```bash
cd src-tauri && cargo clippy 2>&1 | grep "^error" | head -20
```
Expected: no new clippy errors.

- [ ] **Step 4: Smoke test in dev mode (optional but recommended)**

```bash
pnpm tauri dev
```
Verify:
- App loads without CSP errors
- Connecting to a DB works
- SQL editor runs queries
- Agent panel responds

---

## Summary

| Task | Category | Severity Fixed | Effort |
|------|----------|----------------|--------|
| 1: SQL injection in tool-executor | Security | CRITICAL | ~30 min |
| 2: TLS cert validation | Security | CRITICAL | ~20 min |
| 3: SQLite PRAGMA escaping | Security | CRITICAL | ~10 min |
| 4: CSP configuration | Security | HIGH | ~15 min |
| 5: Adapter cache eviction | Memory | HIGH | ~10 min |
| 6: Event listener lifecycle | Memory | HIGH | ~10 min |
| 7: AbortSignal in fetch | Memory | HIGH | ~10 min |
| 8: Query result hard cap | Safety | HIGH | ~30 min |
| 9: Fix is_indexed in completion | Correctness | MEDIUM | ~20 min |
| 10: unwrap → expect | Reliability | MEDIUM | ~15 min |
| 11: Duplicate toast | UX | LOW | ~5 min |
| 12: Verification | — | — | ~15 min |

**Total estimated time: ~3 hours**
