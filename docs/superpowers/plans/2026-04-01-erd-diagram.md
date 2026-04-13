# ERD Diagram View — Implementation Plan

**Date:** 2026-04-01
**Feature:** Entity Relationship Diagram canvas integrated into the Tables desktop app
**Stack:** SvelteKit + Svelte 5 runes, Tauri 2, `@xyflow/svelte`, `@dagrejs/dagre`, `bits-ui`

---

## Overview

This plan adds an ERD (Entity Relationship Diagram) view to Tables. The user clicks an ERD icon in the `ExplorerToolbar`, picks which tables to include via a searchable dialog (with FK-based auto-expansion), and then sees a Supabase-style canvas with table cards, per-column FK edges, a minimap, and position persistence.

**Seven tasks, each with complete code and no placeholders.**

---

## Pre-read Verification Checklist

Before starting, the implementer should read these files to verify exact shapes:

- `/src/lib/commands/types.ts` — confirm `MetaDatabase.name` (NOT `database_name`), `MetaSchema.name`, `MetaTable.schema`, `MetaForeignKey` fields
- `/src/lib/stores/schema.svelte.ts` — confirm `schemaStore.databases`, `schemaStore.selectedDatabase`, `schemaStore.activeConnection`, `schemaStore.activeSchema`
- `/src/lib/stores/session.svelte.ts` — confirm `ViewType` union, `openView` signature
- `/src/lib/components/explorer/ExplorerToolbar.svelte` — confirm existing imports, especially whether `schemaStore` is already imported (it is — line 4)
- `/src/routes/+page.svelte` — confirm the view-switch block around lines 111–120

**Critical shape notes discovered during plan authoring:**
- `MetaDatabase` uses `.name` (not `.database_name`)
- `MetaSchema` uses `.name` (not a separate schema identifier)
- `schemaStore.activeConnection` is `Connection | null` — `.id` is the connection ID
- The existing fallback `{:else}` in `+page.svelte` catches unknown view types; Task 1 adds an explicit `{:else if view.type === "erd"}` branch before it

---

## Constants

```typescript
export const TABLE_NODE_WIDTH = 220;
export const TABLE_HEADER_HEIGHT = 36;
export const COLUMN_ROW_HEIGHT = 24;
```

## ID Formats

- **Node ID:** `${schema}.${table_name}` — e.g., `"public.users"`
- **Edge ID:** `${fk.schema}.${fk.table_name}.${fk.column_name}->${fk.ref_schema ?? fk.schema}.${fk.ref_table}.${fk.ref_column}`

---

## Task 1: Install deps + fix types + add ViewType "erd"

### Goal
Add required npm packages, extend `MetaForeignKey` with the missing `ref_schema` field, add `"erd"` to the `ViewType` union, and add a placeholder ERD branch to the page view-switch so the router is ready.

### Files

**1a. Install packages (run in repo root):**
```bash
pnpm add @xyflow/svelte @dagrejs/dagre
```

**1b. `src/lib/commands/types.ts` — add `ref_schema?: string` to `MetaForeignKey`:**

Find the existing `MetaForeignKey` interface:
```typescript
export interface MetaForeignKey {
  connection_id: string;
  database: string;
  schema: string;
  table_name: string;
  column_name: string;
  ref_table: string;
  ref_column: string;
}
```

Replace with:
```typescript
export interface MetaForeignKey {
  connection_id: string;
  database: string;
  schema: string;        // source schema
  table_name: string;    // source table
  column_name: string;   // source column
  ref_table: string;     // target table
  ref_column: string;    // target column
  ref_schema?: string;   // target schema (optional; defaults to source schema)
}
```

**1c. `src/lib/stores/session.svelte.ts` — extend `ViewType`:**

Find:
```typescript
export type ViewType = "editor" | "table" | "custom";
```

Replace with:
```typescript
export type ViewType = "editor" | "table" | "custom" | "erd";
```

**1d. `src/routes/+page.svelte` — add ERD placeholder branch:**

Find the view-switch block (around line 111):
```svelte
{#if view.type === "editor"}
  <SqlTestingEditor
    id={view.id}
    context={view.data}
    {view}
  />
{:else if view.type === "table"}
  <TablePreview bind:context={view.data} />
{:else}
  <!-- Default Fallback -->
  <div class="flex-1 overflow-auto p-4 space-y-4">
    <pre class="p-4 bg-muted/30 rounded border border-border text-xs">View ID: {view.id}</pre>
  </div>
{/if}
```

Replace with:
```svelte
{#if view.type === "editor"}
  <SqlTestingEditor
    id={view.id}
    context={view.data}
    {view}
  />
{:else if view.type === "table"}
  <TablePreview bind:context={view.data} />
{:else if view.type === "erd"}
  <!-- ERD placeholder — replaced in Task 6 -->
  <div class="flex h-full items-center justify-center text-sm text-muted-foreground">
    ERD coming soon…
  </div>
{:else}
  <!-- Default Fallback -->
  <div class="flex-1 overflow-auto p-4 space-y-4">
    <pre class="p-4 bg-muted/30 rounded border border-border text-xs">View ID: {view.id}</pre>
  </div>
{/if}
```

### Verify
```bash
pnpm check
```
Expect 0 errors.

### Commit
```
feat(erd): install @xyflow/svelte + dagre, add erd ViewType
```

---

## Task 2: `erd-layout.ts` — dagre layout utility

### Goal
Pure function that takes an array of `MetaTable[]` (already filtered to the user's selection) and returns `{ nodes: Node[], edges: Edge[] }` ready for `@xyflow/svelte`. Edges are only drawn between tables that are both in the selection.

### Files

**Create: `src/lib/components/erd/erd-layout.ts`**

```typescript
import type { Node, Edge } from '@xyflow/svelte';
import dagre from '@dagrejs/dagre';
import type { MetaTable } from '$lib/commands/types';

export const TABLE_NODE_WIDTH = 220;
export const TABLE_HEADER_HEIGHT = 36;
export const COLUMN_ROW_HEIGHT = 24;

export interface ErdNodeData {
    table: MetaTable;
}

export function buildErdGraph(tables: MetaTable[]): { nodes: Node[], edges: Edge[] } {
    const g = new dagre.graphlib.Graph();
    g.setDefaultEdgeLabel(() => ({}));
    g.setGraph({ rankdir: 'LR', ranksep: 80, nodesep: 30 });

    for (const table of tables) {
        const id = `${table.schema}.${table.table_name}`;
        const height = TABLE_HEADER_HEIGHT + table.columns.length * COLUMN_ROW_HEIGHT;
        g.setNode(id, { width: TABLE_NODE_WIDTH, height });
    }

    // Collect FK edges between selected tables only
    const tableIds = new Set(tables.map(t => `${t.schema}.${t.table_name}`));
    const edgeDefs: Array<{
        id: string;
        source: string;
        sourceHandle: string;
        target: string;
        targetHandle: string;
    }> = [];

    for (const table of tables) {
        const sourceId = `${table.schema}.${table.table_name}`;
        for (const fk of table.foreign_keys) {
            const refSchema = fk.ref_schema ?? fk.schema;
            const targetId = `${refSchema}.${fk.ref_table}`;
            if (!tableIds.has(targetId)) continue; // skip edges to unselected tables
            g.setEdge(sourceId, targetId);
            edgeDefs.push({
                id: `${sourceId}.${fk.column_name}->${targetId}.${fk.ref_column}`,
                source: sourceId,
                sourceHandle: `${fk.column_name}-source`,
                target: targetId,
                targetHandle: `${fk.ref_column}-target`,
            });
        }
    }

    dagre.layout(g);

    const nodes: Node[] = tables.map(table => {
        const id = `${table.schema}.${table.table_name}`;
        const { x, y, width, height } = g.node(id);
        return {
            id,
            type: 'tableNode',
            position: { x: x - width / 2, y: y - height / 2 },
            data: { table } satisfies ErdNodeData,
        };
    });

    const edges: Edge[] = edgeDefs.map(e => ({
        id: e.id,
        source: e.source,
        sourceHandle: e.sourceHandle,
        target: e.target,
        targetHandle: e.targetHandle,
        type: 'smoothstep',
    }));

    return { nodes, edges };
}
```

### Notes
- `dagre.graphlib.Graph` is the entry point for the `@dagrejs/dagre` API.
- `rankdir: 'LR'` produces a left-to-right layout, which is best for wide table cards.
- `g.node(id)` returns `{ x, y, width, height }` where x/y are the center; we subtract half-dimensions to get the top-left corner that XYFlow expects.
- Edges pointing to tables not in the selection are silently skipped — this is intentional.

### Verify
```bash
pnpm check
```

### Commit
```
feat(erd): dagre layout utility
```

---

## Task 3: `ErdTableNode.svelte` — table card component

### Goal
Custom `@xyflow/svelte` node component. Receives `data: ErdNodeData`. Renders a card with a header (table name + schema badge) and one row per column showing PK badge, column name, nullable indicator, and raw type. Each column row registers both a `source` handle on the right and a `target` handle on the left so FK edges can anchor per-column.

### Files

**Create: `src/lib/components/erd/ErdTableNode.svelte`**

```svelte
<script lang="ts">
    import { Handle, Position } from '@xyflow/svelte';
    import type { ErdNodeData } from './erd-layout';
    import { TABLE_NODE_WIDTH, COLUMN_ROW_HEIGHT, TABLE_HEADER_HEIGHT } from './erd-layout';

    interface Props {
        data: ErdNodeData;
        selected?: boolean;
    }
    let { data, selected = false }: Props = $props();

    const table = $derived(data.table);
</script>

<div
    class="rounded-md border bg-card text-card-foreground shadow-sm overflow-hidden"
    class:ring-2={selected}
    class:ring-primary={selected}
    style="width: {TABLE_NODE_WIDTH}px;"
>
    <!-- Header -->
    <div
        class="flex items-center gap-1.5 border-b border-border bg-muted/50 px-2 font-semibold text-xs text-foreground"
        style="height: {TABLE_HEADER_HEIGHT}px;"
    >
        <span class="truncate">{table.table_name}</span>
        {#if table.schema !== 'public'}
            <span class="ml-auto text-muted-foreground shrink-0">{table.schema}</span>
        {/if}
    </div>

    <!-- Column rows -->
    {#each table.columns as col (col.column_name)}
        <div
            class="relative flex items-center gap-1 px-2 text-xs border-b border-border/50 last:border-0 hover:bg-muted/30"
            style="height: {COLUMN_ROW_HEIGHT}px;"
        >
            <!-- Source handle (right side) -->
            <Handle
                type="source"
                position={Position.Right}
                id="{col.column_name}-source"
                class="!w-2 !h-2 !bg-border !border-border"
            />
            <!-- Target handle (left side) -->
            <Handle
                type="target"
                position={Position.Left}
                id="{col.column_name}-target"
                class="!w-2 !h-2 !bg-border !border-border"
            />

            <!-- PK badge -->
            {#if col.is_primary_key}
                <span class="text-amber-500 shrink-0 font-bold text-[10px]">PK</span>
            {:else}
                <span class="w-[18px] shrink-0"></span>
            {/if}

            <!-- Column name -->
            <span class="truncate flex-1" class:text-muted-foreground={!col.is_primary_key}>
                {col.column_name}
            </span>

            <!-- Nullable dot -->
            {#if col.nullable}
                <span class="text-muted-foreground/50 shrink-0" title="nullable">○</span>
            {/if}

            <!-- Type -->
            <span class="text-muted-foreground/70 text-[10px] shrink-0 font-mono ml-1">
                {col.raw_type.replace(/\(.*\)/, '')}
            </span>
        </div>
    {/each}
</div>
```

### Notes
- `Handle` components from `@xyflow/svelte` must have unique `id` values within the node. Using `${col.column_name}-source` / `${col.column_name}-target` matches the handle IDs generated by `erd-layout.ts`.
- The `!` prefix in the handle classes uses Tailwind's `!important` modifier to override XYFlow's default handle styles.
- `col.raw_type.replace(/\(.*\)/, '')` strips length qualifiers like `varchar(255)` → `varchar` for display.
- The `class:ring-2={selected}` / `class:ring-primary={selected}` binding uses Svelte 5's class directive syntax to show selection state managed by XYFlow.

### Verify
```bash
pnpm check
```

### Commit
```
feat(erd): ErdTableNode card component
```

---

## Task 4: `ErdTableSelector.svelte` — the table selection dialog

### Goal
A `bits-ui` Dialog that presents a searchable, filterable list of all tables in the active schema. The user checks which tables to include, optionally auto-expands via "Add related tables" (FK graph traversal), then clicks "Open ERD (N tables)".

### Files

**Create: `src/lib/components/erd/ErdTableSelector.svelte`**

```svelte
<script lang="ts">
    import { Dialog } from 'bits-ui';
    import type { MetaTable } from '$lib/commands/types';
    import IconX from '@tabler/icons-svelte/icons/x';
    import IconSearch from '@tabler/icons-svelte/icons/search';
    import IconVectorTriangle from '@tabler/icons-svelte/icons/vector-triangle';

    interface Props {
        open: boolean;
        tables: MetaTable[];
        onConfirm: (selected: MetaTable[]) => void;
        onCancel: () => void;
    }
    let { open = $bindable(), tables, onConfirm, onCancel }: Props = $props();

    let search = $state('');
    let selected = $state<Set<string>>(new Set());

    // Key = "schema.table_name"
    function tableKey(t: MetaTable) { return `${t.schema}.${t.table_name}`; }

    // FK count per table (outgoing only — incoming would require a reverse-index pass)
    const fkCount = $derived.by(() => {
        const counts = new Map<string, number>();
        for (const t of tables) {
            const key = tableKey(t);
            counts.set(key, (counts.get(key) ?? 0) + t.foreign_keys.length);
        }
        return counts;
    });

    const filtered = $derived(
        tables.filter(t =>
            t.table_name.toLowerCase().includes(search.toLowerCase()) ||
            t.schema.toLowerCase().includes(search.toLowerCase())
        )
    );

    // Group by schema
    const grouped = $derived.by(() => {
        const map = new Map<string, MetaTable[]>();
        for (const t of filtered) {
            const list = map.get(t.schema) ?? [];
            list.push(t);
            map.set(t.schema, list);
        }
        return map;
    });

    const multiSchema = $derived(grouped.size > 1);

    const selectedTables = $derived(tables.filter(t => selected.has(tableKey(t))));

    function toggle(t: MetaTable) {
        const key = tableKey(t);
        const next = new Set(selected);
        if (next.has(key)) next.delete(key);
        else next.add(key);
        selected = next;
    }

    function selectAll() {
        selected = new Set(filtered.map(tableKey));
    }

    function clearAll() {
        selected = new Set();
    }

    function addRelated() {
        // Iteratively expand selection to include all FK-connected tables
        const allKeys = new Set(tables.map(tableKey));
        const next = new Set(selected);
        let changed = true;
        while (changed) {
            changed = false;
            for (const t of tables) {
                const key = tableKey(t);
                if (!next.has(key)) continue;
                // outgoing FKs from selected tables
                for (const fk of t.foreign_keys) {
                    const refSchema = fk.ref_schema ?? fk.schema;
                    const refKey = `${refSchema}.${fk.ref_table}`;
                    if (allKeys.has(refKey) && !next.has(refKey)) {
                        next.add(refKey);
                        changed = true;
                    }
                }
            }
            // incoming FKs — tables that reference any selected table
            for (const t of tables) {
                const key = tableKey(t);
                if (next.has(key)) continue;
                for (const fk of t.foreign_keys) {
                    const refSchema = fk.ref_schema ?? fk.schema;
                    const refKey = `${refSchema}.${fk.ref_table}`;
                    if (next.has(refKey) && !next.has(key)) {
                        next.add(key);
                        changed = true;
                    }
                }
            }
        }
        selected = next;
    }

    function confirm() {
        onConfirm(selectedTables);
        open = false;
    }
</script>

<Dialog.Root bind:open>
    <Dialog.Portal>
        <Dialog.Overlay class="fixed inset-0 z-50 bg-black/60" />
        <Dialog.Content
            class="fixed left-1/2 top-1/2 z-50 -translate-x-1/2 -translate-y-1/2 w-[520px] max-h-[80vh] flex flex-col rounded-lg border border-border bg-background shadow-xl"
        >
            <!-- Header -->
            <div class="flex items-center gap-2 border-b border-border px-4 py-3">
                <IconVectorTriangle class="h-4 w-4 text-muted-foreground shrink-0" />
                <h2 class="font-semibold text-sm">Select tables for ERD</h2>
                <button
                    class="ml-auto p-1 rounded hover:bg-accent text-muted-foreground"
                    onclick={() => { open = false; onCancel(); }}
                >
                    <IconX class="h-4 w-4" />
                </button>
            </div>

            <!-- Search -->
            <div class="px-3 pt-3 pb-2">
                <div class="flex items-center gap-2 rounded-md border border-border bg-muted/30 px-2 py-1.5">
                    <IconSearch class="h-3.5 w-3.5 text-muted-foreground shrink-0" />
                    <input
                        bind:value={search}
                        placeholder="Search tables…"
                        class="flex-1 bg-transparent text-sm outline-none placeholder:text-muted-foreground"
                    />
                </div>
            </div>

            <!-- Actions row -->
            <div class="flex items-center gap-2 px-3 pb-2 text-xs">
                <button class="text-muted-foreground hover:text-foreground underline-offset-2 hover:underline" onclick={selectAll}>
                    Select all ({filtered.length})
                </button>
                <span class="text-muted-foreground/50">·</span>
                <button class="text-muted-foreground hover:text-foreground underline-offset-2 hover:underline" onclick={clearAll}>
                    Clear
                </button>
                {#if selected.size > 0}
                    <span class="text-muted-foreground/50">·</span>
                    <button class="text-blue-400 hover:text-blue-300 underline-offset-2 hover:underline" onclick={addRelated}>
                        + Add related tables
                    </button>
                {/if}
                <span class="ml-auto text-muted-foreground">{selected.size} selected</span>
            </div>

            <!-- Table list -->
            <div class="flex-1 overflow-y-auto px-2 pb-2">
                {#each [...grouped.entries()] as [schema, schemaTables]}
                    {#if multiSchema}
                        <div class="px-2 py-1 text-xs font-semibold text-muted-foreground uppercase tracking-wide">
                            {schema}
                        </div>
                    {/if}
                    {#each schemaTables as t (tableKey(t))}
                        {@const key = tableKey(t)}
                        {@const isSelected = selected.has(key)}
                        {@const fks = fkCount.get(key) ?? 0}
                        <button
                            class="w-full flex items-center gap-2 rounded-md px-2 py-1.5 text-sm hover:bg-muted/50 text-left"
                            class:bg-muted={isSelected}
                            onclick={() => toggle(t)}
                        >
                            <!-- Checkbox -->
                            <span class="flex h-4 w-4 shrink-0 items-center justify-center rounded border border-border"
                                class:bg-primary={isSelected}
                                class:border-primary={isSelected}
                            >
                                {#if isSelected}
                                    <span class="text-primary-foreground text-[10px] font-bold">✓</span>
                                {/if}
                            </span>
                            <span class="flex-1 truncate">{t.table_name}</span>
                            {#if fks > 0}
                                <span class="text-xs text-muted-foreground bg-muted rounded px-1">{fks} FK</span>
                            {/if}
                            <span class="text-xs text-muted-foreground">{t.columns.length} cols</span>
                        </button>
                    {/each}
                {/each}
                {#if filtered.length === 0}
                    <div class="py-8 text-center text-sm text-muted-foreground">No tables match "{search}"</div>
                {/if}
            </div>

            <!-- Footer -->
            <div class="flex items-center justify-end gap-2 border-t border-border px-4 py-3">
                <button
                    class="rounded-md border border-border px-3 py-1.5 text-sm hover:bg-muted"
                    onclick={() => { open = false; onCancel(); }}
                >
                    Cancel
                </button>
                <button
                    class="rounded-md bg-primary px-3 py-1.5 text-sm text-primary-foreground hover:bg-primary/90 disabled:opacity-50"
                    disabled={selected.size === 0}
                    onclick={confirm}
                >
                    Open ERD ({selected.size} tables)
                </button>
            </div>
        </Dialog.Content>
    </Dialog.Portal>
</Dialog.Root>
```

### Notes
- `bits-ui` v2.16.5 is installed. Import `Dialog` from `'bits-ui'` directly — no pre-built wrapper exists in this codebase.
- `bind:open` uses Svelte 5 bindable props; the `$bindable()` rune in the component's prop default enables two-way binding from the parent.
- The `addRelated` function does a fixed-point graph traversal — it keeps looping until no new tables are added. Both outgoing and incoming FK relationships are traversed.
- `grouped` is a `Map<schema, MetaTable[]>`; iterating with `[...grouped.entries()]` preserves insertion order (alphabetical if schemas appear in order from the store).

### Verify
```bash
pnpm check
```

### Commit
```
feat(erd): ErdTableSelector dialog with search, checkboxes, add-related
```

---

## Task 5: `ErdView.svelte` — the main ERD canvas

### Goal
The main ERD canvas component. Receives the selected `MetaTable[]` and renders them using `@xyflow/svelte` with the custom `ErdTableNode` type. Includes `MiniMap`, `Controls`, `Background`, smooth-step edges, an "Auto layout" button, and localStorage-based position persistence.

### Files

**Create: `src/lib/components/erd/ErdView.svelte`**

```svelte
<script lang="ts">
    import { SvelteFlow, MiniMap, Controls, Background, type Node, type Edge } from '@xyflow/svelte';
    import '@xyflow/svelte/dist/style.css';
    import ErdTableNode from './ErdTableNode.svelte';
    import { buildErdGraph } from './erd-layout';
    import type { MetaTable } from '$lib/commands/types';
    import IconLayout from '@tabler/icons-svelte/icons/layout-board';

    interface Props {
        tables: MetaTable[];
        connectionId: string;
        schema: string;
    }
    let { tables, connectionId, schema }: Props = $props();

    const nodeTypes = { tableNode: ErdTableNode };

    const storageKey = $derived(`erd-positions:${connectionId}:${schema}`);

    function loadPositions(): Record<string, { x: number; y: number }> {
        try {
            return JSON.parse(localStorage.getItem(storageKey) ?? '{}');
        } catch {
            return {};
        }
    }

    function savePositions(nodes: Node[]) {
        const pos: Record<string, { x: number; y: number }> = {};
        for (const n of nodes) pos[n.id] = n.position;
        localStorage.setItem(storageKey, JSON.stringify(pos));
    }

    function buildErdGraphWithSavedPositions(tbls: MetaTable[]) {
        const { nodes: layoutNodes, edges: layoutEdges } = buildErdGraph(tbls);
        const saved = loadPositions();
        const merged = layoutNodes.map(n => ({
            ...n,
            position: saved[n.id] ?? n.position,
        }));
        return { nodes: merged, edges: layoutEdges };
    }

    let { nodes, edges } = $derived.by(() => buildErdGraphWithSavedPositions(tables));

    function autoLayout() {
        localStorage.removeItem(storageKey);
        const result = buildErdGraph(tables);
        nodes = result.nodes;
        edges = result.edges;
    }
</script>

<div class="relative h-full w-full bg-muted/20">
    <!-- Toolbar overlay -->
    <div class="absolute top-2 left-2 z-10 flex items-center gap-1">
        <button
            class="flex items-center gap-1.5 rounded-md border border-border bg-background px-2 py-1 text-xs hover:bg-accent shadow-sm"
            onclick={autoLayout}
            title="Reset layout"
        >
            <IconLayout class="h-3.5 w-3.5" />
            Auto layout
        </button>
    </div>

    <SvelteFlow
        bind:nodes
        bind:edges
        {nodeTypes}
        fitView
        minZoom={0.1}
        maxZoom={2}
        defaultEdgeOptions={{ type: 'smoothstep', animated: false }}
        onNodeDragStop={(_, __, currentNodes) => savePositions(currentNodes)}
    >
        <Background />
        <Controls />
        <MiniMap pannable zoomable />
    </SvelteFlow>
</div>
```

### Notes
- **CSS import is required:** `import '@xyflow/svelte/dist/style.css'` — without this, XYFlow renders without styles and handles/edges may be invisible.
- **`onNodeDragStop` callback signature:** The installed version uses `(event, node, nodes) => void`. Verify against the actual installed types by checking `node_modules/@xyflow/svelte/dist/types.d.ts` if the check fails.
- **`bind:nodes` / `bind:edges`:** XYFlow requires two-way binding to track internal position mutations (e.g., when the user pans).
- **`$derived.by` destructuring:** Svelte 5 allows `let { nodes, edges } = $derived.by(...)` — the returned object's properties become reactive. When `tables` prop changes (e.g., if the user opens a new ERD view), the layout is recomputed.
- **`autoLayout`:** Removes saved positions from localStorage and recomputes the dagre layout from scratch. Assigning `nodes` / `edges` directly (not via the derived) is intentional — it breaks the reactive chain and forces the new layout to persist until the next auto layout or tab reopen.
- `iconLayout` uses `@tabler/icons-svelte/icons/layout-board`. If that icon doesn't exist in the installed version, substitute `layout-grid` or `layout-2`.

### Verify
```bash
pnpm check
```

### Commit
```
feat(erd): ErdView canvas with xyflow, minimap, position persistence
```

---

## Task 6: Wire trigger + view into app shell

### Goal
Add the ERD icon button to `ExplorerToolbar.svelte`, mount the `ErdTableSelector` dialog there, and replace the placeholder ERD branch in `+page.svelte` with the real `ErdView` component.

### Files

**6a. `src/lib/components/explorer/ExplorerToolbar.svelte`**

The file already imports `schemaStore` (line 4) and `activeSession` is derived from `windowState` (line 26). Do not duplicate those imports.

Add to the `<script>` block (after existing imports):
```svelte
import IconVectorTriangle from '@tabler/icons-svelte/icons/vector-triangle';
import ErdTableSelector from '$lib/components/erd/ErdTableSelector.svelte';
import type { MetaTable } from '$lib/commands/types';
```

Add state and helpers after the existing `const activeSession = ...` line:
```svelte
let erdSelectorOpen = $state(false);

// Collect all tables from the selected database across all schemas
const allTables = $derived.by((): MetaTable[] => {
    const db = schemaStore.databases.find(d => d.name === schemaStore.selectedDatabase);
    if (!db) return [];
    return db.schemas.flatMap(s => s.tables);
});

function openErd(selected: MetaTable[]) {
    activeSession?.openView('erd', 'Entity Diagram', {
        tables: selected,
        connectionId: schemaStore.activeConnection?.id ?? '',
        schema: schemaStore.activeSchema ?? 'public',
    });
}
```

**Critical:** `MetaDatabase.name` is the correct field (NOT `database_name`). Verify this against `/src/lib/commands/types.ts` before writing.

In the template, add after the existing "New SQL Editor" button (before `</div>`):

```svelte
<div class="mx-1 h-4 w-px bg-border/50"></div>
<button
    class="p-1 flex items-center justify-center gap-1 hover:bg-accent rounded-sm text-muted-foreground hover:text-foreground transition-colors"
    title="Open ERD Diagram"
    onclick={() => erdSelectorOpen = true}
>
    <IconVectorTriangle class="size-4" />
</button>

<ErdTableSelector
    bind:open={erdSelectorOpen}
    tables={allTables}
    onConfirm={openErd}
    onCancel={() => {}}
/>
```

The `ErdTableSelector` can be placed anywhere in the template outside the toolbar `<div>` (it uses a Portal), but placing it at the bottom of the component template is cleanest.

**6b. `src/routes/+page.svelte`**

Add the import at the top of the `<script>` block (with other component imports):
```svelte
import ErdView from '$lib/components/erd/ErdView.svelte';
```

Replace the ERD placeholder branch added in Task 1:
```svelte
{:else if view.type === "erd"}
    <!-- ERD placeholder — replaced in Task 6 -->
    <div class="flex h-full items-center justify-center text-sm text-muted-foreground">
        ERD coming soon…
    </div>
```

With:
```svelte
{:else if view.type === "erd"}
    <ErdView
        tables={view.data?.tables ?? []}
        connectionId={view.data?.connectionId ?? ''}
        schema={view.data?.schema ?? 'public'}
    />
```

### Verify
```bash
pnpm check
```
Expect 0 errors.

### Commit
```
feat(erd): wire ERD trigger button and view into app shell
```

---

## Task 7: Final integration check

### Goal
Confirm zero TypeScript errors and zero Rust compilation errors. Fix any issues introduced by the feature.

### Steps

**7a. Frontend type check:**
```bash
pnpm check
```
Expected: 0 errors, 0 warnings (or only pre-existing warnings).

**7b. Rust check (no Rust changes in this feature, but verify no regressions):**
```bash
cd src-tauri && cargo check 2>&1 | grep "^error"
```
Expected: no output (no errors).

**7c. Common issues and fixes:**

| Symptom | Likely cause | Fix |
|---|---|---|
| `onNodeDragStop` type error | Callback signature mismatch | Check `node_modules/@xyflow/svelte/dist/types.d.ts` for the actual signature; adjust lambda parameters |
| `Dialog.Root` / `Dialog.Portal` not found | bits-ui import path changed | Try `import { Dialog } from 'bits-ui'` → check `node_modules/bits-ui/dist/index.d.ts` for export name |
| `dagre.graphlib.Graph` is not a constructor | Incorrect dagre import | Try `import * as dagre from '@dagrejs/dagre'` or check the package's default export |
| `layout-board` icon not found | Icon name doesn't exist in installed version | Substitute `layout-grid` or `layout-2` from `@tabler/icons-svelte` |
| `$derived.by` destructuring issues | Svelte 5 beta incompatibility | Convert to `const graph = $derived.by(...); let nodes = $state(graph.nodes); let edges = $state(graph.edges);` |

**7d. Commit any fixes:**
```
fix(erd): resolve type errors from integration
```

---

## Architecture Summary

```
ExplorerToolbar
  └─ onclick "ERD" button
       └─ erdSelectorOpen = true
            └─ ErdTableSelector (bits-ui Dialog)
                 └─ onConfirm(selectedTables)
                      └─ activeSession.openView("erd", "Entity Diagram", { tables, connectionId, schema })
                           └─ +page.svelte view switch
                                └─ ErdView
                                     ├─ buildErdGraph(tables) → dagre layout → { nodes, edges }
                                     ├─ loadPositions() from localStorage
                                     └─ SvelteFlow canvas
                                          ├─ ErdTableNode (custom node, per-column handles)
                                          ├─ smoothstep edges (per-column anchors)
                                          ├─ MiniMap + Controls + Background
                                          └─ onNodeDragStop → savePositions() to localStorage
```

## File Index

| File | Status | Purpose |
|---|---|---|
| `src/lib/commands/types.ts` | Modified | Add `ref_schema?: string` to `MetaForeignKey` |
| `src/lib/stores/session.svelte.ts` | Modified | Add `"erd"` to `ViewType` union |
| `src/routes/+page.svelte` | Modified | Add `{:else if view.type === "erd"}` branch with `<ErdView>` |
| `src/lib/components/explorer/ExplorerToolbar.svelte` | Modified | Add ERD button + `ErdTableSelector` mount |
| `src/lib/components/erd/erd-layout.ts` | Created | dagre layout pure function |
| `src/lib/components/erd/ErdTableNode.svelte` | Created | Custom XYFlow node card |
| `src/lib/components/erd/ErdTableSelector.svelte` | Created | Table picker dialog |
| `src/lib/components/erd/ErdView.svelte` | Created | Main ERD canvas |
