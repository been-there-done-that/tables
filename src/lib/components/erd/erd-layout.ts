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
