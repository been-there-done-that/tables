/**
 * Explorer State Store
 * 
 * Global reactive store for database explorer state.
 * Handles expanded nodes, children cache, and loading state.
 * 
 * This exists because Svelte 5's $bindable doesn't properly propagate
 * Set/Map changes between parent and child components.
 */

import type { ExplorerNode, NodeType } from '$lib/components/explorer/drivers';

// Reactive state using Svelte 5 runes
let _expanded = $state<Set<string>>(new Set());
let _childrenCache = $state<Map<string, ExplorerNode[]>>(new Map());
let _loadingNodes = $state<Set<string>>(new Set());

// Version counter to force reactivity on Map/Set mutations
let _version = $state(0);

/**
 * Explorer state store with reactive getters and mutators.
 */
export const explorerStateStore = {
    // Getters that return current state and create dependencies
    get expanded(): Set<string> {
        // Reading _version creates a dependency for reactivity
        const _ = _version;
        return _expanded;
    },

    get childrenCache(): Map<string, ExplorerNode[]> {
        const _ = _version;
        return _childrenCache;
    },

    get loadingNodes(): Set<string> {
        const _ = _version;
        return _loadingNodes;
    },

    get version(): number {
        return _version;
    },

    // Expansion controls
    expand(nodeId: string) {
        _expanded.add(nodeId);
        _version++;
        console.log('[ExplorerState] Expanded:', nodeId, 'total:', _expanded.size);
    },

    collapse(nodeId: string) {
        _expanded.delete(nodeId);
        _version++;
        console.log('[ExplorerState] Collapsed:', nodeId, 'total:', _expanded.size);
    },

    toggle(nodeId: string): boolean {
        if (_expanded.has(nodeId)) {
            this.collapse(nodeId);
            return false;
        } else {
            this.expand(nodeId);
            return true;
        }
    },

    isExpanded(nodeId: string): boolean {
        const _ = _version;
        return _expanded.has(nodeId);
    },

    // Children cache controls
    setChildren(nodeId: string, children: ExplorerNode[]) {
        _childrenCache.set(nodeId, children);
        _version++;
        console.log('[ExplorerState] Cached children for:', nodeId, 'count:', children.length);
    },

    getChildren(nodeId: string): ExplorerNode[] {
        const _ = _version;
        return _childrenCache.get(nodeId) ?? [];
    },

    hasChildren(nodeId: string): boolean {
        const _ = _version;
        return _childrenCache.has(nodeId);
    },

    // Loading state controls
    setLoading(nodeId: string, isLoading: boolean) {
        if (isLoading) {
            _loadingNodes.add(nodeId);
        } else {
            _loadingNodes.delete(nodeId);
        }
        _version++;
    },

    isLoading(nodeId: string): boolean {
        const _ = _version;
        return _loadingNodes.has(nodeId);
    },

    // Reset all state
    reset() {
        _expanded = new Set();
        _childrenCache = new Map();
        _loadingNodes = new Set();
        _version++;
        console.log('[ExplorerState] Reset all state');
    },

    // Clear children cache only (used when driver changes)
    clearCache() {
        _childrenCache = new Map();
        _version++;
        console.log('[ExplorerState] Cleared children cache');
    }
};
