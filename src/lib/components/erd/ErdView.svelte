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
        buildErdGraph(tables).then(result => {
            localStorage.removeItem(storageKey);
            nodes = result.nodes;
            edges = result.edges;
        });
    }

    let bgColor = $state('');

    onMount(() => {
        bgColor = getComputedStyle(document.documentElement)
            .getPropertyValue('--theme-border-subtle')
            .trim();
    });

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
            // toSvg returns "data:image/svg+xml;charset=utf-8,<url-encoded-svg>"
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
        <Background patternColor={bgColor} />
        <Controls />
        <MiniMap pannable zoomable />
    </SvelteFlow>
</div>
