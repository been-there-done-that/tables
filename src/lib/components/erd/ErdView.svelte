<script lang="ts">
    import { SvelteFlow, MiniMap, Controls, Background, type Node, type Edge } from '@xyflow/svelte';
    import '@xyflow/svelte/dist/style.css';
    import ErdTableNode from './ErdTableNode.svelte';
    import { buildErdGraph } from './erd-layout';
    import type { MetaTable } from '$lib/commands/types';
    import IconLayout from '@tabler/icons-svelte/icons/layout-board';
    import IconDownload from '@tabler/icons-svelte/icons/download';
    import { toPng } from 'html-to-image';

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

    function savePositions(currentNodes: Node[]) {
        const pos: Record<string, { x: number; y: number }> = {};
        for (const n of currentNodes) pos[n.id] = n.position;
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

    let nodes = $state<Node[]>([]);
    let edges = $state<Edge[]>([]);

    $effect(() => {
        const result = buildErdGraphWithSavedPositions(tables);
        nodes = result.nodes;
        edges = result.edges;
    });

    function autoLayout() {
        localStorage.removeItem(storageKey);
        const { nodes: fresh, edges: freshEdges } = buildErdGraph(tables);
        nodes = fresh;
        edges = freshEdges;
    }

    let downloading = $state(false);

    async function downloadImage() {
        console.log('[ERD download] starting');
        const el = document.querySelector<HTMLElement>('.svelte-flow');
        console.log('[ERD download] target element:', el, el ? `${el.offsetWidth}x${el.offsetHeight}` : 'NOT FOUND');
        if (!el) {
            console.error('[ERD download] .svelte-flow not found in DOM');
            return;
        }
        downloading = true;
        try {
            console.log('[ERD download] calling toPng...');
            const dataUrl = await toPng(el, { cacheBust: true, pixelRatio: 2 });
            console.log('[ERD download] dataUrl length:', dataUrl.length, 'prefix:', dataUrl.slice(0, 60));
            const a = document.createElement('a');
            a.href = dataUrl;
            a.download = `erd-${schema}-${new Date().toISOString().slice(0, 10)}.png`;
            document.body.appendChild(a);
            a.click();
            document.body.removeChild(a);
            console.log('[ERD download] click triggered');
        } catch (err) {
            console.error('[ERD download] error:', err);
        } finally {
            downloading = false;
        }
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
        <button
            class="flex items-center gap-1.5 rounded-md border border-border bg-background px-2 py-1 text-xs hover:bg-accent shadow-sm disabled:opacity-50"
            onclick={downloadImage}
            disabled={downloading}
            title="Download as PNG"
        >
            <IconDownload class="h-3.5 w-3.5" />
            {downloading ? 'Saving…' : 'Download PNG'}
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
        onnodedragstop={({ nodes: currentNodes }) => savePositions(currentNodes)}
    >
        <Background />
        <Controls />
        <MiniMap pannable zoomable />
    </SvelteFlow>
</div>
