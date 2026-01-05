// src/lib/explorer/stores/nodes.svelte.ts
// Note: Svelte 5 uses $state for reactivity. We use a class-based store with runes.


import type { ExplorerNode } from "../types";

export class NodeStore {
    // The core source of truth: a flat map of ID -> Node
    // We use $state for deep reactivity if needed, or just $state.raw for performance if we treat nodes as immutable snapshots
    // Since we update fields like loadState, deeply reactive Map is useful.
    // However, Svelte 5 `Map` support in $state is tricky.
    // Best practice: internal Map, exposed via accessor, triggers updates via reassignment or fine-grained signals.

    // Let's use a simple reactive map wrapper approach or just $state(Map) if supported (it is effectively supported as a reactive container).

    private _nodes = $state(new Map<string, ExplorerNode>());

    get map() {
        return this._nodes;
    }

    get(id: string): ExplorerNode | undefined {
        return this._nodes.get(id);
    }

    getAll(): ExplorerNode[] {
        return Array.from(this._nodes.values());
    }

    upsert(node: ExplorerNode) {
        // If node exists, merge properties? Or overwrite? 
        // For now, overwrite is safer for consistency, but we might want to preserve some UI state if we had it separately.
        // Since UI state like 'expanded' is usually separate, overwrite is okay.
        // BUT: careful with parentId if it changes (rare).
        this._nodes.set(node.id, clone(node));
    }

    update(id: string, patch: Partial<ExplorerNode>) {
        const node = this._nodes.get(id);
        if (!node) return;

        const updated = { ...node, ...patch };
        this._nodes.set(id, updated);
    }

    remove(id: string) {
        this._nodes.delete(id);
    }

    removeSubtree(rootId: string) {
        // Naive implementation: iterate all nodes to find children.
        // Optimize later if needed with a separate parent -> children index.
        const idsToDelete = new Set<string>();
        const stack = [rootId];

        while (stack.length > 0) {
            const currentId = stack.pop()!;
            idsToDelete.add(currentId);

            // Find children
            for (const [id, node] of this._nodes) {
                if (node.parentId === currentId && !idsToDelete.has(id)) {
                    stack.push(id);
                }
            }
        }

        for (const id of idsToDelete) {
            this._nodes.delete(id);
        }
    }

    clear() {
        this._nodes.clear();
    }
}

// Helper for structural cloning to avoid reference leaks
function clone<T>(obj: T): T {
    return structuredClone(obj);
}

export const nodes = new NodeStore();
