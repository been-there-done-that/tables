# Copy & Export — Design Spec
**Date:** 2026-04-01
**Status:** Ready for implementation

---

## Overview

Two related but distinct features:
1. **Copy Results** — copy selected cells/rows to clipboard in various formats, with persistent format preference and contextual cell/row actions
2. **Export to File** — download full query results to disk, backend-driven, non-blocking with a live progress indicator in the toolbar

---

## Part 1: Copy Results

### 1.1 Format Selector (Toolbar)

A dropdown in the results toolbar that sets the format for `Cmd+C` / `Ctrl+C`. Persists globally in settings (`settings.copyFormat`, default: `"plain"`).

**Formats (in dropdown order):**

| Label | Key | Output |
|-------|-----|--------|
| Plain | `plain` | Raw value for single cell; TSV (no header) for multi-cell — best for pasting into spreadsheets |
| TSV | `tsv` | Tab-separated with quotes and header |
| CSV | `csv` | Comma-separated with header |
| JSON | `json` | Array of objects `[{"col": val}]` |
| JSON (pretty) | `json_pretty` | Same, indented 2 spaces |
| SQL INSERT | `sql_insert` | `INSERT INTO table (cols) VALUES (...)` |
| Markdown | `markdown` | `\| col \| col \|` table with separator row |
| WHERE condition | `sql_where` | `WHERE col1 = val AND col2 = val` (full row as filter) |
| IN list | `sql_in` | `(val1, val2, val3)` — only available when single column selected |
| Column names | `column_names` | Space-separated column headers only |

**Keyboard shortcut:** `Cmd+C` uses the selected format. Format selector shows currently active format. Small persist icon indicates it's saved.

**Visual:** Compact dropdown in the results toolbar, left of the Export button. Shows format abbreviation (e.g. "TSV", "JSON") with a copy icon.

---

### 1.2 Context Menu (Right-click on selection)

Right-click on any cell or selection shows:

```
Copy                          Cmd+C     ← uses current format
Copy as →                               ← submenu with all formats
  Plain
  TSV
  CSV
  JSON
  JSON (pretty)
  SQL INSERT
  Markdown
  WHERE condition
  IN list (only if single column)
  Column names
──────────────────────────────
Copy Column Name
──────────────────────────────
Paste                         Cmd+V
──────────────────────────────
Set NULL                                ← only in editable grid, nullable column
Set Default                             ← only in editable grid, column has DEFAULT
Revert Cell                             ← only if cell has a pending edit
──────────────────────────────
Delete Row                              ← only in editable grid
──────────────────────────────
Filter by this value                    ← adds a quick filter for this cell value
```

---

### 1.3 Row/Cell Hover Actions

**Row hover (gutter):**
When hovering a row, show two icon buttons in the row number gutter (left side):
- `[ ⊕ ]` — Add row below (only in editable grid)
- `[ ✕ ]` — Delete row (only in editable grid)
- `[ ⎘ ]` — Copy row (uses current format)

**Cell hover (right edge of cell):**
When hovering an editable cell, show a small `⋯` button at the right edge. Clicking opens a mini popover:
- Set NULL (if column is nullable)
- Set Default (if column has a DEFAULT defined)
- Revert to original (if cell has a pending edit)
- Copy cell value

For read-only results (query output, not table browser), only show `⎘` copy cell icon on hover.

---

### 1.4 Format Behaviour Details

**`sql_insert`**
```sql
INSERT INTO users (id, name, email) VALUES (1, 'Alice', 'alice@example.com');
INSERT INTO users (id, name, email) VALUES (2, 'Bob', 'bob@example.com');
```
- Uses proper SQL quoting per type (numbers unquoted, strings single-quoted, NULLs as NULL)
- Requires table context (available in table browser; in query results use a generic `results` table name or prompt)

**`sql_where`**
```sql
WHERE id = 1 AND name = 'Alice' AND status = 'active'
```
- Joins all selected columns with AND
- Numeric values unquoted, strings single-quoted, NULLs become `IS NULL`
- For multi-row selection: wraps in `(row1 conditions) OR (row2 conditions)`

**`sql_in`**
```sql
(1, 2, 3, 4, 5)
```
- Only available when a single column is selected
- Numeric types unquoted, strings single-quoted
- Ready to paste directly after `WHERE id IN`

**`plain`**
- Single cell: raw value (no quotes)
- Multi-cell: TSV without header (matches Excel paste)

---

### 1.5 Copy Flash Feedback

After any copy action, the copied cells flash a brief highlight (`bg-primary/20`) for ~400ms. Already partially done — make consistent across all copy paths.

---

## Part 2: Export to File

### 2.1 User Flow

1. User clicks **Export** button in results toolbar
2. Dropdown appears:
   - CSV
   - TSV
   - JSON (array)
   - JSONL (one object per line)
   - SQL INSERT (rows only)
   - SQL Script (CREATE TABLE DDL + INSERTs)
3. User selects format → native file save dialog (Tauri `dialog::save`) pre-fills `{table_or_query}_{YYYY-MM-DD}.{ext}`
4. Export starts immediately — toolbar shows live progress indicator
5. User can continue working (other queries, tab switching — fully non-blocking)

---

### 2.2 Toolbar Progress Indicator

A compact status chip in the results toolbar replaces the Export button while an export is running:

```
[ ↓  Exporting… 12,400 rows  1.2 MB/s  00:04 ]  [✕]
```

- Animated download arrow icon (subtle pulse)
- Row count ticking up live
- Transfer rate (computed from bytes written / elapsed)
- Elapsed time counter (MM:SS)
- `✕` cancel button

**Clicking the chip** opens a detail popover (see 2.3).

**When complete**, the chip transitions to:
```
[ ✓  CSV saved  23,841 rows  ~/Downloads/users_2026-04-01.csv ]  [Open]  [Show in Finder]
```
Auto-dismisses after 8 seconds. User can click to keep it visible.

**If error:**
```
[ ⚠  Export failed — disk full ]  [Retry]  [✕]
```

---

### 2.3 Export Detail Popover

Clicking the toolbar chip opens a small popover panel showing:

```
┌─────────────────────────────────────────────────┐
│  Exporting CSV                         [Cancel]  │
│                                                  │
│  Query   SELECT * FROM orders WHERE …            │
│  File    ~/Downloads/orders_2026-04-01.csv       │
│  Status  Reading results                         │
│  Rows    12,400 written                          │
│  Size    4.2 MB                                  │
│  Rate    1.2 MB/s                                │
│  Time    00:04 elapsed                           │
│                                                  │
│  ████████████░░░░░░░░░░  ~42% (est.)             │
└─────────────────────────────────────────────────┘
```

Status transitions through:
1. `Connecting…` — re-opening connection
2. `Executing query…` — running the SQL
3. `Reading results` — streaming + writing file
4. `Done`

Progress bar is estimated if total row count is unknown (first pass counts, or uses LIMIT if query has one). If unestimatable, shows an indeterminate bar.

**Multiple simultaneous exports** (e.g. two tabs both exporting): the toolbar chip shows `[ ↓ 2 exports running ]`. Popover stacks both as separate cards.

---

### 2.4 Backend Architecture

**Tauri commands:**
```rust
// Start export — returns exportId immediately
invoke('start_export', {
  connectionId: string,
  sessionId: string,
  database: string,
  query: string,
  format: "csv" | "tsv" | "json" | "jsonl" | "sql_insert" | "sql_script",
  filePath: string,
  tableName?: string,  // for sql_insert/sql_script DDL header
})

// Cancel a running export
invoke('cancel_export', { exportId: string })
```

**Tauri events emitted during export:**
```rust
// Emitted every ~500ms or every 1000 rows (whichever comes first)
emit('export-progress', {
  exportId: string,
  rowsWritten: u64,
  bytesWritten: u64,
  elapsedMs: u64,
  status: "connecting" | "executing" | "streaming" | "done" | "error" | "cancelled",
  error?: string,
  filePath: string,
})
```

**Rust implementation:**
- Each export gets a UUID (`exportId`) and a `CancellationToken`
- Stored in a `DashMap<String, ExportHandle>` in `AppState`
- Uses a server-side cursor (PostgreSQL) or `LIMIT`/`OFFSET` chunks (SQLite/MySQL)
- Writes in chunks — never holds all rows in memory
- File written with `BufWriter` for performance
- On cancel: token is signalled, cursor dropped, partial file deleted

**SQL Script format:**
```sql
-- Exported from Tables on 2026-04-01
-- Query: SELECT * FROM users

CREATE TABLE IF NOT EXISTS users (
  id integer,
  name text,
  email text
);

INSERT INTO users (id, name, email) VALUES (1, 'Alice', 'alice@example.com');
INSERT INTO users (id, name, email) VALUES (2, 'Bob', 'bob@example.com');
-- ... (batched 500 rows per INSERT for large tables)
```
Column types inferred from result column metadata returned by the driver.

---

### 2.5 Format Details

| Format | Header | Encoding | Notes |
|--------|--------|----------|-------|
| CSV | Yes | UTF-8 BOM | RFC 4180, all fields quoted |
| TSV | Yes | UTF-8 | Tab-separated, no quoting |
| JSON | — | UTF-8 | `[{...}, {...}]` — entire array |
| JSONL | — | UTF-8 | One JSON object per line, streamable |
| SQL INSERT | — | UTF-8 | Batched 500 rows per statement |
| SQL Script | Yes (DDL) | UTF-8 | CREATE TABLE + batched INSERTs |

NULL values:
- CSV/TSV: empty field
- JSON/JSONL: `null`
- SQL: `NULL` keyword

---

## Open Questions / Decided

| Question | Decision |
|----------|----------|
| Re-execute or use cached results? | Re-execute (backend) — no size limit |
| Export button location | Results toolbar (right side) |
| Progress UI location | Inline in results toolbar chip → popover on click |
| Multiple exports | Supported, stacked in popover |
| Cancel behaviour | Partial file deleted on cancel |
| SQL INSERT table name in query results? | Use `results` as fallback; prompt user if no table context |
| Format persistence | Copy format: global setting. Export format: last-used in session |

---

## Implementation Order

1. **Copy formats** — extend existing clipboard code, add format selector dropdown, persist to settings
2. **Cell/row hover actions** — gutter icons + cell-edge Set NULL / Set Default
3. **Context menu** — right-click with full copy submenu + cell actions
4. **Export backend** — Rust `start_export` / `cancel_export` commands + progress events
5. **Export toolbar UI** — progress chip, detail popover, completion/error states
