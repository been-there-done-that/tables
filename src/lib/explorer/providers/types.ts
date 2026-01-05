// src/lib/explorer/providers/types.ts
import type { ExplorerNode, NodeKind } from "../types";

export interface ExplorerProvider {
    id: string; // e.g., "postgres"

    /**
     * Returns a list of children for a given node.
     * This should primarily read from the cache or return virtual nodes (groups).
     * It should NOT block on long networking calls if possible (use background jobs + loadState).
     */
    listChildren(node: ExplorerNode): Promise<ExplorerNode[]>;

    /**
     * Trigger a refresh for a specific node.
     * This initiates the backend job to update the cache.
     */
    refresh(node: ExplorerNode): Promise<void>;

    /**
     * Define virtual groups for a specific node kind.
     * Returns empty array if no groups exist for this kind.
     */
    groupsFor?(kind: NodeKind): {
        id: string;
        label: string;
        icon?: any;
    }[];
}
