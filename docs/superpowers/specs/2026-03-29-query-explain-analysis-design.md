# Query Explain & Analysis — Design Spec

**Date:** 2026-03-29
**Status:** Approved

---

## Overview

A query explain and analysis feature for Tables. When the user clicks **Explain** in the editor toolbar, Tables runs `EXPLAIN (ANALYZE, BUFFERS, FORMAT JSON)` on the current statement, parses the result into a typed plan tree, and displays it in the bottom panel as a waterfall visualization. Detected issues (sequential scans, row estimate mismatches) are surfaced inline. An **Ask AI** button sends the plan and relevant schema context to the existing agent chat for on-demand analysis.

---

## Scope

### In scope
- `⚡ Explain` toolbar button (keyboard shortcut `Cmd+Shift+E`)
- New `explain_query` Tauri command returning typed `PlanNode` structs
- Bottom panel: **Explain** tab that activates when explain runs
- Waterfall visualization: horizontal bars per plan node, color-coded by % of total execution time, indented by tree depth
- Inline hint rows for row estimate mismatches
- Issues panel below the waterfall: actionable cards for sequential scans, stale stats, etc.
- **Ask AI** button: sends formatted plan summary + schema context to the agent chat panel
- PostgreSQL only; SQLite shows raw `EXPLAIN` text output with no visualization

### Out of scope (future)
- Depesz-style table view and Stats tab
- Explain options dropdown (VERBOSE, BUFFERS toggle, GENERIC PLAN)
- MySQL / MongoDB explain support
- Automatic explain on every query run

---

## Architecture

### Backend — new `explain_query` Tauri command

**File:** `src-tauri/src/commands/query_commands.rs` (new command) or a new `explain_commands.rs`

```rust
#[tauri::command]
async fn explain_query(
    connection_id: String,
    session_id: String,
    database: Option<String>,
    query: String,
    state: tauri::State<'_, AppState>,
) -> Result<ExplainResult, String>
```

The command:
1. Prepends `EXPLAIN (ANALYZE, BUFFERS, FORMAT JSON)` to the query
2. Executes via the existing `ConnectionManager` / `DatabaseAdapter`
3. Parses the returned JSON into a `PlanNode` tree
4. Computes derived fields: `exclusive_ms`, `pct_of_total`, `issues`
5. Returns `ExplainResult`

**Data types:**

```rust
#[derive(serde::Serialize)]
pub struct ExplainResult {
    pub planning_ms: f64,
    pub execution_ms: f64,
    pub total_rows: u64,
    pub plan: PlanNode,
    pub issues: Vec<PlanIssue>,
}

#[derive(serde::Serialize)]
pub struct PlanNode {
    pub node_type: String,       // "Seq Scan", "Index Scan", "Nested Loop", …
    pub relation_name: Option<String>,
    pub index_name: Option<String>,
    pub total_ms: f64,           // actual total time (inclusive)
    pub exclusive_ms: f64,       // time in this node only (excl. children)
    pub pct_of_total: f64,       // exclusive_ms / execution_ms * 100
    pub planned_rows: u64,
    pub actual_rows: u64,
    pub loops: u64,
    pub buffers_hit: Option<u64>,
    pub buffers_read: Option<u64>,
    pub depth: u32,              // tree depth, 0 = root
    pub children: Vec<PlanNode>,
}

#[derive(serde::Serialize)]
pub struct PlanIssue {
    pub severity: IssueSeverity,  // "danger" | "warning"
    pub kind: IssueKind,          // SeqScan | RowEstimateMismatch | StaleStats
    pub node_type: String,
    pub relation: Option<String>,
    pub message: String,          // human-readable description
    pub suggestion: String,       // actionable fix
}
```

**Issue detection rules (Rust):**
- `SeqScan` (danger): node_type == "Seq Scan" AND actual_rows > 1000
- `RowEstimateMismatch` (warning): actual_rows / planned_rows > 10× or < 0.1×
- Both checks run in a recursive pass over the `PlanNode` tree before serialization

**SQLite fallback:** The command detects `EngineProfile` and, for SQLite, runs plain `EXPLAIN` (no options, returns text rows) and returns an `ExplainResult` with `plan.node_type = "raw"` and the raw text in `message`. The frontend renders this as a plain code block.

---

### Frontend — components

#### `ExplainPanel.svelte`
New component at `src/lib/components/ExplainPanel.svelte`.

Props: `result: ExplainResult`

Renders:
- **Header row**: Execution Plan title + Planning / Execution / Rows stat chips + Ask AI button
- **Waterfall**: `WaterfallChart.svelte` (see below)
- **Divider**
- **Issues section**: renders `result.issues` as issue cards

#### `WaterfallChart.svelte`
New component at `src/lib/components/WaterfallChart.svelte`.

Props: `nodes: PlanNode[]` (flat, pre-ordered; depth field drives indentation)

Renders a flat list of rows (tree flattened depth-first). Each row:
- Label column (200px): indent (`depth × 16px`), severity icon, node name + relation
- Bar track: `background: var(--bg-tertiary)`, fill width = `node.pct_of_total%`
  - Fill color: `var(--danger)` if pct > 50, `var(--warning)` if pct > 10, `var(--success)` otherwise
- ms column: exclusive_ms formatted to 1 decimal
- pct column: pct_of_total formatted as integer %
- Hint row (conditional): shown when node has a `RowEstimateMismatch` issue — displays planned vs actual rows inline below the bar

Hovering a row highlights it (`var(--bg-hover)`). Clicking a node is reserved for a future detail drawer.

#### Integration into existing panels

**`QueryEditorToolbar.svelte`**: Add `⚡ Explain` button after the Run button. The `onExplain` callback prop already exists — just render the button. Uses `IconBolt` from `@tabler/icons-svelte`.

**`SqlTestingEditor.svelte`**: `handleExplain` already exists (line 885) but currently routes through `executeQueryText`. Replace with a new path: call `invoke('explain_query', ...)` directly, store result in a local `$state<ExplainResult | null>`, and push an `"explain"` result to the bottom panel.

**`BottomPanel.svelte`**: Add an "Explain" tab entry. Tab is only shown when the current editor view has an explain result. Badge shows issue count when `result.issues.length > 0`. When the tab is active, renders `<ExplainPanel result={explainResult} />` instead of the results table.

#### Ask AI integration

When the user clicks **Ask AI** in the explain panel header:

1. Format a message string:
   ```
   I ran EXPLAIN ANALYZE on this query:

   <query text>

   Execution plan summary:
   - Total execution: {execution_ms}ms
   - Slowest node: {node_type} on {relation} ({pct}% of time)
   - Issues: {issues.map(i => i.message).join('; ')}

   Please analyze this plan and suggest both query-level optimizations and
   table-level improvements (indexes, ANALYZE, etc.).
   ```
2. Add a `pendingMessage: string | null` field to `agentStore` (or `windowState`). Set it to the formatted message string.
3. Open the agent panel (via existing `windowState` mechanism).
4. `AgentPanel.svelte` reads `pendingMessage` on mount/effect — if set, auto-populates the composer input and clears the store field.
5. Agent has access to schema context (table columns, existing indexes) via existing tool infrastructure (`describe_table`, `list_tables`, etc.).

---

## Data Flow

```
User clicks ⚡ Explain
    → SqlTestingEditor.handleExplain()
    → invoke('explain_query', { connectionId, sessionId, database, query })
    → Rust: prepend EXPLAIN options, execute via ConnectionManager
    → Rust: parse JSON → PlanNode tree → detect issues → ExplainResult
    → Frontend: store explainResult in editor view state
    → BottomPanel: activate Explain tab, show badge if issues > 0
    → ExplainPanel renders WaterfallChart + issues cards

User clicks ✨ Ask AI
    → Format plan summary string
    → Open agent chat panel with pre-loaded message
    → Agent analyzes query + schema, returns suggestions
```

---

## Color Semantics

Follow the app's existing pattern: use Tailwind color classes for semantic colors, CSS custom properties for structural tokens.

| Semantic     | Tailwind class / token          | Usage                        |
|-------------|---------------------------------|------------------------------|
| Danger       | `text-red-400`, `bg-red-500/15`, `border-red-500/30` | Seq scans, >50% time nodes |
| Warning      | `text-orange-400`, `bg-orange-500/15`, `border-orange-500/30` | Row mismatches, 10–50% nodes |
| Success      | `text-green-500`, `bg-green-500/10` | Fast nodes, <10% time |
| Background   | `var(--color-tertiary)`         | Bar track background         |
| Border       | `var(--color-border-subtle)`    | Dividers                     |
| Muted text   | `var(--color-muted-foreground)` | Pct column, hint text        |

These tokens are already defined in `app.css` and follow the same pattern as `QueryEditorToolbar.svelte`.

---

## Icons

All from `@tabler/icons-svelte`:

| Location          | Icon                    |
|-------------------|-------------------------|
| Explain button    | `IconBolt`              |
| Explain tab       | `IconBolt`              |
| Ask AI button     | `IconSparkles`          |
| Seq scan issue    | `IconAlertTriangle`     |
| Row mismatch      | `IconInfoCircle`        |
| Fast node         | `IconCheck`             |
| Timing chips      | `IconClock`             |
| Panel title       | `IconReportAnalytics`   |

---

## Error Handling

- If `explain_query` fails (e.g. syntax error in query): show an error state in the Explain tab with the Postgres error message, same style as the existing query error display.
- If the query is not a `SELECT` / `WITH` statement: show a "EXPLAIN is only supported for SELECT queries" message.
- SQLite: show raw text output in a `<pre>` block with a note that full visualization requires PostgreSQL.

---

## Testing

**Rust unit tests** in `src-tauri/src/commands/explain_commands.rs`:
- Parse a known EXPLAIN JSON fixture → assert PlanNode fields
- Detect SeqScan issue on a plan with actual_rows > 1000
- Detect RowEstimateMismatch when ratio > 10×
- Recursive plans (nested children) flatten correctly

**Frontend**: manual QA — run explain on a query with a seq scan, verify waterfall colors and issue cards appear correctly.
