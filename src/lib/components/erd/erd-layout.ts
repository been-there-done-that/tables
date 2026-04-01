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
                targetHandle: `${fk.ref_column}-target`,
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
        edges: edgeDefs
            .filter(e => !e.isSelfLoop)
            .map(e => ({ id: e.id, sources: [e.source], targets: [e.target] })),
    };

    let layout;
    try {
        layout = await elk.layout(elkGraph);
    } catch (err) {
        throw new Error(`ELK layout failed: ${err instanceof Error ? err.message : String(err)}`);
    }
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
