// src/lib/explorer/events.ts
import { listen } from "@tauri-apps/api/event";
import { nodes } from "./stores/nodes.svelte";
import { jobs } from "./stores/jobs.svelte";
import type { ExplorerNode } from "./types";

let unlistenFns: (() => void)[] = [];

export async function initExplorerEvents() {
    // Cleanup old listeners if any
    cleanupExplorerEvents();

    console.log("[ExplorerEvents] Initializing listeners...");

    // 1. Level Complete (Partial progress)
    const unlistenLevel = await listen("introspection:level_complete", (event: any) => {
        const payload = event.payload;
        console.log("[ExplorerEvents] Level Complete:", payload);

        // Level 2 = Schemas discovered (Database is now 'partial' ready)
        if (payload.level === 2 && payload.database) {
            updateNodeStateByMeta(
                { database: payload.database },
                'database',
                { loadState: 'partial', childCount: payload.schemas?.length }
            );
        }
    });

    // 2. Schema Ready (Tables/Views fully cached for a schema)
    const unlistenSchema = await listen("introspection:schema_ready", (event: any) => {
        const payload = event.payload;
        console.log("[ExplorerEvents] Schema Ready:", payload);

        if (payload.database && payload.schema) {
            // Find the schema node and mark it ready
            updateNodeStateByMeta(
                { database: payload.database, schema: payload.schema },
                'schema',
                { loadState: 'ready' } // It is ready to be expanded fully
            );
        } else if (payload.database) {
            // Sometimes we just get database ready signal
            updateNodeStateByMeta(
                { database: payload.database },
                'database',
                { loadState: 'ready' }
            );
        }
    });

    // 3. Introspection Complete (Job done)
    const unlistenComplete = await listen("introspection:complete", (event: any) => {
        const payload = event.payload;
        console.log("[ExplorerEvents] Complete:", payload);

        // Based on scope, mark things ready
        const scope = payload.scope;
        if (scope.type === 'database') {
            updateNodeStateByMeta(
                { database: scope.name },
                'database',
                { loadState: 'ready' }
            );
        } else if (scope.type === 'schema') {
            updateNodeStateByMeta(
                { database: scope.database, schema: scope.schema },
                'schema',
                { loadState: 'ready' }
            );
        }

        // We might also want to clear any running jobs here if we were tracking them by ID and missed the clean up
    });

    const unlistenError = await listen("introspection:error", (event: any) => {
        const payload = event.payload;
        console.error("[ExplorerEvents] Error:", payload);
        // Find related node if possible and mark error? 
        // Hard to map generic error to specific node without context ID, but we can try.
    });

    unlistenFns.push(unlistenLevel, unlistenSchema, unlistenComplete, unlistenError);
}

export function cleanupExplorerEvents() {
    unlistenFns.forEach(fn => fn());
    unlistenFns = [];
}

/**
 * Helper to find a node by its metadata identity and update it.
 * This is O(N) currenty, but N is usually small (visible nodes). 
 * If N grows, we need an index.
 */
function updateNodeStateByMeta(metaMatch: Record<string, any>, kind: string, patch: Partial<ExplorerNode>) {
    // We only update loaded nodes.
    const allNodes = nodes.getAll();

    // We match by kind + specific meta keys
    const target = allNodes.find(n => {
        if (n.kind !== kind) return false;
        if (!n.meta) return false;

        // Check all keys in metaMatch
        for (const [k, v] of Object.entries(metaMatch)) {
            if (n.meta[k] !== v) return false;
        }
        return true;
    });

    if (target) {
        console.log(`[ExplorerEvents] Updating ${target.id} state to`, patch.loadState);
        nodes.update(target.id, patch);
    }
}
