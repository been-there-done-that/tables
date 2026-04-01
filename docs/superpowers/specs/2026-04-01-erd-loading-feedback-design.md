# ERD Loading Feedback Design

**Goal:** Give users clear, non-intrusive feedback when opening large ERDs, and prevent silent hangs on 100+ table schemas.

**Architecture:** Confirmation gate in the table selector → fetching moves into ErdView → floating progress chip on the canvas.

**Tech Stack:** Svelte 5 runes, @xyflow/svelte, Tauri IPC (`get_schema_table_details`)

---

## Confirmation Gate

Shown inside `ErdTableSelector` when the user attempts to open an ERD with more than 50 tables selected.

- Threshold: **> 50 tables**
- Display: inline warning banner replacing the normal confirm button area
- Copy: `⚠ 260 tables selected — large schemas take longer to load.`
- Actions: `[ Open anyway ]` and `[ Cancel ]`
- Under 50 tables: no gate, opens immediately (no behaviour change)
- The gate is a soft warning — "Open anyway" always proceeds

## Data Fetching Moves Into ErdView

Currently `ExplorerToolbar.openErd()` calls `Promise.all(selected.map(get_schema_table_details))` — all N calls fire in parallel before the canvas opens. This is replaced:

- `openErd()` in the toolbar passes **lightweight table stubs** (no columns, no FKs — just `schema`, `table_name`, `database`) when calling `openView('erd', ...)`
- `ErdView` receives these stubs and owns the full fetch + layout lifecycle
- Fetching is **batched 20 at a time** using a sequential chunk loop to avoid overwhelming Tauri IPC

```
for each chunk of 20 tables:
  await Promise.all(chunk.map(get_schema_table_details))
  progress += chunk.length
```

## Progress Chip

A small floating chip anchored to the **bottom-left** of the canvas, visible only during loading.

**States (in order):**

| State | Text |
|---|---|
| Fetching | `⟳  Fetching tables  45 / 260` |
| Layout | `⟳  Computing layout...` |
| Done (1 s then fade) | `✓  Done` |
| Done with failures | `✓  258 tables, 2 failed` |

- Chip is absolutely positioned, `z-10`, bottom-left of the `.relative` canvas wrapper
- Uses `$state` for `fetched`, `total`, `phase: 'fetching' | 'layout' | 'done'`
- After `done`: 1 second timeout then sets `visible = false` (CSS opacity transition)
- Failures: tables where `get_schema_table_details` throws are silently skipped but counted; the chip reports them at the end

## Error Handling

- Per-table fetch errors are caught individually — one bad table does not abort the whole load
- If ELK layout throws, chip shows `✗  Layout failed` and the canvas stays empty with a retry button
