# ERD Loading Feedback Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Show a small floating progress chip while the ERD fetches and lays out tables, and gate large selections (> 50 tables) with an inline warning in the selector.

**Architecture:** Three changes: (1) confirmation gate in `ErdTableSelector`, (2) `openErd` in `ExplorerToolbar` simplified to pass lightweight stubs, (3) `ErdView` owns all fetching in batched chunks with a reactive progress chip.

**Tech Stack:** Svelte 5 runes, @xyflow/svelte, Tauri `invoke`, Tailwind CSS

---

## File Map

| File | Change |
|---|---|
| `src/lib/components/erd/ErdTableSelector.svelte` | Add `LARGE_THRESHOLD = 50` constant; replace footer with conditional warning banner + "Open anyway" button |
| `src/lib/components/explorer/ExplorerToolbar.svelte` | Remove `Promise.all` enrichment from `openErd`; pass stubs directly to `openView` |
| `src/lib/components/erd/ErdView.svelte` | Add batched fetch loop, loading state, `enrichedTables` cache, progress chip HTML |

---

## Task 1: Confirmation gate in ErdTableSelector

**Files:**
- Modify: `src/lib/components/erd/ErdTableSelector.svelte`

**Context:** The footer currently has a single "Open ERD (N tables)" button. When > 50 tables are selected, replace the button label with "Open anyway" and show an inline amber warning banner above the button row.

- [ ] **Step 1: Add LARGE_THRESHOLD constant and replace the footer block**

In `src/lib/components/erd/ErdTableSelector.svelte`, add this constant after the `confirm` function (before `</script>`):

```svelte
    const LARGE_THRESHOLD = 50;
```

Then replace the entire `<!-- Footer -->` block (the `<div class="flex items-center justify-end gap-2 border-t ...">` block at the bottom of the template) with:

```svelte
            <!-- Footer -->
            <div class="flex flex-col gap-2 border-t border-border px-4 py-3">
                {#if selected.size > LARGE_THRESHOLD}
                    <div class="flex items-center gap-2 rounded-md border border-amber-500/30 bg-amber-500/10 px-3 py-2 text-xs text-amber-600 dark:text-amber-400">
                        <span>⚠</span>
                        <span>{selected.size} tables selected — large schemas take longer to load.</span>
                    </div>
                {/if}
                <div class="flex items-center justify-end gap-2">
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
                        {selected.size > LARGE_THRESHOLD ? 'Open anyway' : 'Open ERD'} ({selected.size} tables)
                    </button>
                </div>
            </div>
```

- [ ] **Step 2: Type-check**

```bash
pnpm check
```

Expected: `0 ERRORS`

- [ ] **Step 3: Commit**

```bash
git add src/lib/components/erd/ErdTableSelector.svelte
git commit -m "feat(erd): add confirmation gate for large table selections (>50)"
```

---

## Task 2: Simplify openErd in ExplorerToolbar

**Files:**
- Modify: `src/lib/components/explorer/ExplorerToolbar.svelte`

**Context:** `openErd` currently fires `Promise.all(N × invoke)` before opening the canvas — blocking the UI with no feedback. ErdView will now own fetching. `openErd` just needs to pass the lightweight stubs (already available from the schema store) and call `openView`.

- [ ] **Step 1: Replace the openErd function body**

Find this block in `src/lib/components/explorer/ExplorerToolbar.svelte`:

```typescript
    async function openErd(selected: MetaTable[]) {
        const connectionId = schemaStore.activeConnection?.id ?? '';
        // Fetch full column + FK details for each selected table (columns are lazy-loaded)
        const enriched = await Promise.all(selected.map(async (t) => {
            try {
                const details = await invoke<any>("get_schema_table_details", {
                    connectionId,
                    database: t.database,
                    schema: t.schema,
                    tableName: t.table_name,
                });
                return { ...t, columns: details.columns ?? [], foreign_keys: details.foreign_keys ?? [] };
            } catch {
                return t;
            }
        }));
        activeSession?.openView('erd', 'Entity Diagram', {
            tables: enriched,
            connectionId,
            schema: schemaStore.activeSchema ?? 'public',
        });
    }
```

Replace with:

```typescript
    function openErd(selected: MetaTable[]) {
        const connectionId = schemaStore.activeConnection?.id ?? '';
        activeSession?.openView('erd', 'Entity Diagram', {
            tables: selected,
            connectionId,
            schema: schemaStore.activeSchema ?? 'public',
        });
    }
```

- [ ] **Step 2: Remove unused invoke import if no longer used**

Check whether `invoke` is used anywhere else in `ExplorerToolbar.svelte`:

```bash
grep -n "invoke" src/lib/components/explorer/ExplorerToolbar.svelte
```

If the only `invoke` reference was in `openErd`, remove the import line:
```typescript
import { invoke } from "@tauri-apps/api/core";
```

If `invoke` appears elsewhere in the file, leave the import as-is.

- [ ] **Step 3: Type-check**

```bash
pnpm check
```

Expected: `0 ERRORS`

- [ ] **Step 4: Commit**

```bash
git add src/lib/components/explorer/ExplorerToolbar.svelte
git commit -m "refactor(erd): move table fetching from toolbar into ErdView"
```

---

## Task 3: Batched fetching + progress chip in ErdView

**Files:**
- Modify: `src/lib/components/erd/ErdView.svelte`

**Context:** ErdView currently receives pre-enriched `tables`. After Task 2 it receives lightweight stubs. This task adds: batched IPC fetching (20 at a time), reactive loading state (`fetchPhase`, `fetchedCount`, `failedCount`), caches enriched tables in `enrichedTables` so `autoLayout` can re-run without re-fetching, and renders a floating progress chip bottom-left of the canvas.

- [ ] **Step 1: Replace the entire `<script>` block in ErdView.svelte**

Replace the full contents of `src/lib/components/erd/ErdView.svelte` with:

```svelte
<script lang="ts">
    import { SvelteFlow, MiniMap, Controls, Background, getNodesBounds, getViewportForBounds, type Node, type Edge } from '@xyflow/svelte';
    import '@xyflow/svelte/dist/style.css';
    import ErdTableNode from './ErdTableNode.svelte';
    import ErdSelfLoopEdge from './ErdSelfLoopEdge.svelte';
    import { buildErdGraph } from './erd-layout';
    import type { MetaTable } from '$lib/commands/types';
    import IconLayout from '@tabler/icons-svelte/icons/layout-board';
    import IconDownload from '@tabler/icons-svelte/icons/download';
    import { toSvg } from 'html-to-image';
    import { save } from '@tauri-apps/plugin-dialog';
    import { invoke } from '@tauri-apps/api/core';
    import { onMount } from 'svelte';

    interface Props {
        tables: MetaTable[]; // lightweight stubs — columns/FKs fetched here
        connectionId: string;
        schema: string;
    }
    let { tables, connectionId, schema }: Props = $props();

    const nodeTypes = { tableNode: ErdTableNode };
    const edgeTypes = { selfLoop: ErdSelfLoopEdge };

    const storageKey = $derived(`erd-positions:${connectionId}:${schema}`);

    function loadPositions(): Record<string, { x: number; y: number }> {
        try {
            return JSON.parse(localStorage.getItem(storageKey) ?? '{}');
        } catch {
            return {};
        }
    }

    function savePositions(currentNodes: Node[]) {
        const pos: Record<string, { x: number; y: number }> = {};
        for (const n of currentNodes) pos[n.id] = n.position;
        localStorage.setItem(storageKey, JSON.stringify(pos));
    }

    // ── Loading state ──────────────────────────────────────────────────────────
    type LoadPhase = 'fetching' | 'layout' | 'done' | 'error';
    let loadPhase = $state<LoadPhase>('fetching');
    let fetchedCount = $state(0);
    let totalCount = $state(0);
    let failedCount = $state(0);
    let chipVisible = $state(false);

    // Enriched tables are cached so autoLayout can re-run without re-fetching.
    let enrichedTables = $state<MetaTable[]>([]);

    const BATCH_SIZE = 20;

    async function fetchEnrichedTables(stubs: MetaTable[]): Promise<MetaTable[]> {
        const results: MetaTable[] = [];
        totalCount = stubs.length;
        fetchedCount = 0;
        failedCount = 0;

        for (let i = 0; i < stubs.length; i += BATCH_SIZE) {
            const chunk = stubs.slice(i, i + BATCH_SIZE);
            const settled = await Promise.allSettled(
                chunk.map(t =>
                    invoke<any>('get_schema_table_details', {
                        connectionId,
                        database: t.database,
                        schema: t.schema,
                        tableName: t.table_name,
                    }).then((details: any) => ({
                        ...t,
                        columns: details.columns ?? [],
                        foreign_keys: details.foreign_keys ?? [],
                    }))
                )
            );
            for (let j = 0; j < settled.length; j++) {
                const result = settled[j];
                if (result.status === 'fulfilled') {
                    results.push(result.value);
                } else {
                    results.push(chunk[j]); // fallback to stub on error
                    failedCount++;
                }
            }
            fetchedCount += chunk.length;
        }
        return results;
    }

    async function buildWithSavedPositions(tbls: MetaTable[]) {
        const { nodes: layoutNodes, edges: layoutEdges } = await buildErdGraph(tbls);
        const saved = loadPositions();
        const merged = layoutNodes.map(n => ({
            ...n,
            position: saved[n.id] ?? n.position,
        }));
        return { nodes: merged, edges: layoutEdges };
    }

    let nodes = $state<Node[]>([]);
    let edges = $state<Edge[]>([]);

    $effect(() => {
        if (tables.length === 0) return;
        chipVisible = true;
        loadPhase = 'fetching';

        fetchEnrichedTables(tables)
            .then(enriched => {
                enrichedTables = enriched;
                loadPhase = 'layout';
                return buildWithSavedPositions(enriched);
            })
            .then(result => {
                nodes = result.nodes;
                edges = result.edges;
                loadPhase = 'done';
                setTimeout(() => { chipVisible = false; }, 1500);
            })
            .catch(() => {
                loadPhase = 'error';
            });
    });

    function autoLayout() {
        if (enrichedTables.length === 0) return;
        buildErdGraph(enrichedTables).then(result => {
            localStorage.removeItem(storageKey);
            nodes = result.nodes;
            edges = result.edges;
        });
    }

    // ── Background colour ──────────────────────────────────────────────────────
    let bgColor = $state('');

    onMount(() => {
        bgColor = getComputedStyle(document.documentElement)
            .getPropertyValue('--theme-border-subtle')
            .trim();
    });

    // ── SVG export ─────────────────────────────────────────────────────────────
    const IMAGE_WIDTH = 1920;
    const IMAGE_HEIGHT = 1080;
    let downloading = $state(false);

    async function downloadImage() {
        const viewport = document.querySelector<HTMLElement>('.svelte-flow__viewport');
        if (!viewport) return;

        const nodesBounds = getNodesBounds(nodes);
        const transform = getViewportForBounds(nodesBounds, IMAGE_WIDTH, IMAGE_HEIGHT, 0.1, 2, 0.1);

        downloading = true;
        try {
            const defaultName = `erd-${schema}-${new Date().toISOString().slice(0, 10)}.svg`;
            const path = await save({
                defaultPath: defaultName,
                filters: [{ name: 'SVG Image', extensions: ['svg'] }],
            });
            if (!path) return;

            const dataUrl = await toSvg(viewport, {
                width: IMAGE_WIDTH,
                height: IMAGE_HEIGHT,
                style: {
                    width: `${IMAGE_WIDTH}px`,
                    height: `${IMAGE_HEIGHT}px`,
                    transform: `translate(${transform.x}px, ${transform.y}px) scale(${transform.zoom})`,
                },
            });
            const svg = decodeURIComponent(
                dataUrl.replace(/^data:image\/svg\+xml;charset=utf-8,/, '')
            );
            await invoke('save_svg_file', { path, svg });
        } finally {
            downloading = false;
        }
    }
</script>

<div class="relative h-full w-full bg-[--theme-bg-primary]">
    <!-- Toolbar overlay -->
    <div class="absolute top-2 left-2 z-10 flex items-center gap-1">
        <button
            class="flex items-center gap-1.5 rounded-md border border-[--theme-border-default] bg-[--theme-bg-secondary] px-2 py-1 text-xs text-[--theme-fg-primary] hover:bg-[--theme-bg-hover] shadow-sm"
            onclick={autoLayout}
            title="Reset layout"
        >
            <IconLayout class="h-3.5 w-3.5" />
            Auto layout
        </button>
        <button
            class="flex items-center gap-1.5 rounded-md border border-[--theme-border-default] bg-[--theme-bg-secondary] px-2 py-1 text-xs text-[--theme-fg-primary] hover:bg-[--theme-bg-hover] shadow-sm disabled:opacity-50"
            onclick={downloadImage}
            disabled={downloading}
            title="Export as SVG"
        >
            <IconDownload class="h-3.5 w-3.5" />
            {downloading ? 'Saving…' : 'Export SVG'}
        </button>
    </div>

    <SvelteFlow
        bind:nodes
        bind:edges
        {nodeTypes}
        {edgeTypes}
        fitView
        minZoom={0.1}
        maxZoom={2}
        defaultEdgeOptions={{ type: 'smoothstep', animated: false, style: 'stroke-width: 2.5px;' }}
        onnodedragstop={({ nodes: currentNodes }) => savePositions(currentNodes)}
    >
        <Background patternColor={bgColor} />
        <Controls />
        <MiniMap pannable zoomable />
    </SvelteFlow>

    <!-- Progress chip: bottom-left, fades out on done -->
    {#if chipVisible}
        <div
            class="absolute bottom-4 left-4 z-20 flex items-center gap-2 rounded-md border border-[--theme-border-default] bg-[--theme-bg-secondary] px-3 py-2 text-xs text-[--theme-fg-primary] shadow-md transition-opacity duration-700"
            class:opacity-0={loadPhase === 'done'}
        >
            {#if loadPhase === 'fetching'}
                <span class="inline-block animate-spin">⟳</span>
                <span>Fetching tables {fetchedCount} / {totalCount}</span>
            {:else if loadPhase === 'layout'}
                <span class="inline-block animate-spin">⟳</span>
                <span>Computing layout…</span>
            {:else if loadPhase === 'done'}
                <span>✓</span>
                {#if failedCount > 0}
                    <span>{totalCount - failedCount} tables loaded, {failedCount} failed</span>
                {:else}
                    <span>Done</span>
                {/if}
            {:else if loadPhase === 'error'}
                <span class="text-red-400">✗ Layout failed</span>
            {/if}
        </div>
    {/if}
</div>
```

- [ ] **Step 2: Type-check**

```bash
pnpm check
```

Expected: `0 ERRORS`

- [ ] **Step 3: Commit**

```bash
git add src/lib/components/erd/ErdView.svelte
git commit -m "feat(erd): batched table fetching with progress chip

ErdView now owns enrichment: fetches table details in batches of 20,
tracks fetchedCount/totalCount/failedCount, and shows a floating chip
bottom-left (Fetching N/total → Computing layout… → Done ✓).
Enriched tables cached in enrichedTables so autoLayout never re-fetches."
```
