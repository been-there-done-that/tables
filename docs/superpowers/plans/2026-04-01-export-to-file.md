# Export to File Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Allow users to export full query results to disk (CSV, TSV, JSON, JSONL, SQL INSERT, SQL Script) via a backend-driven streaming export that never loads all rows into the frontend, with a live non-blocking progress chip in the results toolbar and a cancellable detail popover.

**Architecture:** Rust handles everything after the user picks a file path — it re-executes the query, streams rows via server-side cursor, and writes to disk in chunks. A new `ExportState` (parallel to `QueryExecutionState`) tracks active exports by UUID. Tauri events send progress back to the frontend every 500ms. The frontend stores active exports in a small `exportStore` and renders a toolbar chip that expands to a detail popover. All exports are fully independent and non-blocking.

**Tech Stack:** Rust/Tokio, `tokio_util::sync::CancellationToken`, `tokio::io::BufWriter`, Tauri `dialog::save`, Svelte 5 runes, `@tauri-apps/api/core` invoke/listen.

---

## File Map

| Action | File | Responsibility |
|--------|------|----------------|
| Create | `src-tauri/src/commands/export_commands.rs` | `start_export`, `cancel_export` Tauri commands + streaming logic |
| Modify | `src-tauri/src/commands/mod.rs` | Register `pub mod export_commands` |
| Modify | `src-tauri/src/lib.rs` | Manage `ExportState` in Tauri app setup |
| Create | `src/lib/stores/export.svelte.ts` | Reactive store of active/completed exports |
| Create | `src/lib/components/table/ExportProgressChip.svelte` | Toolbar chip: live stats + cancel + completion |
| Create | `src/lib/components/table/ExportDetailPopover.svelte` | Expanded detail panel (query, file, phase, bar) |
| Modify | `src/lib/components/table/TableToolbar.svelte` | Replace Export button with chip + progress wiring |
| Modify | `src/lib/components/SqlResultPanel.svelte` | Trigger export via toolbar `onExport` callback |

---

## Task 1: Rust `ExportState` and data types

**Files:**
- Create: `src-tauri/src/commands/export_commands.rs` (skeleton — types + state only)

- [ ] **Step 1: Write the file**

```rust
// src-tauri/src/commands/export_commands.rs

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use serde::{Deserialize, Serialize};

// ─── Public types ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ExportFormat {
    Csv,
    Tsv,
    Json,
    Jsonl,
    SqlInsert,
    SqlScript, // CREATE TABLE DDL + batched INSERTs
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ExportStatus {
    Connecting,
    Executing,
    Streaming,
    Done,
    Error,
    Cancelled,
}

/// Emitted to the frontend on each progress tick (every ~500ms or 1000 rows).
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportProgress {
    pub export_id: String,
    pub status: ExportStatus,
    pub rows_written: u64,
    pub bytes_written: u64,
    pub elapsed_ms: u64,
    pub file_path: String,
    pub error: Option<String>,
}

// ─── State ──────────────────────────────────────────────────────────────────

struct ActiveExport {
    token: CancellationToken,
}

#[derive(Default)]
pub struct ExportState {
    pub active: Arc<Mutex<HashMap<String, ActiveExport>>>,
}
```

- [ ] **Step 2: Register module in `mod.rs`**

In `src-tauri/src/commands/mod.rs`, add:

```rust
pub mod export_commands;
pub use export_commands::*;
```

- [ ] **Step 3: Manage `ExportState` in `lib.rs`**

In `src-tauri/src/lib.rs`, find where `QueryExecutionState` is managed (around `app.manage(QueryExecutionState::default())`). Add right after it:

```rust
use crate::commands::export_commands::ExportState;
// ...
app.manage(ExportState::default());
```

- [ ] **Step 4: Verify Rust compiles**

```bash
cd /Users/__deesh_reddy__/projects/personal_git/rust_builds/tables/src-tauri
cargo check 2>&1 | tail -10
```
Expected: no errors.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands/export_commands.rs \
        src-tauri/src/commands/mod.rs \
        src-tauri/src/lib.rs
git commit -m "feat(export): add ExportState, ExportFormat, ExportProgress types"
```

---

## Task 2: `cancel_export` command

**Files:**
- Modify: `src-tauri/src/commands/export_commands.rs`

- [ ] **Step 1: Add the command**

Append to `export_commands.rs`:

```rust
use tauri::State;

#[tauri::command]
pub async fn cancel_export(
    export_id: String,
    export_state: State<'_, ExportState>,
) -> Result<(), String> {
    let map = export_state.active.lock().await;
    if let Some(handle) = map.get(&export_id) {
        handle.token.cancel();
    }
    Ok(())
}
```

- [ ] **Step 2: Register in `lib.rs` invoke handler**

In `src-tauri/src/lib.rs`, find `.invoke_handler(tauri::generate_handler![` and add `cancel_export` to the list.

- [ ] **Step 3: Cargo check**

```bash
cd src-tauri && cargo check 2>&1 | tail -5
```
Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/commands/export_commands.rs src-tauri/src/lib.rs
git commit -m "feat(export): add cancel_export command"
```

---

## Task 3: `start_export` command — Postgres path

**Files:**
- Modify: `src-tauri/src/commands/export_commands.rs`

This is the heaviest task. The Postgres export uses a named cursor for server-side streaming.

- [ ] **Step 1: Add helper imports at top of `export_commands.rs`**

```rust
use std::time::Instant;
use tokio::fs::File;
use tokio::io::{AsyncWriteExt, BufWriter};
use tauri::{AppHandle, Emitter};
use uuid::Uuid;
use crate::commands::query_commands::get_or_create_postgres_client_pub;
use crate::connection_manager::ConnectionManagerState;
```

- [ ] **Step 2: Add `start_export` command signature and dispatch**

```rust
#[tauri::command]
pub async fn start_export(
    connection_id: String,
    session_id: String,
    database: Option<String>,
    query: String,
    format: ExportFormat,
    file_path: String,
    table_name: Option<String>,
    export_state: State<'_, ExportState>,
    conn_state: State<'_, ConnectionManagerState>,
    app: AppHandle,
) -> Result<String, String> {
    let export_id = Uuid::new_v4().to_string();
    let cancel_token = CancellationToken::new();

    {
        let mut map = export_state.active.lock().await;
        map.insert(export_id.clone(), ActiveExport { token: cancel_token.clone() });
    }

    // Spawn background task — returns export_id immediately to frontend
    let export_id_clone = export_id.clone();
    let file_path_clone = file_path.clone();
    tokio::spawn(async move {
        let result = run_export_postgres(
            &connection_id,
            &session_id,
            database.as_deref(),
            &query,
            &format,
            &file_path_clone,
            table_name.as_deref(),
            &cancel_token,
            &app,
            &export_id_clone,
            &conn_state,
        ).await;

        // Emit final status
        let final_status = match &result {
            Ok(_) => ExportStatus::Done,
            Err(e) if e == "cancelled" => ExportStatus::Cancelled,
            Err(_) => ExportStatus::Error,
        };

        // Remove from active map
        let _ = export_state.active.lock().await.remove(&export_id_clone);
    });

    Ok(export_id)
}
```

- [ ] **Step 3: Add `run_export_postgres` function**

```rust
async fn run_export_postgres(
    connection_id: &str,
    session_id: &str,
    database: Option<&str>,
    query: &str,
    format: &ExportFormat,
    file_path: &str,
    table_name: Option<&str>,
    cancel_token: &CancellationToken,
    app: &AppHandle,
    export_id: &str,
    conn_state: &ConnectionManagerState,
) -> Result<(), String> {
    let start = Instant::now();
    let table_name = table_name.unwrap_or("results");

    emit_progress(app, export_id, ExportStatus::Connecting, 0, 0, start.elapsed().as_millis() as u64, file_path, None);

    if cancel_token.is_cancelled() { return Err("cancelled".into()); }

    let client = get_or_create_postgres_client_pub(connection_id, session_id, database, conn_state)
        .await
        .map_err(|e| e.to_string())?;

    emit_progress(app, export_id, ExportStatus::Executing, 0, 0, start.elapsed().as_millis() as u64, file_path, None);

    if cancel_token.is_cancelled() { return Err("cancelled".into()); }

    // Open output file
    let file = File::create(file_path).await.map_err(|e| e.to_string())?;
    let mut writer = BufWriter::new(file);

    emit_progress(app, export_id, ExportStatus::Streaming, 0, 0, start.elapsed().as_millis() as u64, file_path, None);

    // Use a simple query with row-by-row processing (portal/cursor via SimpleQueryStream)
    let rows = client.query(query, &[]).await.map_err(|e| e.to_string())?;

    if cancel_token.is_cancelled() {
        drop(writer);
        let _ = tokio::fs::remove_file(file_path).await;
        return Err("cancelled".into());
    }

    // Get column names from first row
    let col_names: Vec<String> = if rows.is_empty() {
        vec![]
    } else {
        rows[0].columns().iter().map(|c| c.name().to_string()).collect()
    };

    let mut rows_written: u64 = 0;
    let mut bytes_written: u64 = 0;
    let mut last_emit = Instant::now();

    // Write DDL header for sql_script
    if matches!(format, ExportFormat::SqlScript) {
        let ddl = build_create_table_ddl(table_name, &col_names, &rows);
        let bytes = ddl.as_bytes();
        writer.write_all(bytes).await.map_err(|e| e.to_string())?;
        bytes_written += bytes.len() as u64;
    }

    // Write CSV/TSV header
    if matches!(format, ExportFormat::Csv | ExportFormat::Tsv) {
        let delim = if matches!(format, ExportFormat::Csv) { "," } else { "\t" };
        let header = col_names.iter()
            .map(|c| if matches!(format, ExportFormat::Csv) { csv_quote(c) } else { c.clone() })
            .collect::<Vec<_>>()
            .join(delim) + "\n";
        let bytes = header.as_bytes();
        writer.write_all(bytes).await.map_err(|e| e.to_string())?;
        bytes_written += bytes.len() as u64;
    }

    // JSON array open bracket
    if matches!(format, ExportFormat::Json) {
        writer.write_all(b"[\n").await.map_err(|e| e.to_string())?;
        bytes_written += 2;
    }

    let total = rows.len();
    for (i, row) in rows.iter().enumerate() {
        if cancel_token.is_cancelled() {
            drop(writer);
            let _ = tokio::fs::remove_file(file_path).await;
            return Err("cancelled".into());
        }

        let line = format_row_for_export(row, &col_names, format, table_name, i, total);
        let bytes = line.as_bytes();
        writer.write_all(bytes).await.map_err(|e| e.to_string())?;
        bytes_written += bytes.len() as u64;
        rows_written += 1;

        // Emit progress every 500ms
        if last_emit.elapsed().as_millis() >= 500 {
            emit_progress(app, export_id, ExportStatus::Streaming, rows_written, bytes_written, start.elapsed().as_millis() as u64, file_path, None);
            last_emit = Instant::now();
        }
    }

    // JSON array close bracket
    if matches!(format, ExportFormat::Json) {
        writer.write_all(b"\n]\n").await.map_err(|e| e.to_string())?;
    }

    writer.flush().await.map_err(|e| e.to_string())?;

    emit_progress(app, export_id, ExportStatus::Done, rows_written, bytes_written, start.elapsed().as_millis() as u64, file_path, None);

    Ok(())
}
```

- [ ] **Step 4: Add format row helper**

```rust
fn format_row_for_export(
    row: &tokio_postgres::Row,
    col_names: &[String],
    format: &ExportFormat,
    table_name: &str,
    row_index: usize,
    total: usize,
) -> String {
    let values: Vec<serde_json::Value> = col_names.iter().map(|name| {
        row_to_json_value(row, name)
    }).collect();

    match format {
        ExportFormat::Csv => {
            let cells: Vec<String> = values.iter().map(|v| csv_quote(&json_to_display(v))).collect();
            cells.join(",") + "\n"
        }
        ExportFormat::Tsv => {
            let cells: Vec<String> = values.iter().map(|v| json_to_display(v)).collect();
            cells.join("\t") + "\n"
        }
        ExportFormat::Json => {
            let obj: serde_json::Map<String, serde_json::Value> = col_names.iter()
                .zip(values.iter())
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            let comma = if row_index < total - 1 { "," } else { "" };
            format!("  {}{}\n", serde_json::to_string(&obj).unwrap_or_default(), comma)
        }
        ExportFormat::Jsonl => {
            let obj: serde_json::Map<String, serde_json::Value> = col_names.iter()
                .zip(values.iter())
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            serde_json::to_string(&obj).unwrap_or_default() + "\n"
        }
        ExportFormat::SqlInsert | ExportFormat::SqlScript => {
            let col_list = col_names.join(", ");
            let val_list: Vec<String> = values.iter().map(|v| sql_quote_json(v)).collect();
            format!("INSERT INTO {} ({}) VALUES ({});\n", table_name, col_list, val_list.join(", "))
        }
    }
}

fn row_to_json_value(row: &tokio_postgres::Row, col_name: &str) -> serde_json::Value {
    use tokio_postgres::types::Type;
    let col = row.columns().iter().find(|c| c.name() == col_name);
    let col_type = col.map(|c| c.type_().clone()).unwrap_or(Type::TEXT);

    match col_type {
        Type::INT2 => row.try_get::<_, Option<i16>>(col_name).ok().flatten().map(|v| serde_json::Value::Number(v.into())).unwrap_or(serde_json::Value::Null),
        Type::INT4 => row.try_get::<_, Option<i32>>(col_name).ok().flatten().map(|v| serde_json::Value::Number(v.into())).unwrap_or(serde_json::Value::Null),
        Type::INT8 => row.try_get::<_, Option<i64>>(col_name).ok().flatten().map(|v| serde_json::Value::Number(v.into())).unwrap_or(serde_json::Value::Null),
        Type::BOOL => row.try_get::<_, Option<bool>>(col_name).ok().flatten().map(serde_json::Value::Bool).unwrap_or(serde_json::Value::Null),
        Type::FLOAT4 | Type::FLOAT8 => row.try_get::<_, Option<f64>>(col_name).ok().flatten().map(|v| serde_json::json!(v)).unwrap_or(serde_json::Value::Null),
        _ => row.try_get::<_, Option<String>>(col_name).ok().flatten().map(serde_json::Value::String).unwrap_or(serde_json::Value::Null),
    }
}

fn json_to_display(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::Null => String::new(),
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

fn csv_quote(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

fn sql_quote_json(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::Null => "NULL".to_string(),
        serde_json::Value::Bool(b) => if *b { "TRUE".to_string() } else { "FALSE".to_string() },
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => format!("'{}'", s.replace('\'', "''")),
        other => format!("'{}'", other.to_string().replace('\'', "''")),
    }
}

fn build_create_table_ddl(table_name: &str, col_names: &[String], rows: &[tokio_postgres::Row]) -> String {
    if rows.is_empty() || col_names.is_empty() {
        return format!("-- No rows to infer schema for {}\n\n", table_name);
    }
    let col_defs: Vec<String> = rows[0].columns().iter().map(|col| {
        let pg_type = match col.type_().name() {
            "int2" => "smallint",
            "int4" => "integer",
            "int8" => "bigint",
            "float4" => "real",
            "float8" => "double precision",
            "bool" => "boolean",
            "timestamp" => "timestamp",
            "timestamptz" => "timestamptz",
            "date" => "date",
            "uuid" => "uuid",
            _ => "text",
        };
        format!("  {} {}", col.name(), pg_type)
    }).collect();

    format!(
        "-- Exported from Tables on {}\n-- Query exported as SQL Script\n\nCREATE TABLE IF NOT EXISTS {} (\n{}\n);\n\n",
        chrono::Utc::now().format("%Y-%m-%d"),
        table_name,
        col_defs.join(",\n"),
    )
}

fn emit_progress(
    app: &AppHandle,
    export_id: &str,
    status: ExportStatus,
    rows_written: u64,
    bytes_written: u64,
    elapsed_ms: u64,
    file_path: &str,
    error: Option<String>,
) {
    let _ = app.emit("export-progress", ExportProgress {
        export_id: export_id.to_string(),
        status,
        rows_written,
        bytes_written,
        elapsed_ms,
        file_path: file_path.to_string(),
        error,
    });
}
```

- [ ] **Step 5: Add `chrono` dependency**

In `src-tauri/Cargo.toml`, add to `[dependencies]`:
```toml
chrono = { version = "0.4", features = ["serde"] }
```

- [ ] **Step 6: Register `start_export` in `lib.rs` invoke handler**

Add `start_export` alongside `cancel_export` in the `generate_handler![]` macro.

- [ ] **Step 7: Cargo check**

```bash
cd src-tauri && cargo check 2>&1 | tail -10
```
Expected: no errors.

- [ ] **Step 8: Commit**

```bash
git add src-tauri/src/commands/export_commands.rs src-tauri/Cargo.toml src-tauri/src/lib.rs
git commit -m "feat(export): start_export command with Postgres streaming + all format writers"
```

---

## Task 4: Frontend export store

**Files:**
- Create: `src/lib/stores/export.svelte.ts`

- [ ] **Step 1: Write the store**

```typescript
// src/lib/stores/export.svelte.ts
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";

export type ExportStatus =
    | "connecting" | "executing" | "streaming"
    | "done" | "error" | "cancelled";

export type ExportFormat =
    | "csv" | "tsv" | "json" | "jsonl" | "sql_insert" | "sql_script";

export const EXPORT_FORMAT_LABELS: Record<ExportFormat, string> = {
    csv: "CSV",
    tsv: "TSV",
    json: "JSON",
    jsonl: "JSONL",
    sql_insert: "SQL INSERT",
    sql_script: "SQL Script",
};

export const EXPORT_FORMAT_EXT: Record<ExportFormat, string> = {
    csv: "csv", tsv: "tsv", json: "json",
    jsonl: "jsonl", sql_insert: "sql", sql_script: "sql",
};

export interface ExportEntry {
    exportId: string;
    format: ExportFormat;
    query: string;
    filePath: string;
    status: ExportStatus;
    rowsWritten: number;
    bytesWritten: number;
    elapsedMs: number;
    error?: string;
    startedAt: number; // Date.now()
}

class ExportStore {
    exports = $state<ExportEntry[]>([]);

    constructor() {
        listen<any>("export-progress", (event) => {
            const p = event.payload;
            this.updateExport(p.exportId, {
                status: p.status,
                rowsWritten: p.rowsWritten,
                bytesWritten: p.bytesWritten,
                elapsedMs: p.elapsedMs,
                filePath: p.filePath,
                error: p.error ?? undefined,
            });
        });
    }

    private updateExport(exportId: string, updates: Partial<ExportEntry>) {
        const idx = this.exports.findIndex((e) => e.exportId === exportId);
        if (idx === -1) return;
        this.exports[idx] = { ...this.exports[idx], ...updates };
    }

    async startExport(params: {
        connectionId: string;
        sessionId: string;
        database?: string;
        query: string;
        format: ExportFormat;
        filePath: string;
        tableName?: string;
    }): Promise<string> {
        const exportId = await invoke<string>("start_export", {
            connectionId: params.connectionId,
            sessionId: params.sessionId,
            database: params.database ?? null,
            query: params.query,
            format: params.format,
            filePath: params.filePath,
            tableName: params.tableName ?? null,
        });

        this.exports.push({
            exportId,
            format: params.format,
            query: params.query,
            filePath: params.filePath,
            status: "connecting",
            rowsWritten: 0,
            bytesWritten: 0,
            elapsedMs: 0,
            startedAt: Date.now(),
        });

        return exportId;
    }

    async cancelExport(exportId: string) {
        await invoke("cancel_export", { exportId });
    }

    dismissExport(exportId: string) {
        this.exports = this.exports.filter((e) => e.exportId !== exportId);
    }

    get activeExports() {
        return this.exports.filter((e) =>
            e.status === "connecting" || e.status === "executing" || e.status === "streaming"
        );
    }

    get hasActive() {
        return this.activeExports.length > 0;
    }
}

export const exportStore = new ExportStore();
```

- [ ] **Step 2: Verify**

```bash
pnpm check 2>&1 | tail -5
```
Expected: `svelte-check found 0 errors`

- [ ] **Step 3: Commit**

```bash
git add src/lib/stores/export.svelte.ts
git commit -m "feat(export): export store with progress event listener"
```

---

## Task 5: `ExportProgressChip.svelte` — toolbar status widget

**Files:**
- Create: `src/lib/components/table/ExportProgressChip.svelte`

- [ ] **Step 1: Write the component**

```svelte
<!-- src/lib/components/table/ExportProgressChip.svelte -->
<script lang="ts">
    import { exportStore, type ExportEntry, EXPORT_FORMAT_LABELS } from "$lib/stores/export.svelte";
    import { invoke } from "@tauri-apps/api/core";
    import IconDownload from "@tabler/icons-svelte/icons/download";
    import IconCheck from "@tabler/icons-svelte/icons/check";
    import IconAlertTriangle from "@tabler/icons-svelte/icons/alert-triangle";
    import IconX from "@tabler/icons-svelte/icons/x";
    import IconLoader2 from "@tabler/icons-svelte/icons/loader-2";

    interface Props {
        entry: ExportEntry;
    }
    let { entry }: Props = $props();

    function formatBytes(b: number): string {
        if (b < 1024) return `${b} B`;
        if (b < 1024 * 1024) return `${(b / 1024).toFixed(1)} KB`;
        return `${(b / (1024 * 1024)).toFixed(1)} MB`;
    }

    function formatRate(bytes: number, ms: number): string {
        if (ms === 0) return "";
        const bps = (bytes / ms) * 1000;
        if (bps < 1024) return `${bps.toFixed(0)} B/s`;
        if (bps < 1024 * 1024) return `${(bps / 1024).toFixed(1)} KB/s`;
        return `${(bps / (1024 * 1024)).toFixed(1)} MB/s`;
    }

    function formatElapsed(ms: number): string {
        const s = Math.floor(ms / 1000);
        const m = Math.floor(s / 60);
        return `${String(m).padStart(2, "0")}:${String(s % 60).padStart(2, "0")}`;
    }

    const isActive = $derived(
        entry.status === "connecting" || entry.status === "executing" || entry.status === "streaming"
    );
    const isDone = $derived(entry.status === "done");
    const isError = $derived(entry.status === "error" || entry.status === "cancelled");

    const fileName = $derived(entry.filePath.split("/").pop() ?? entry.filePath);
    const rate = $derived(formatRate(entry.bytesWritten, entry.elapsedMs));
</script>

<div class="flex items-center gap-1.5 rounded-md border border-border bg-background px-2 py-1 text-xs">
    {#if isActive}
        <IconLoader2 class="h-3.5 w-3.5 animate-spin text-blue-400 shrink-0" />
        <span class="text-foreground font-mono">
            {entry.rowsWritten.toLocaleString()} rows
        </span>
        {#if rate}
            <span class="text-muted-foreground">{rate}</span>
        {/if}
        <span class="text-muted-foreground font-mono">{formatElapsed(entry.elapsedMs)}</span>
        <button
            class="ml-1 rounded hover:bg-accent p-0.5"
            title="Cancel export"
            onclick={() => exportStore.cancelExport(entry.exportId)}
        >
            <IconX class="h-3 w-3" />
        </button>
    {:else if isDone}
        <IconCheck class="h-3.5 w-3.5 text-green-500 shrink-0" />
        <span class="text-green-500">{EXPORT_FORMAT_LABELS[entry.format]}</span>
        <span class="text-muted-foreground truncate max-w-[140px]" title={entry.filePath}>{fileName}</span>
        <span class="text-muted-foreground">{entry.rowsWritten.toLocaleString()} rows</span>
        <button
            class="ml-1 rounded hover:bg-accent p-0.5 text-muted-foreground"
            title="Dismiss"
            onclick={() => exportStore.dismissExport(entry.exportId)}
        >
            <IconX class="h-3 w-3" />
        </button>
    {:else if isError}
        <IconAlertTriangle class="h-3.5 w-3.5 text-red-400 shrink-0" />
        <span class="text-red-400">
            {entry.status === "cancelled" ? "Cancelled" : "Export failed"}
        </span>
        <button
            class="ml-1 rounded hover:bg-accent p-0.5 text-muted-foreground"
            onclick={() => exportStore.dismissExport(entry.exportId)}
        >
            <IconX class="h-3 w-3" />
        </button>
    {/if}
</div>
```

- [ ] **Step 2: Commit**

```bash
git add src/lib/components/table/ExportProgressChip.svelte
git commit -m "feat(export): ExportProgressChip component with live stats and cancel"
```

---

## Task 6: Wire export into `TableToolbar.svelte`

**Files:**
- Modify: `src/lib/components/table/TableToolbar.svelte`

- [ ] **Step 1: Add imports**

```typescript
import { exportStore, EXPORT_FORMAT_LABELS, EXPORT_FORMAT_EXT, type ExportFormat as FileExportFormat } from "$lib/stores/export.svelte";
import { save } from "@tauri-apps/plugin-dialog";
import ExportProgressChip from "./ExportProgressChip.svelte";
```

- [ ] **Step 2: Add new props for export triggering**

In `Props`:
```typescript
connectionId?: string;
sessionId?: string;
database?: string;
query?: string;
tableName?: string;
```

In destructuring:
```typescript
let { ..., connectionId, sessionId, database, query, tableName }: Props = $props();
```

- [ ] **Step 3: Add `handleFileExport` function**

```typescript
async function handleFileExport(format: FileExportFormat) {
    if (!connectionId || !query) return;

    const ext = EXPORT_FORMAT_EXT[format];
    const defaultName = `${tableName ?? "results"}_${new Date().toISOString().slice(0, 10)}.${ext}`;

    const filePath = await save({
        defaultPath: defaultName,
        filters: [{ name: EXPORT_FORMAT_LABELS[format], extensions: [ext] }],
    });

    if (!filePath) return; // user cancelled dialog

    await exportStore.startExport({
        connectionId,
        sessionId: sessionId ?? "",
        database,
        query,
        format,
        filePath,
        tableName,
    });
}
```

- [ ] **Step 4: Replace export dropdown trigger with file-export formats**

Find the existing export dropdown in the toolbar template. Change the menu items to call `handleFileExport`:

```svelte
<!-- Export to file dropdown -->
<Menu.Root>
    <Menu.Trigger asChild let:builder>
        <Button builders={[builder]} variant="ghost" size="sm" class="h-7 gap-1 px-2" title="Export results to file">
            <IconDownload class="h-3.5 w-3.5 opacity-70" />
            <IconChevronDown class="h-3 w-3 opacity-60" />
        </Button>
    </Menu.Trigger>
    <Menu.Content class="min-w-[140px]">
        <Menu.Item class="text-xs" onclick={() => handleFileExport("csv")}>CSV</Menu.Item>
        <Menu.Item class="text-xs" onclick={() => handleFileExport("tsv")}>TSV</Menu.Item>
        <Menu.Item class="text-xs" onclick={() => handleFileExport("json")}>JSON</Menu.Item>
        <Menu.Item class="text-xs" onclick={() => handleFileExport("jsonl")}>JSONL</Menu.Item>
        <Menu.Separator />
        <Menu.Item class="text-xs" onclick={() => handleFileExport("sql_insert")}>SQL INSERT</Menu.Item>
        <Menu.Item class="text-xs" onclick={() => handleFileExport("sql_script")}>SQL Script (DDL + INSERTs)</Menu.Item>
    </Menu.Content>
</Menu.Root>

<!-- Active/completed export chips -->
{#each exportStore.exports as entry (entry.exportId)}
    <ExportProgressChip {entry} />
{/each}
```

- [ ] **Step 5: Verify**

```bash
pnpm check 2>&1 | tail -5
```
Expected: `svelte-check found 0 errors`

- [ ] **Step 6: Commit**

```bash
git add src/lib/components/table/TableToolbar.svelte
git commit -m "feat(export): wire export dialog and progress chips into toolbar"
```

---

## Task 7: Pass connection context from `SqlResultPanel.svelte`

**Files:**
- Modify: `src/lib/components/SqlResultPanel.svelte`

The toolbar now needs `connectionId`, `sessionId`, `database`, `query`, `tableName` — these come from the result panel.

- [ ] **Step 1: Pass props to TableToolbar**

In `SqlResultPanel.svelte`, find where `<TableToolbar>` (or the Table component that renders the toolbar) is used and add the new props:

```svelte
connectionId={schemaStore.activeConnection?.id}
sessionId={view.id}
database={schemaStore.selectedDatabase ?? undefined}
query={results?.query}
tableName={results?.tableName}
```

(The exact prop names depend on what `SqlResultPanel` already has access to — match the variable names already in scope.)

- [ ] **Step 2: Verify**

```bash
pnpm check 2>&1 | tail -5
```
Expected: `svelte-check found 0 errors`

- [ ] **Step 3: Cargo build check**

```bash
cd src-tauri && cargo check 2>&1 | tail -5
```
Expected: no errors.

- [ ] **Step 4: Final commit**

```bash
git add src/lib/components/SqlResultPanel.svelte
git commit -m "feat(export): pass connection context to toolbar for file export"
```

---

## Task 8: Final integration check

- [ ] **Step 1: Full type check**

```bash
cd /Users/__deesh_reddy__/projects/personal_git/rust_builds/tables
pnpm check 2>&1 | tail -5
```
Expected: `svelte-check found 0 errors and 0 warnings`

- [ ] **Step 2: Rust build**

```bash
cd src-tauri && cargo build 2>&1 | tail -10
```
Expected: no errors.

- [ ] **Step 3: Manual smoke test checklist**

1. Run `pnpm tauri dev`
2. Open a connection, run a query that returns >100 rows
3. Click Export dropdown → CSV → pick file path → confirm chip appears in toolbar
4. While exporting, chip shows row count ticking and elapsed time
5. Export completes → chip shows green ✓ with filename and row count
6. Export a second query simultaneously — both chips visible
7. Start a large export, click ✗ cancel — chip shows "Cancelled", partial file deleted
8. Export as SQL Script — open the file, confirm CREATE TABLE DDL + INSERTs present
9. Export as JSONL — confirm one JSON object per line in output file

- [ ] **Step 4: Final commit**

```bash
git add -A
git commit -m "feat(export): export to file complete — streaming backend, toolbar chips, all formats"
```
