# ERD Diagram Improvements Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Polish the ERD diagram with themed backgrounds, auto-sizing nodes, FK/PK-only handles with badges, ELK layout, arrow markers, self-loop side curves, and SVG export.

**Architecture:** Replace dagre with elkjs for layout (async, handles disconnected subgraphs well), rewrite ErdTableNode with NodeResizer and conditional handles, add a custom ErdSelfLoopEdge component for self-referencing FKs, and switch export from PNG to SVG via html-to-image `toSvg`.

**Tech Stack:** @xyflow/svelte (NodeResizer, MarkerType, BaseEdge), elkjs (layout), html-to-image (toSvg), @tauri-apps/plugin-dialog (save), Rust fs::write (svg file write)

---

## File Structure

| File | Role |
|------|------|
| `src/lib/components/erd/erd-layout.ts` | Layout engine (ELK), auto-width computation, edge building with markers and self-loop detection |
| `src/lib/components/erd/ErdTableNode.svelte` | Node card: schema.table header, opaque bg, FK/PK-only handles, badges+icons, NodeResizer |
| `src/lib/components/erd/ErdSelfLoopEdge.svelte` | New: custom SVG edge for FK self-references, right-side cubic bezier loop |
| `src/lib/components/erd/ErdView.svelte` | Async graph build, selfLoop edge type registration, themed Background, toSvg export |
| `src-tauri/src/commands/util_commands.rs` | Replace save_png_file with save_svg_file (plain text write, no base64) |
| `src-tauri/src/plugins/core.rs` | Update aggregate_plugin_commands! macro |
| `src-tauri/Cargo.toml` | Remove base64 dependency |

---

## Task 1: Swap layout dependencies

**Files:**
- Modify: `package.json` (via pnpm)

- [ ] **Step 1: Remove dagre, add elkjs**

```bash
cd /path/to/tables
pnpm remove @dagrejs/dagre -w
pnpm add elkjs -w
```

Expected output: `Done in Xs`

- [ ] **Step 2: Verify elkjs types are available**

```bash
ls node_modules/elkjs/lib/
```

Expected: see `elk.bundled.js` in the listing.

- [ ] **Step 3: Verify TypeScript can find elkjs types**

```bash
cat node_modules/elkjs/package.json | grep '"types"'
```

Expected: a `"types"` field pointing to a `.d.ts` file (elkjs ships its own types).

- [ ] **Step 4: Commit**

```bash
git add package.json pnpm-lock.yaml
git commit -m "chore(erd): swap dagre for elkjs"
```

---

## Task 2: Rewrite erd-layout.ts with ELK

**Files:**
- Modify: `src/lib/components/erd/erd-layout.ts`

- [ ] **Step 1: Replace the entire file**

```typescript
import type { Node, Edge } from '@xyflow/svelte';
import { MarkerType } from '@xyflow/svelte';
import ELK from 'elkjs/lib/elk.bundled.js';
import type { MetaTable } from '$lib/commands/types';

export const TABLE_HEADER_HEIGHT = 36;
export const COLUMN_ROW_HEIGHT = 24;

export interface ErdNodeData {
    table: MetaTable;
}

function computeNodeWidth(table: MetaTable): number {
    const headerLen = `${table.schema}.${table.table_name}`.length;
    const maxColLen = table.columns.reduce((max, col) => {
        const len = col.column_name.length + col.raw_type.replace(/\(.*\)/, '').length + 10;
        return Math.max(max, len);
    }, 0);
    const raw = Math.max(headerLen, maxColLen) * 7 + 80;
    return Math.max(200, Math.min(500, raw));
}

export async function buildErdGraph(tables: MetaTable[]): Promise<{ nodes: Node[]; edges: Edge[] }> {
    const elk = new ELK();

    const tableIds = new Set(tables.map(t => `${t.schema}.${t.table_name}`));

    interface EdgeDef {
        id: string;
        source: string;
        sourceHandle: string;
        target: string;
        targetHandle: string;
        isSelfLoop: boolean;
    }

    const edgeDefs: EdgeDef[] = [];

    for (const table of tables) {
        const sourceId = `${table.schema}.${table.table_name}`;
        for (const fk of table.foreign_keys) {
            const refSchema = fk.ref_schema ?? fk.schema;
            const targetId = `${refSchema}.${fk.ref_table}`;
            if (!tableIds.has(targetId)) continue;

            const isSelfLoop = sourceId === targetId;
            edgeDefs.push({
                id: `${sourceId}.${fk.column_name}->${targetId}.${fk.ref_column}`,
                source: sourceId,
                sourceHandle: `${fk.column_name}-source`,
                // Self-loop: both handles on the right side so the loop never crosses the node
                targetHandle: isSelfLoop ? `${fk.ref_column}-source` : `${fk.ref_column}-target`,
                target: targetId,
                isSelfLoop,
            });
        }
    }

    const elkGraph = {
        id: 'root',
        layoutOptions: {
            'elk.algorithm': 'layered',
            'elk.direction': 'RIGHT',
            'elk.spacing.nodeNode': '80',
            'elk.layered.spacing.nodeNodeBetweenLayers': '150',
            'elk.spacing.edgeNode': '40',
            'elk.layered.nodePlacement.strategy': 'BRANDES_KOEPF',
        },
        children: tables.map(table => ({
            id: `${table.schema}.${table.table_name}`,
            width: computeNodeWidth(table),
            height: TABLE_HEADER_HEIGHT + table.columns.length * COLUMN_ROW_HEIGHT,
        })),
        // Self-loop edges are excluded from ELK — their position is derived from node position at render time
        edges: edgeDefs
            .filter(e => !e.isSelfLoop)
            .map(e => ({ id: e.id, sources: [e.source], targets: [e.target] })),
    };

    const layout = await elk.layout(elkGraph);
    const elkNodes = new Map((layout.children ?? []).map(n => [n.id, n]));

    const nodes: Node[] = tables.map(table => {
        const id = `${table.schema}.${table.table_name}`;
        const elkNode = elkNodes.get(id);
        const width = computeNodeWidth(table);
        const height = TABLE_HEADER_HEIGHT + table.columns.length * COLUMN_ROW_HEIGHT;
        return {
            id,
            type: 'tableNode',
            position: { x: elkNode?.x ?? 0, y: elkNode?.y ?? 0 },
            width,
            height,
            data: { table } satisfies ErdNodeData,
        };
    });

    const edges: Edge[] = edgeDefs.map(e => ({
        id: e.id,
        source: e.source,
        sourceHandle: e.sourceHandle,
        target: e.target,
        targetHandle: e.targetHandle,
        type: e.isSelfLoop ? 'selfLoop' : 'smoothstep',
        markerEnd: { type: MarkerType.ArrowClosed },
    }));

    return { nodes, edges };
}
```

- [ ] **Step 2: Run type check**

```bash
pnpm check
```

Expected: `0 ERRORS`. If elkjs types aren't found, add `// @ts-ignore` above the `import ELK` line.

- [ ] **Step 3: Commit**

```bash
git add src/lib/components/erd/erd-layout.ts
git commit -m "feat(erd): replace dagre with ELK layout engine, add auto-width and ArrowClosed markers"
```

---

## Task 3: Create ErdSelfLoopEdge.svelte

**Files:**
- Create: `src/lib/components/erd/ErdSelfLoopEdge.svelte`

This component renders a right-side cubic bezier loop for FK edges where source and target are the same table. It uses `BaseEdge` from @xyflow/svelte which handles the interactive hit area and marker rendering.

- [ ] **Step 1: Create the file**

```svelte
<script lang="ts">
    import { BaseEdge } from '@xyflow/svelte';

    interface Props {
        id: string;
        sourceX: number;
        sourceY: number;
        targetX: number;
        targetY: number;
        markerEnd?: string;
        style?: string;
    }
    let {
        id,
        sourceX,
        sourceY,
        targetX,
        targetY,
        markerEnd = '',
        style = '',
    }: Props = $props();

    // Both source and target are on the right side of the same node.
    // Bulge 70px to the right so the loop is clearly visible outside the node.
    const offset = 70;
    const path = `M ${sourceX} ${sourceY} C ${sourceX + offset} ${sourceY}, ${targetX + offset} ${targetY}, ${targetX} ${targetY}`;
</script>

<BaseEdge {id} {path} {markerEnd} {style} />
```

- [ ] **Step 2: Run type check**

```bash
pnpm check
```

Expected: `0 ERRORS`.

- [ ] **Step 3: Commit**

```bash
git add src/lib/components/erd/ErdSelfLoopEdge.svelte
git commit -m "feat(erd): add ErdSelfLoopEdge custom component for self-referencing FK edges"
```

---

## Task 4: Rewrite ErdTableNode.svelte

**Files:**
- Modify: `src/lib/components/erd/ErdTableNode.svelte`

Key changes:
- Header shows `schema.table_name` as one string
- Container uses `w-full` (width managed by @xyflow from node `width` field) and `bg-[--theme-bg-secondary]`
- Handles only on PK and FK columns (both source on right + target on left per column)
- PK: amber `IconKey` + "PK" badge; FK: blue `IconArrowUpRight` + "FK" badge
- `NodeResizer` visible when node is selected

- [ ] **Step 1: Replace the entire file**

```svelte
<script lang="ts">
    import { Handle, Position, NodeResizer } from '@xyflow/svelte';
    import type { ErdNodeData } from './erd-layout';
    import { COLUMN_ROW_HEIGHT, TABLE_HEADER_HEIGHT } from './erd-layout';
    import IconKey from '@tabler/icons-svelte/icons/key';
    import IconArrowUpRight from '@tabler/icons-svelte/icons/arrow-up-right';

    interface Props {
        data: ErdNodeData;
        selected?: boolean;
    }
    let { data, selected = false }: Props = $props();

    const table = $derived(data.table);
    const fkColumns = $derived(new Set(table.foreign_keys.map(fk => fk.column_name)));
</script>

<NodeResizer minWidth={200} maxWidth={500} isVisible={selected} />

<div
    class="rounded-md border border-[--theme-border-default] bg-[--theme-bg-secondary] text-[--theme-fg-primary] shadow-sm overflow-hidden w-full"
    class:ring-2={selected}
    class:ring-primary={selected}
>
    <!-- Header: schema.table_name -->
    <div
        class="flex items-center border-b border-[--theme-border-default] bg-[--theme-bg-tertiary] px-2 font-semibold text-xs"
        style="height: {TABLE_HEADER_HEIGHT}px;"
    >
        <span class="truncate">
            <span class="text-[--theme-fg-secondary]">{table.schema}.</span>{table.table_name}
        </span>
    </div>

    <!-- Column rows -->
    {#each table.columns as col (col.column_name)}
        {@const isFk = fkColumns.has(col.column_name)}
        {@const isConnectable = col.is_primary_key || isFk}
        <div
            class="relative flex items-center gap-1 px-2 text-xs border-b border-[--theme-border-default]/50 last:border-0 hover:bg-[--theme-bg-hover]"
            style="height: {COLUMN_ROW_HEIGHT}px;"
        >
            {#if isConnectable}
                <!-- Source handle (right side) -->
                <Handle
                    type="source"
                    position={Position.Right}
                    id="{col.column_name}-source"
                    class="!w-2 !h-2 !bg-[--theme-border-default] !border-[--theme-border-default]"
                />
                <!-- Target handle (left side) -->
                <Handle
                    type="target"
                    position={Position.Left}
                    id="{col.column_name}-target"
                    class="!w-2 !h-2 !bg-[--theme-border-default] !border-[--theme-border-default]"
                />
            {/if}

            <!-- PK badge -->
            {#if col.is_primary_key}
                <IconKey class="h-3 w-3 text-amber-500 shrink-0" />
                <span class="text-amber-500 shrink-0 font-bold text-[10px]">PK</span>
            {/if}

            <!-- FK badge -->
            {#if isFk}
                <IconArrowUpRight class="h-3 w-3 text-blue-400 shrink-0" />
                <span class="text-blue-400 shrink-0 font-bold text-[10px]">FK</span>
            {/if}

            <!-- Spacer when no badges -->
            {#if !col.is_primary_key && !isFk}
                <span class="w-5 shrink-0"></span>
            {/if}

            <!-- Column name -->
            <span
                class="truncate flex-1"
                class:text-[--theme-fg-secondary]={!col.is_primary_key}
            >
                {col.column_name}
            </span>

            <!-- Nullable indicator -->
            {#if col.nullable}
                <span class="text-[--theme-fg-tertiary] shrink-0 text-[10px]" title="nullable">○</span>
            {/if}

            <!-- Type -->
            <span class="text-[--theme-fg-tertiary] text-[10px] shrink-0 font-mono ml-1">
                {col.raw_type.replace(/\(.*\)/, '')}
            </span>
        </div>
    {/each}
</div>
```

- [ ] **Step 2: Run type check**

```bash
pnpm check
```

Expected: `0 ERRORS`.

- [ ] **Step 3: Commit**

```bash
git add src/lib/components/erd/ErdTableNode.svelte
git commit -m "feat(erd): redesign node — schema.table header, opaque bg, FK/PK-only handles, badges, NodeResizer"
```

---

## Task 5: Update ErdView.svelte

**Files:**
- Modify: `src/lib/components/erd/ErdView.svelte`

Changes:
- Register `selfLoop` edge type
- `buildErdGraphWithSavedPositions` and `autoLayout` become async (call `.then()`)
- Pass live `--theme-border-subtle` CSS variable to `<Background color>`
- Container background uses `bg-[--theme-bg-primary]` instead of `bg-muted/20`
- Switch export: `toPng` → `toSvg`, PNG save → SVG save, `save_png_file` → `save_svg_file`
- Toolbar buttons use explicit theme token classes

- [ ] **Step 1: Replace the entire file**

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

    interface Props {
        tables: MetaTable[];
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

    async function buildErdGraphWithSavedPositions(tbls: MetaTable[]) {
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
        buildErdGraphWithSavedPositions(tables).then(result => {
            nodes = result.nodes;
            edges = result.edges;
        });
    });

    function autoLayout() {
        localStorage.removeItem(storageKey);
        buildErdGraph(tables).then(result => {
            nodes = result.nodes;
            edges = result.edges;
        });
    }

    const bgColor = getComputedStyle(document.documentElement)
        .getPropertyValue('--theme-border-subtle')
        .trim();

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
            // toSvg returns a data URL: "data:image/svg+xml;charset=utf-8,<url-encoded-svg>"
            // Decode to raw SVG string before writing
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
        defaultEdgeOptions={{ type: 'smoothstep', animated: false }}
        onnodedragstop={({ nodes: currentNodes }) => savePositions(currentNodes)}
    >
        <Background color={bgColor} />
        <Controls />
        <MiniMap pannable zoomable />
    </SvelteFlow>
</div>
```

- [ ] **Step 2: Run type check**

```bash
pnpm check
```

Expected: `0 ERRORS`.

- [ ] **Step 3: Commit**

```bash
git add src/lib/components/erd/ErdView.svelte
git commit -m "feat(erd): async ELK build, selfLoop edge type, themed background, SVG export"
```

---

## Task 6: Rust — replace save_png_file with save_svg_file

**Files:**
- Modify: `src-tauri/src/commands/util_commands.rs`
- Modify: `src-tauri/src/plugins/core.rs`
- Modify: `src-tauri/Cargo.toml`

- [ ] **Step 1: Replace util_commands.rs entirely**

```rust
#[tauri::command]
pub async fn save_svg_file(path: String, svg: String) -> Result<(), String> {
    let dest = std::path::PathBuf::from(&path);
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("mkdir error: {e}"))?;
    }
    std::fs::write(&dest, svg.as_bytes()).map_err(|e| format!("write error: {e}"))?;
    Ok(())
}
```

- [ ] **Step 2: Update core.rs — swap command name in macro**

In `src-tauri/src/plugins/core.rs`, find:
```rust
            // Util commands
            save_png_file,
```
Replace with:
```rust
            // Util commands
            save_svg_file,
```

- [ ] **Step 3: Remove base64 from Cargo.toml**

In `src-tauri/Cargo.toml`, remove the line:
```toml
base64 = "0.22"
```

- [ ] **Step 4: Run cargo check**

```bash
cd src-tauri && cargo check 2>&1 | tail -5
```

Expected: `Finished` with no errors (warnings are OK).

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands/util_commands.rs src-tauri/src/plugins/core.rs src-tauri/Cargo.toml src-tauri/Cargo.lock
git commit -m "feat(erd): replace save_png_file with save_svg_file, remove base64 dep"
```

---

## Task 7: Final verification

- [ ] **Step 1: Full type check**

```bash
pnpm check
```

Expected: `0 ERRORS`.

- [ ] **Step 2: Full Rust check**

```bash
cd src-tauri && cargo check 2>&1 | grep -E "^error|Finished"
```

Expected: `Finished` only, no `error` lines.

- [ ] **Step 3: Visual smoke test**

Run `pnpm tauri dev` and verify:
- ERD opens with ELK layout (tables spread across both axes)
- Node headers show `schema.table_name`
- Nodes have opaque dark background matching theme
- Only PK/FK columns show handles (dots on left/right)
- PK columns show amber key icon + "PK" badge
- FK columns show blue arrow icon + "FK" badge
- Edges have arrowheads pointing at the referenced table
- Self-referencing FK edges loop to the right side of the node (not through it)
- Dragging a node corner resizes it
- Background dots match theme border colour
- "Export SVG" button opens save dialog, saves a valid `.svg` file that opens in a browser

- [ ] **Step 4: Final commit if any last fixes**

```bash
git add -p
git commit -m "fix(erd): post-integration tweaks"
```
