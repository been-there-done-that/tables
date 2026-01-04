<script lang="ts">
    import { writable } from "svelte/store";
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
    import { onMount } from "svelte";

    // Node Types Registry
    const nodeTypes = {
        table: TableNode,
    };

    // State
    let nodes = writable<Node[]>([]);
    let edges = writable<Edge[]>([]);

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
            // Approximate dimension if not measured yet.
            // For accurate layout, we often need real measurements.
            // We can estimate based on column count: ~30px per column row + 40px header.
            const colCount = node.data.columns
                ? (node.data.columns as any[]).length
                : 5;
            const height = 40 + colCount * 28;
            const width = 220; // Fixed width

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

    // React to schema changes
    // We use $effect or reactive statements ($:) depending on Svelte 5 vs 4.
    // The codebase seems to use Svelte 5 runes ($state) in stores, but this file is .svelte.
    // We'll use standard reactive statements for now.

    $: activeSchemaName = schemaStore.activeSchema;
    $: activeDatabaseName = schemaStore.selectedDatabase;
    $: activeConnection = schemaStore.activeConnection;

    function loadGraph() {
        try {
            if (!activeConnection || !activeDatabaseName || !activeSchemaName)
                return;

            const db = schemaStore.databases.find(
                (d) => d.name === activeDatabaseName,
            );
            if (!db) {
                console.warn(
                    "[SchemaVisualizer] Database not found:",
                    activeDatabaseName,
                );
                return;
            }

            const schema = db.schemas.find((s) => s.name === activeSchemaName);
            if (!schema) {
                console.warn(
                    "[SchemaVisualizer] Schema not found:",
                    activeSchemaName,
                );
                return;
            }

            const tables = schema.tables || [];
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
                            // Only add edge if target is in the current view
                            const edgeId = `${t.table_name}-${fk.column_name}->${fk.ref_table}`;
                            newEdges.push({
                                id: edgeId,
                                source: t.table_name,
                                target: fk.ref_table,
                                type: "smoothstep",
                                animated: false,
                                style: "stroke-width: 2px; stroke: #94a3b8;",
                                label: fk.column_name, // Optional: label the edge with the FK column
                            });
                        }
                    });
                }
            });

            // Layout
            const layouted = getLayoutedElements(newNodes, newEdges);
            nodes.set(layouted.nodes as Node[]);
            edges.set(layouted.edges);
        } catch (e) {
            console.error("[SchemaVisualizer] Error loading graph:", e);
        }
    }

    // Reload when dependencies change
    $: if (activeSchemaName && activeDatabaseName && activeConnection) {
        loadGraph();
    }
</script>

<div class="h-full w-full bg-slate-50">
    <SvelteFlow nodes={$nodes} edges={$edges} {nodeTypes} fitView minZoom={0.1}>
        <Controls />
        <Background />
        <MiniMap />
    </SvelteFlow>
</div>
