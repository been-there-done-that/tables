# Copy Results Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend the results grid with a persistent copy-format selector, full copy-format support (TSV, CSV, JSON, SQL INSERT, WHERE condition, IN list, Markdown), a right-click context menu with all formats, and cell/row hover quick actions (Set NULL, Set Default, Revert, Delete Row).

**Architecture:** All logic is pure frontend. A new `copyFormats.ts` module owns all format serialisation. The `Settings` store gains a `copyFormat` field that persists across restarts. `Table.svelte` reads from settings and dispatches to `copyFormats.ts`. The toolbar gets a compact format-selector dropdown. `CellContextMenu.svelte` and `TableRow.svelte` get the new actions.

**Tech Stack:** SvelteKit / Svelte 5 runes, TypeScript, Tauri clipboard plugin (already wired), `@tabler/icons-svelte`, existing `$lib/components/ui/dropdown-menu`.

---

## File Map

| Action | File | What changes |
|--------|------|-------------|
| Create | `src/lib/components/table/copyFormats.ts` | Pure serialisation functions for every copy format |
| Modify | `src/lib/components/table/types.ts` | Extend `ClipboardFormat` union |
| Modify | `src/lib/stores/settings.svelte.ts` | Add `copyFormat` to `Settings` + default |
| Modify | `src/lib/components/table/Table.svelte` | Read `copyFormat` from settings, dispatch to `copyFormats.ts` |
| Modify | `src/lib/components/table/TableToolbar.svelte` | Format-selector dropdown |
| Modify | `src/lib/components/table/CellContextMenu.svelte` | "Copy as →" submenu + Set NULL / Set Default / Revert |
| Modify | `src/lib/components/table/TableRow.svelte` | Gutter hover icons (copy row, delete row) |
| Modify | `src/lib/components/table/TableCell.svelte` | Cell-edge hover icon (copy cell, Set NULL) |

---

## Task 1: Create `copyFormats.ts` — pure serialisation module

**Files:**
- Create: `src/lib/components/table/copyFormats.ts`

- [ ] **Step 1: Write the module**

```typescript
// src/lib/components/table/copyFormats.ts

export type CopyFormat =
    | "plain"         // raw value (single cell) or TSV no-header (multi)
    | "tsv"           // tab-separated with header
    | "csv"           // comma-separated with header
    | "json"          // JSON array of objects, compact
    | "json_pretty"   // JSON array of objects, indented
    | "sql_insert"    // INSERT INTO table (...) VALUES (...)
    | "markdown"      // | col | col | table
    | "sql_where"     // WHERE col = val AND col = val
    | "sql_in"        // (val1, val2, val3)  — single column only
    | "column_names"; // space-separated column headers

export const COPY_FORMAT_LABELS: Record<CopyFormat, string> = {
    plain:        "Plain",
    tsv:          "TSV",
    csv:          "CSV",
    json:         "JSON",
    json_pretty:  "JSON (pretty)",
    sql_insert:   "SQL INSERT",
    markdown:     "Markdown",
    sql_where:    "WHERE condition",
    sql_in:       "IN list",
    column_names: "Column names",
};

export interface CopyColumn {
    id: string;
    label: string;
    type?: string; // ColumnType string from types.ts
}

export interface CopyOptions {
    tableName?: string; // required for sql_insert
}

// --- helpers ---

const NUMERIC_TYPES = new Set([
    "int", "integer", "smallint", "bigint", "serial", "bigserial",
    "float", "float4", "float8", "double", "double precision",
    "real", "numeric", "decimal", "number",
]);

function isNumeric(col: CopyColumn): boolean {
    return NUMERIC_TYPES.has((col.type ?? "").toLowerCase());
}

function sqlQuote(value: any, col: CopyColumn): string {
    if (value === null || value === undefined) return "NULL";
    if (typeof value === "boolean") return value ? "TRUE" : "FALSE";
    if (isNumeric(col) && !isNaN(Number(value))) return String(value);
    const escaped = String(value).replace(/'/g, "''");
    return `'${escaped}'`;
}

function escapeMarkdown(s: string): string {
    return String(s).replace(/\|/g, "\\|");
}

// --- main export ---

/**
 * Serialise `rows` × `columns` into the requested copy format.
 * Returns the string to place on the clipboard.
 *
 * Throws if `format === "sql_in"` and `columns.length !== 1`.
 * Throws if `format === "sql_insert"` and `options.tableName` is not provided (falls back to "results").
 */
export function formatForCopy(
    rows: Record<string, any>[],
    columns: CopyColumn[],
    format: CopyFormat,
    options: CopyOptions = {},
): string {
    if (rows.length === 0 || columns.length === 0) return "";

    const tableName = options.tableName ?? "results";

    switch (format) {
        case "plain": {
            if (rows.length === 1 && columns.length === 1) {
                const v = rows[0][columns[0].id];
                return v === null || v === undefined ? "" : String(v);
            }
            // Fall through to TSV without header
            return rows
                .map((r) => columns.map((c) => {
                    const v = r[c.id];
                    return v === null || v === undefined ? "" : String(v);
                }).join("\t"))
                .join("\n");
        }

        case "tsv": {
            const header = columns.map((c) => c.label).join("\t");
            const body = rows.map((r) =>
                columns.map((c) => {
                    const v = r[c.id];
                    if (v === null || v === undefined) return "";
                    const s = String(v);
                    // Quote if contains tab, newline, or quote
                    if (s.includes("\t") || s.includes("\n") || s.includes('"')) {
                        return `"${s.replace(/"/g, '""')}"`;
                    }
                    return s;
                }).join("\t")
            ).join("\n");
            return `${header}\n${body}`;
        }

        case "csv": {
            const quoteCSV = (v: any): string => {
                if (v === null || v === undefined) return "";
                const s = String(v);
                if (s.includes(",") || s.includes('"') || s.includes("\n")) {
                    return `"${s.replace(/"/g, '""')}"`;
                }
                return s;
            };
            const header = columns.map((c) => quoteCSV(c.label)).join(",");
            const body = rows.map((r) =>
                columns.map((c) => quoteCSV(r[c.id])).join(",")
            ).join("\n");
            return `${header}\n${body}`;
        }

        case "json": {
            const arr = rows.map((r) =>
                Object.fromEntries(columns.map((c) => [c.label, r[c.id] ?? null]))
            );
            return JSON.stringify(arr);
        }

        case "json_pretty": {
            const arr = rows.map((r) =>
                Object.fromEntries(columns.map((c) => [c.label, r[c.id] ?? null]))
            );
            return JSON.stringify(arr, null, 2);
        }

        case "sql_insert": {
            const colNames = columns.map((c) => c.label).join(", ");
            return rows
                .map((r) => {
                    const vals = columns.map((c) => sqlQuote(r[c.id], c)).join(", ");
                    return `INSERT INTO ${tableName} (${colNames}) VALUES (${vals});`;
                })
                .join("\n");
        }

        case "markdown": {
            const header = `| ${columns.map((c) => escapeMarkdown(c.label)).join(" | ")} |`;
            const sep    = `| ${columns.map(() => "---").join(" | ")} |`;
            const body   = rows.map((r) =>
                `| ${columns.map((c) => escapeMarkdown(
                    r[c.id] === null || r[c.id] === undefined ? "" : String(r[c.id])
                )).join(" | ")} |`
            ).join("\n");
            return `${header}\n${sep}\n${body}`;
        }

        case "sql_where": {
            const rowCondition = (r: Record<string, any>) =>
                columns
                    .map((c) => {
                        const v = r[c.id];
                        if (v === null || v === undefined) return `${c.label} IS NULL`;
                        return `${c.label} = ${sqlQuote(v, c)}`;
                    })
                    .join(" AND ");

            if (rows.length === 1) {
                return `WHERE ${rowCondition(rows[0])}`;
            }
            return rows.map((r) => `(${rowCondition(r)})`).join(" OR ");
        }

        case "sql_in": {
            if (columns.length !== 1) {
                throw new Error("sql_in format requires exactly one column selected");
            }
            const col = columns[0];
            const vals = rows.map((r) => sqlQuote(r[col.id], col)).join(",\n");
            return `(\n${vals}\n)`;
        }

        case "column_names": {
            return columns.map((c) => c.label).join(" ");
        }
    }
}
```

- [ ] **Step 2: Verify TypeScript compiles**

```bash
cd /Users/__deesh_reddy__/projects/personal_git/rust_builds/tables
pnpm check 2>&1 | tail -5
```
Expected: `svelte-check found 0 errors`

- [ ] **Step 3: Commit**

```bash
git add src/lib/components/table/copyFormats.ts
git commit -m "feat(copy): add copyFormats.ts with all serialisation formats"
```

---

## Task 2: Extend `ClipboardFormat` and add `copyFormat` to settings

**Files:**
- Modify: `src/lib/components/table/types.ts`
- Modify: `src/lib/stores/settings.svelte.ts`

- [ ] **Step 1: Update `ClipboardFormat` in `types.ts`**

Find the existing `ClipboardFormat` type (currently `"tsv" | "csv" | "json"`) and replace it:

```typescript
// src/lib/components/table/types.ts  (replace existing ClipboardFormat line)
export type ClipboardFormat = import("./copyFormats").CopyFormat;
```

This re-exports `CopyFormat` under the existing name so all existing call sites that import `ClipboardFormat` keep working without changes.

- [ ] **Step 2: Add `copyFormat` to `Settings` interface in `settings.svelte.ts`**

In `src/lib/stores/settings.svelte.ts`, find `export interface Settings {` and add one field:

```typescript
    copyFormat: import("../components/table/copyFormats").CopyFormat;
```

Then find `const DEFAULT_SETTINGS: Settings = {` and add:

```typescript
    copyFormat: "plain",
```

- [ ] **Step 3: Verify**

```bash
pnpm check 2>&1 | tail -5
```
Expected: `svelte-check found 0 errors`

- [ ] **Step 4: Commit**

```bash
git add src/lib/components/table/types.ts src/lib/stores/settings.svelte.ts
git commit -m "feat(copy): add copyFormat to ClipboardFormat type and settings store"
```

---

## Task 3: Update `Table.svelte` to use `copyFormats.ts`

**Files:**
- Modify: `src/lib/components/table/Table.svelte`

The current `handleCopy` (around line 2028) is a hand-rolled inline implementation. Replace it with a call to `formatForCopy`.

- [ ] **Step 1: Add import at the top of `Table.svelte`'s `<script>` block**

Find the existing imports and add:

```typescript
import { formatForCopy, type CopyFormat } from "./copyFormats";
import { settingsStore } from "$lib/stores/settings.svelte";
```

(If `settingsStore` is already imported, skip that line.)

- [ ] **Step 2: Replace `clipboardFormat` local state with a derived from settings**

Find:
```typescript
let clipboardFormat: ClipboardFormat = "tsv";
let includeHeaders = true;
```

Replace with:
```typescript
// Format is now driven by persisted settings; local var kept for toolbar two-way binding
let clipboardFormat = $derived(settingsStore.settings.copyFormat);
let includeHeaders = true; // keep for future toggle; currently always true
```

- [ ] **Step 3: Replace `handleCopy` implementation**

Find the existing `handleCopy` function (around line 2028) and replace its body:

```typescript
function handleCopy() {
    const bounds = getActiveBounds();
    if (!bounds) return;

    const rowsSlice = filteredRows.slice(bounds.top, bounds.bottom + 1);
    const colsSlice = visibleColumns.slice(bounds.left, bounds.right + 1);
    if (!rowsSlice.length || !colsSlice.length) return;

    const copyColumns = colsSlice.map((c) => ({
        id: c.id,
        label: c.label ?? c.id,
        type: c.type,
    }));

    let text: string;
    try {
        text = formatForCopy(rowsSlice, copyColumns, clipboardFormat, {
            tableName: tableContext?.tableName,
        });
    } catch (e: any) {
        // e.g. sql_in with multiple columns — silently fallback to plain
        text = formatForCopy(rowsSlice, copyColumns, "plain", {});
    }

    writeClipboardText(text);
}
```

- [ ] **Step 4: Expose `copyForFormat` helper for context menu**

Add a new exported function right after `copySelection`:

```typescript
/** Copy current selection using an explicit format (used by context menu). */
export function copySelectionAs(format: CopyFormat) {
    const bounds = getActiveBounds();
    if (!bounds) return;

    const rowsSlice = filteredRows.slice(bounds.top, bounds.bottom + 1);
    const colsSlice = visibleColumns.slice(bounds.left, bounds.right + 1);
    if (!rowsSlice.length || !colsSlice.length) return;

    const copyColumns = colsSlice.map((c) => ({
        id: c.id,
        label: c.label ?? c.id,
        type: c.type,
    }));

    let text: string;
    try {
        text = formatForCopy(rowsSlice, copyColumns, format, {
            tableName: tableContext?.tableName,
        });
    } catch {
        text = formatForCopy(rowsSlice, copyColumns, "plain", {});
    }
    writeClipboardText(text);
}
```

- [ ] **Step 5: Verify**

```bash
pnpm check 2>&1 | tail -5
```
Expected: `svelte-check found 0 errors`

- [ ] **Step 6: Commit**

```bash
git add src/lib/components/table/Table.svelte
git commit -m "feat(copy): wire Table.svelte to copyFormats.ts and persisted format setting"
```

---

## Task 4: Format-selector dropdown in `TableToolbar.svelte`

**Files:**
- Modify: `src/lib/components/table/TableToolbar.svelte`

- [ ] **Step 1: Add imports**

At the top of the `<script>` block, add:

```typescript
import { COPY_FORMAT_LABELS, type CopyFormat } from "./copyFormats";
import { settingsStore } from "$lib/stores/settings.svelte";
import IconClipboardCopy from "@tabler/icons-svelte/icons/clipboard-copy";
```

- [ ] **Step 2: Add a `copyFormat` prop and two-way binding**

In the `Props` interface, add:

```typescript
onCopyFormatChange?: (format: CopyFormat) => void;
```

In the destructured props:
```typescript
let { ..., onCopyFormatChange }: Props = $props();
```

- [ ] **Step 3: Add the format selector dropdown to the toolbar template**

Find the area in the toolbar template where the Export button lives (around the `<IconDownload>` section). Add the format selector **before** the export dropdown:

```svelte
<!-- Copy format selector -->
<Menu.Root>
    <Menu.Trigger asChild let:builder>
        <Button
            builders={[builder]}
            variant="ghost"
            size="sm"
            class="h-7 gap-1 px-2 text-xs text-muted-foreground font-mono"
            title="Copy format (Cmd+C)"
        >
            <IconClipboardCopy class="h-3.5 w-3.5" />
            {COPY_FORMAT_LABELS[settingsStore.settings.copyFormat]}
            <IconChevronDown class="h-3 w-3 opacity-60" />
        </Button>
    </Menu.Trigger>
    <Menu.Content class="min-w-[160px]">
        {#each (Object.entries(COPY_FORMAT_LABELS) as [fmt, label])}
            <Menu.Item
                class="text-xs flex items-center gap-2"
                onclick={() => {
                    settingsStore.updateSetting("copyFormat", fmt as CopyFormat);
                    onCopyFormatChange?.(fmt as CopyFormat);
                }}
            >
                {#if settingsStore.settings.copyFormat === fmt}
                    <IconCheck class="h-3.5 w-3.5 text-primary" />
                {:else}
                    <span class="w-3.5" />
                {/if}
                {label}
            </Menu.Item>
        {/each}
    </Menu.Content>
</Menu.Root>
```

- [ ] **Step 4: Verify**

```bash
pnpm check 2>&1 | tail -5
```
Expected: `svelte-check found 0 errors`

- [ ] **Step 5: Commit**

```bash
git add src/lib/components/table/TableToolbar.svelte
git commit -m "feat(copy): add persistent copy-format selector to results toolbar"
```

---

## Task 5: Extend `CellContextMenu.svelte` with copy submenu

**Files:**
- Modify: `src/lib/components/table/CellContextMenu.svelte`

- [ ] **Step 1: Update Props to add copy-as and revert callbacks**

Find the `interface Props` block and extend it:

```typescript
interface Props {
    x: number;
    y: number;
    onEdit: () => void;
    onCopy: () => void;
    onCopyAs: (format: CopyFormat) => void;   // NEW
    onPaste: () => void;
    onSetNull: () => void;
    onSetDefault: () => void;
    onRevertCell: () => void;                  // NEW
    onDeleteRow: () => void;
    onFilterByValue: () => void;               // NEW
    onClose: () => void;
    // context flags
    isEditable?: boolean;                      // NEW — hides edit-only actions in read-only results
    isNullable?: boolean;                      // NEW — show Set NULL only when column is nullable
    hasDefault?: boolean;                      // NEW — show Set Default only when column has DEFAULT
    hasPendingEdit?: boolean;                  // NEW — show Revert only when cell is dirty
    isSingleColumn?: boolean;                  // NEW — show IN list only for single-column selections
}
```

Update the destructuring:

```typescript
let {
    x, y,
    onEdit, onCopy, onCopyAs, onPaste,
    onSetNull, onSetDefault, onRevertCell, onDeleteRow, onFilterByValue,
    onClose,
    isEditable = false,
    isNullable = false,
    hasDefault = false,
    hasPendingEdit = false,
    isSingleColumn = false,
}: Props = $props();
```

- [ ] **Step 2: Add import for formats**

```typescript
import { COPY_FORMAT_LABELS, type CopyFormat } from "./copyFormats";
```

- [ ] **Step 3: Replace the context menu template**

Replace the menu content with the full structured menu:

```svelte
<div
    bind:this={menuEl}
    use:handleClickOutside
    tabindex="-1"
    class="fixed z-50 min-w-[200px] rounded-md border border-border bg-popover p-1 shadow-md text-sm"
    style="left: {x}px; top: {y}px;"
>
    <!-- Copy (uses current format) -->
    <button class="menu-item" onclick={() => { onCopy(); onClose(); }}>
        <IconCopy class="h-3.5 w-3.5" />
        <span>Copy</span>
        <span class="ml-auto text-xs text-muted-foreground">⌘C</span>
    </button>

    <!-- Copy as submenu -->
    <div class="relative group">
        <button class="menu-item">
            <IconClipboard class="h-3.5 w-3.5" />
            <span>Copy as</span>
            <span class="ml-auto text-xs text-muted-foreground">▶</span>
        </button>
        <div class="absolute left-full top-0 hidden group-hover:block min-w-[160px] rounded-md border border-border bg-popover p-1 shadow-md z-50">
            {#each (Object.entries(COPY_FORMAT_LABELS) as [fmt, label])}
                {#if fmt !== "sql_in" || isSingleColumn}
                    <button
                        class="menu-item text-xs"
                        onclick={() => { onCopyAs(fmt as CopyFormat); onClose(); }}
                    >
                        {label}
                    </button>
                {/if}
            {/each}
        </div>
    </div>

    <div class="my-1 h-px bg-border" />

    <!-- Paste -->
    <button class="menu-item" onclick={() => { onPaste(); onClose(); }}>
        <IconClipboard class="h-3.5 w-3.5" />
        <span>Paste</span>
        <span class="ml-auto text-xs text-muted-foreground">⌘V</span>
    </button>

    {#if isEditable}
        <div class="my-1 h-px bg-border" />

        {#if isNullable}
            <button class="menu-item" onclick={() => { onSetNull(); onClose(); }}>
                <IconBan class="h-3.5 w-3.5" />
                <span>Set NULL</span>
            </button>
        {/if}

        {#if hasDefault}
            <button class="menu-item" onclick={() => { onSetDefault(); onClose(); }}>
                <IconRestore class="h-3.5 w-3.5" />
                <span>Set Default</span>
            </button>
        {/if}

        {#if hasPendingEdit}
            <button class="menu-item" onclick={() => { onRevertCell(); onClose(); }}>
                <IconRestore class="h-3.5 w-3.5" />
                <span>Revert cell</span>
            </button>
        {/if}

        <div class="my-1 h-px bg-border" />

        <button class="menu-item text-red-400 hover:text-red-300" onclick={() => { onDeleteRow(); onClose(); }}>
            <IconPlayerStop class="h-3.5 w-3.5" />
            <span>Delete row</span>
        </button>
    {/if}

    <div class="my-1 h-px bg-border" />

    <button class="menu-item text-xs text-muted-foreground" onclick={() => { onFilterByValue(); onClose(); }}>
        <span>Filter by this value</span>
    </button>
</div>

<style>
    .menu-item {
        @apply flex w-full items-center gap-2 rounded px-2 py-1.5 text-sm hover:bg-accent hover:text-accent-foreground cursor-default;
    }
</style>
```

- [ ] **Step 4: Wire new props in `Table.svelte`**

In `Table.svelte`, find where `<CellContextMenu>` is rendered and add the new props:

```svelte
<CellContextMenu
    x={contextMenuState.x}
    y={contextMenuState.y}
    onEdit={contextEdit}
    onCopy={contextCopy}
    onCopyAs={(fmt) => copySelectionAs(fmt)}
    onPaste={contextPaste}
    onSetNull={contextSetNull}
    onSetDefault={contextSetDefault}
    onRevertCell={contextRevertCell}
    onDeleteRow={contextDeleteRow}
    onFilterByValue={contextFilterByValue}
    onClose={() => (contextMenuState = null)}
    isEditable={isEditableGrid}
    isNullable={contextColumnIsNullable}
    hasDefault={contextColumnHasDefault}
    hasPendingEdit={contextCellHasPendingEdit}
    isSingleColumn={getActiveBounds()?.left === getActiveBounds()?.right}
/>
```

Add the missing context helpers near the other `context*` functions in `Table.svelte`:

```typescript
function contextRevertCell() {
    if (contextMenuState === null) return;
    const { rowIndex, columnIndex } = contextMenuState;
    const row = filteredRows[rowIndex];
    const col = visibleColumns[columnIndex];
    if (!row || !col) return;
    editManager.revertCell(getRowId(row), col.id);
}

function contextFilterByValue() {
    if (contextMenuState === null) return;
    const { rowIndex, columnIndex } = contextMenuState;
    const row = filteredRows[rowIndex];
    const col = visibleColumns[columnIndex];
    if (!row || !col) return;
    const value = row[col.id];
    // Trigger the existing filter mechanism with this column+value
    onFilterByValue?.({ columnId: col.id, value });
}

// Derived flags for context menu
const contextColumnIsNullable = $derived(() => {
    if (!contextMenuState) return false;
    const col = visibleColumns[contextMenuState.columnIndex];
    return col?.nullable ?? true;
});

const contextColumnHasDefault = $derived(() => {
    if (!contextMenuState) return false;
    const col = visibleColumns[contextMenuState.columnIndex];
    return col?.hasDefault ?? false;
});

const contextCellHasPendingEdit = $derived(() => {
    if (!contextMenuState) return false;
    const row = filteredRows[contextMenuState.rowIndex];
    const col = visibleColumns[contextMenuState.columnIndex];
    if (!row || !col) return false;
    return editManager.hasPendingEdit(getRowId(row), col.id);
});
```

Also add `onFilterByValue` to the Props interface in `Table.svelte` if not already there:

```typescript
onFilterByValue?: (params: { columnId: string; value: any }) => void;
```

And add `revertCell` and `hasPendingEdit` to `TableEditManager.svelte.ts`:

```typescript
// In TableEditManager class:
hasPendingEdit(rowId: any, columnId: string): boolean {
    const rId = String(rowId);
    return !!(this.pendingEdits[rId] && columnId in this.pendingEdits[rId]);
}

revertCell(rowId: any, columnId: string) {
    const rId = String(rowId);
    const current = { ...this.pendingEdits };
    if (current[rId]) {
        delete current[rId][columnId];
        if (Object.keys(current[rId]).length === 0) delete current[rId];
        this.pendingEdits = current;
    }
    this.originalValues.delete(`${rId}:${columnId}`);
}
```

- [ ] **Step 5: Verify**

```bash
pnpm check 2>&1 | tail -5
```
Expected: `svelte-check found 0 errors`

- [ ] **Step 6: Commit**

```bash
git add src/lib/components/table/CellContextMenu.svelte \
        src/lib/components/table/Table.svelte \
        src/lib/components/table/TableEditManager.svelte.ts
git commit -m "feat(copy): full copy-as context menu and cell revert action"
```

---

## Task 6: Row gutter hover actions

**Files:**
- Modify: `src/lib/components/table/TableRow.svelte`

- [ ] **Step 1: Add hover state and gutter icons**

In `TableRow.svelte`, find the row number / gutter cell (the first `<td>` with the row index) and extend it:

```svelte
<script lang="ts">
    // add to existing props:
    interface Props {
        // ... existing props ...
        onCopyRow?: () => void;
        onDeleteRow?: () => void;
        isEditable?: boolean;
    }
    let { ..., onCopyRow, onDeleteRow, isEditable = false }: Props = $props();

    import IconCopy from "@tabler/icons-svelte/icons/copy";
    import IconTrash from "@tabler/icons-svelte/icons/trash";

    let rowHovered = $state(false);
</script>

<!-- Row element: add onmouseenter/leave -->
<tr
    ...existing attributes...
    onmouseenter={() => rowHovered = true}
    onmouseleave={() => rowHovered = false}
>
    <!-- Gutter cell (row number) -->
    <td class="relative w-10 select-none text-center text-xs text-muted-foreground border-r border-border/50 bg-muted/20">
        {#if rowHovered}
            <div class="absolute inset-0 flex items-center justify-center gap-0.5">
                <button
                    class="h-5 w-5 rounded hover:bg-accent flex items-center justify-center"
                    title="Copy row"
                    onclick={(e) => { e.stopPropagation(); onCopyRow?.(); }}
                >
                    <IconCopy class="h-3 w-3" />
                </button>
                {#if isEditable}
                    <button
                        class="h-5 w-5 rounded hover:bg-red-500/20 flex items-center justify-center"
                        title="Delete row"
                        onclick={(e) => { e.stopPropagation(); onDeleteRow?.(); }}
                    >
                        <IconTrash class="h-3 w-3 text-red-400" />
                    </button>
                {/if}
            </div>
        {:else}
            {rowIndex + 1}
        {/if}
    </td>
    <!-- ... rest of cells ... -->
</tr>
```

- [ ] **Step 2: Wire `onCopyRow` from `Table.svelte`**

In `Table.svelte`, find where `<TableRow>` is rendered inside `TableBody` and pass:

```svelte
onCopyRow={() => {
    // copy this single row using current format
    const col = visibleColumns;
    const copyColumns = col.map((c) => ({ id: c.id, label: c.label ?? c.id, type: c.type }));
    const text = formatForCopy([row], copyColumns, clipboardFormat, { tableName: tableContext?.tableName });
    writeClipboardText(text);
}}
onDeleteRow={() => handleDeleteRow(rowIndex)}
isEditable={isEditableGrid}
```

- [ ] **Step 3: Verify**

```bash
pnpm check 2>&1 | tail -5
```
Expected: `svelte-check found 0 errors`

- [ ] **Step 4: Commit**

```bash
git add src/lib/components/table/TableRow.svelte src/lib/components/table/Table.svelte
git commit -m "feat(copy): row gutter hover actions — copy row and delete row"
```

---

## Task 7: Cell-edge hover icon (copy cell, Set NULL)

**Files:**
- Modify: `src/lib/components/table/TableCell.svelte`

- [ ] **Step 1: Add cell hover state and edge icon**

```svelte
<script lang="ts">
    // add to existing props:
    interface Props {
        // ... existing ...
        onCopyCell?: () => void;
        onSetNull?: () => void;
        isEditable?: boolean;
        isNullable?: boolean;
    }
    let { ..., onCopyCell, onSetNull, isEditable = false, isNullable = false }: Props = $props();

    import IconCopy from "@tabler/icons-svelte/icons/copy";
    import IconSlash from "@tabler/icons-svelte/icons/slash";

    let cellHovered = $state(false);
</script>

<td
    ...existing attributes...
    class="relative ..."
    onmouseenter={() => cellHovered = true}
    onmouseleave={() => cellHovered = false}
>
    <!-- existing cell content -->
    ...

    <!-- hover action icons at right edge -->
    {#if cellHovered && !isEditing}
        <div class="absolute right-0.5 top-1/2 -translate-y-1/2 flex items-center gap-0.5 bg-background/80 rounded px-0.5">
            <button
                class="h-4 w-4 rounded hover:bg-accent flex items-center justify-center opacity-70 hover:opacity-100"
                title="Copy cell value"
                onclick={(e) => { e.stopPropagation(); onCopyCell?.(); }}
            >
                <IconCopy class="h-3 w-3" />
            </button>
            {#if isEditable && isNullable}
                <button
                    class="h-4 w-4 rounded hover:bg-accent flex items-center justify-center opacity-70 hover:opacity-100"
                    title="Set NULL"
                    onclick={(e) => { e.stopPropagation(); onSetNull?.(); }}
                >
                    <IconSlash class="h-3 w-3 text-muted-foreground" />
                </button>
            {/if}
        </div>
    {/if}
</td>
```

- [ ] **Step 2: Wire from `Table.svelte`/`TableBody.svelte`**

Pass down to each `<TableCell>`:

```svelte
onCopyCell={() => {
    const text = value === null || value === undefined ? "" : String(value);
    writeClipboardText(text);
}}
onSetNull={() => editManager.setPendingEdit(getRowId(row), col.id, null, row[col.id])}
isEditable={isEditableGrid}
isNullable={col.nullable ?? true}
```

- [ ] **Step 3: Verify**

```bash
pnpm check 2>&1 | tail -5
```
Expected: `svelte-check found 0 errors`

- [ ] **Step 4: Commit**

```bash
git add src/lib/components/table/TableCell.svelte
git commit -m "feat(copy): cell-edge hover icons for copy cell and set null"
```

---

## Task 8: Final integration check

- [ ] **Step 1: Run full type check**

```bash
cd /Users/__deesh_reddy__/projects/personal_git/rust_builds/tables
pnpm check 2>&1 | tail -5
```
Expected: `svelte-check found 0 errors and 0 warnings`

- [ ] **Step 2: Manual smoke test checklist**

Open the app in dev mode (`pnpm tauri dev`):
1. Open a query result with multiple rows
2. Select a range — press `Cmd+C` — paste into a text editor — confirm TSV format
3. Change format selector to "JSON" — `Cmd+C` — confirm JSON array
4. Change to "WHERE condition" — select 2 rows — confirm `(id = 1 AND ...) OR (id = 2 AND ...)`
5. Right-click a cell — confirm "Copy as" submenu appears with all formats
6. Hover a row — confirm gutter shows copy + delete icons
7. Hover a cell — confirm right-edge copy icon appears
8. Settings persist: close app, reopen — format selector should remember last choice

- [ ] **Step 3: Final commit**

```bash
git add -A
git commit -m "feat(copy): copy results — format selector, context menu, hover actions complete"
```
