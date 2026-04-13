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
                fetchedCount++;
                if (result.status === 'fulfilled') {
                    results.push(result.value);
                } else {
                    results.push(chunk[j]); // fallback to stub on error
                    failedCount++;
                }
            }
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

    let fetchGeneration = 0;

    $effect(() => {
        if (tables.length === 0) return;
        const gen = ++fetchGeneration;
        chipVisible = true;
        loadPhase = 'fetching';
        nodes = [];
        edges = [];

        fetchEnrichedTables(tables)
            .then(enriched => {
                if (gen !== fetchGeneration) return null;
                enrichedTables = enriched;
                loadPhase = 'layout';
                return buildWithSavedPositions(enriched);
            })
            .then(result => {
                if (!result || gen !== fetchGeneration) return;
                nodes = result.nodes;
                edges = result.edges;
                loadPhase = 'done';
                setTimeout(() => { chipVisible = false; }, 1500);
            })
            .catch(() => {
                if (gen !== fetchGeneration) return;
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

        const defaultName = `erd-${schema}-${new Date().toISOString().slice(0, 10)}.svg`;
        const path = await save({
            defaultPath: defaultName,
            filters: [{ name: 'SVG Image', extensions: ['svg'] }],
        });
        if (!path) return;

        // Show overlay and yield a frame so the UI renders before the blocking toSvg call.
        downloading = true;
        await new Promise(resolve => setTimeout(resolve, 50));

        try {
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

<div class="relative h-full w-full bg-(--theme-bg-primary)">
    <!-- Toolbar overlay -->
    <div class="absolute top-2 left-2 z-10 flex items-center gap-1">
        <button
            class="flex items-center gap-1.5 rounded-md border border-(--theme-border-default) bg-(--theme-bg-secondary) px-2 py-1 text-xs text-(--theme-fg-primary) hover:bg-(--theme-bg-hover) shadow-sm disabled:opacity-50"
            onclick={autoLayout}
            disabled={loadPhase !== 'done' && loadPhase !== 'error'}
            title="Reset layout"
        >
            <IconLayout class="h-3.5 w-3.5" />
            Auto layout
        </button>
        <button
            class="flex items-center gap-1.5 rounded-md border border-(--theme-border-default) bg-(--theme-bg-secondary) px-2 py-1 text-xs text-(--theme-fg-primary) hover:bg-(--theme-bg-hover) shadow-sm disabled:opacity-50"
            onclick={downloadImage}
            disabled={downloading}
            title="Export as SVG"
        >
            <IconDownload class="h-3.5 w-3.5" />
            {downloading ? 'Generating…' : 'Export SVG'}
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

    <!-- Download overlay: blocks interaction and shows progress while toSvg runs -->
    {#if downloading}
        <div class="absolute inset-0 z-50 flex items-center justify-center bg-black/40">
            <div class="flex items-center gap-3 rounded-lg border border-(--theme-border-default) bg-(--theme-bg-secondary) px-6 py-4 text-sm text-(--theme-fg-primary) shadow-lg">
                <span class="inline-block animate-spin text-base">⟳</span>
                <span>Generating SVG export…</span>
            </div>
        </div>
    {/if}

    <!-- Progress chip: bottom-left, fades out on done -->
    {#if chipVisible}
        <div
            class="absolute bottom-4 left-4 z-20 flex items-center gap-2 rounded-md border border-(--theme-border-default) bg-(--theme-bg-secondary) px-3 py-2 text-xs text-(--theme-fg-primary) shadow-md transition-opacity duration-700"
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
