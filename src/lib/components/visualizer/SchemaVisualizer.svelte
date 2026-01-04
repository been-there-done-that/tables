<script lang="ts">
    import {
        SvelteFlow,
        Background,
        Controls,
        MiniMap,
        useSvelteFlow,
        type Node,
        type Edge,
    } from "@xyflow/svelte";
    import "@xyflow/svelte/dist/style.css";
    import TableNode from "./TableNode.svelte";
    import dagre from "dagre";
    import { schemaStore } from "$lib/stores/schema.svelte";

    // Props
    let {
        database = null,
        schema = null,
        focusedTable = null,
    }: {
        database?: string | null;
        schema?: string | null;
        focusedTable?: string | null;
    } = $props();

    // Node Types Registry
    const nodeTypes = {
        table: TableNode,
    };

    // State with Svelte 5 Runes (using .raw for performance with large arrays)
    let nodes = $state.raw<Node[]>([]);
    let edges = $state.raw<Edge[]>([]);

    const dagreGraph = new dagre.graphlib.Graph();
    dagreGraph.setDefaultEdgeLabel(() => ({}));

    // Helper to layout graph
    const getLayoutedElements = (
        _nodes: Node[],
        _edges: Edge[],
        direction = "LR",
    ) => {
        const isHorizontal = direction === "LR";
        dagreGraph.setGraph({ rankdir: direction });

        _nodes.forEach((node) => {
            const colCount = node.data.columns
                ? (node.data.columns as any[]).length
                : 5;
            const height = 44 + colCount * 28; // Adjusted estimate
            const width = 240;

            dagreGraph.setNode(node.id, { width, height });
        });

        _edges.forEach((edge) => {
            dagreGraph.setEdge(edge.source, edge.target);
        });

        dagre.layout(dagreGraph);

        return {
            nodes: _nodes.map((node) => {
                const nodeWithPosition = dagreGraph.node(node.id);
                return {
                    ...node,
                    targetPosition: isHorizontal ? "left" : "top",
                    sourcePosition: isHorizontal ? "right" : "bottom",
                    position: {
                        x: nodeWithPosition.x - nodeWithPosition.width / 2,
                        y: nodeWithPosition.y - nodeWithPosition.height / 2,
                    },
                };
            }),
            edges: _edges,
        };
    };

    // Driven state from props or store
    const effectiveDbName = $derived(database || schemaStore.selectedDatabase);
    const effectiveSchemaName = $derived(schema || schemaStore.activeSchema);
    const activeConnection = $derived(schemaStore.activeConnection);

    function loadGraph() {
        try {
            if (!activeConnection || !effectiveDbName || !effectiveSchemaName)
                return;

            const db = schemaStore.databases.find(
                (d) => d.name === effectiveDbName,
            );
            if (!db) {
                console.warn(
                    "[SchemaVisualizer] Database not found:",
                    effectiveDbName,
                );
                return;
            }

            const schemaData = db.schemas.find(
                (s) => s.name === effectiveSchemaName,
            );
            if (!schemaData) {
                console.warn(
                    "[SchemaVisualizer] Schema not found:",
                    effectiveSchemaName,
                );
                return;
            }

            const tables = schemaData.tables || [];
            console.log(
                "[SchemaVisualizer] Loading graph for tables:",
                tables.length,
            );

            // Create Nodes
            const newNodes: Node[] = tables.map((t) => ({
                id: t.table_name,
                type: "table",
                position: { x: 0, y: 0 },
                data: {
                    label: t.table_name,
                    columns: Array.isArray(t.columns) ? t.columns : [],
                },
                dragHandle: ".custom-drag-handle",
            }));

            // Create Edges
            const newEdges: Edge[] = [];
            const tableNames = new Set(tables.map((t) => t.table_name));

            tables.forEach((t) => {
                if (Array.isArray(t.foreign_keys)) {
                    t.foreign_keys.forEach((fk) => {
                        if (tableNames.has(fk.ref_table)) {
                            const edgeId = `${t.table_name}-${fk.column_name}->${fk.ref_table}`;
                            newEdges.push({
                                id: edgeId,
                                source: t.table_name,
                                target: fk.ref_table,
                                type: "smoothstep",
                                animated: false,
                                style: "stroke-width: 2px; stroke: #94a3b8;",
                                label: fk.column_name,
                            });
                        }
                    });
                }
            });

            // Layout
            const layouted = getLayoutedElements(newNodes, newEdges);
            nodes = layouted.nodes as Node[];
            edges = layouted.edges;
        } catch (e) {
            console.error("[SchemaVisualizer] Error loading graph:", e);
        }
    }

    // Effect to reload graph when dependencies change
    $effect(() => {
        if (effectiveDbName && effectiveSchemaName && activeConnection) {
            // Check focusedTable too just to trigger effect if needed, though we render all now.
            loadGraph();
        }
    });
</script>

<div class="h-full w-full bg-slate-50">
    <SvelteFlow
        {nodes}
        {edges}
        {nodeTypes}
        fitView
        minZoom={0.1}
        nodesDraggable={true}
        nodesConnectable={false}
    >
        <Controls />
        <Background />
        <MiniMap />
    </SvelteFlow>
</div>
