<!-- src/lib/components/explorer/DatabaseExplorer.svelte -->
<script lang="ts">
    import { onMount, onDestroy } from "svelte";
    import ExplorerRow from "./ExplorerRow.svelte";
    import { nodes } from "$lib/explorer/stores/nodes.svelte";
    import {
        initExplorerEvents,
        cleanupExplorerEvents,
    } from "$lib/explorer/events";
    import { PostgresProvider } from "$lib/explorer/providers/postgres";
    import type { ExplorerNode } from "$lib/explorer/types";
    import { toast } from "svelte-sonner";

    // --- State ---
    let expanded = $state(new Set<string>());

    // --- Props ---
    let {
        onAction = (action: string, node: ExplorerNode) => {},
    }: {
        onAction?: (action: string, node: ExplorerNode) => void;
    } = $props();

    // We map provider IDs to instances. currently just one hardcoded.
    const providers = {
        postgres: PostgresProvider,
    };

    // --- Life Cycle ---
    onMount(async () => {
        await initExplorerEvents();

        // Seed the initial logic: we need at least one connection node?
        // OR: backend logic dictates what connects.
        // Assuming 'schemaStore.activeConnection' was the old driver.
        // For this refactor, we should listen to connection events OR just check if we have data.

        // HACK/BRIDGE: Let's grab the active connection from somewhere or rely on the user to "Connect"
        // But since we are replacing the existing component, we need to know *what* to show.
        // Usually `+page.svelte` passes a connection prop or we read a global store.

        // For now, let's assume valid state is managed by `schemaStore` side-effects until we fully port that too.
        // BUT: this component must be self-sufficient.

        // Let's create a ROOT connection node if needed check if `nodes` is empty?
        // Actually, the best way: The parent (layout) calls `connect()` which should seed the store.
        // IF we are hot-reloading: let's inspect `nodes`.
    });

    onDestroy(() => {
        cleanupExplorerEvents();
    });

    // --- Derived Rendering List (Flattening) ---
    // This transforms the flat Map + expanded Set into a linear list of visible rows
    // It's re-calculated whenever `nodes.map` or `expanded` changes.
    // $derived works for this.

    // 1. Find root nodes (parentId === null)
    // 2. recursive traverse if expanded

    const visibleNodes = $derived.by(() => {
        const result: { node: ExplorerNode; depth: number }[] = [];
        const allNodes = nodes.getAll();

        // Optimization: build adjacency list for faster traversal
        // Map<parentId, Children[]>
        const childrenMap = new Map<string, ExplorerNode[]>();
        const roots: ExplorerNode[] = [];

        // Single pass to organize
        for (const n of allNodes) {
            if (!n.parentId) {
                roots.push(n);
            } else {
                if (!childrenMap.has(n.parentId))
                    childrenMap.set(n.parentId, []);
                childrenMap.get(n.parentId)!.push(n);
            }
        }

        // Sort helper (can be customized by kind/label)
        const sortNodes = (list: ExplorerNode[]) => {
            return list.sort((a, b) => {
                // Groups first
                if (a.kind === "group" && b.kind !== "group") return -1;
                if (a.kind !== "group" && b.kind === "group") return 1;
                return a.label.localeCompare(b.label);
            });
        };

        // Recursive DFS
        const traverse = (node: ExplorerNode, depth: number) => {
            result.push({ node, depth });

            if (expanded.has(node.id)) {
                const children = childrenMap.get(node.id);
                if (children && children.length > 0) {
                    const sorted = sortNodes(children);
                    for (const child of sorted) {
                        traverse(child, depth + 1);
                    }
                }
            }
        };

        // Start with roots
        for (const root of sortNodes(roots)) {
            traverse(root, 0);
        }

        return result;
    });

    // --- Actions ---

    async function handleToggle(node: ExplorerNode) {
        if (expanded.has(node.id)) {
            // Collapse: just remove from set
            const next = new Set(expanded);
            next.delete(node.id);
            expanded = next;
        } else {
            // Expand
            // 1. Check if we need to fetch children
            // Heuristic: if childCount > 0 but no children in store?
            // Better: Always ask provider.listChildren?
            // Provider.listChildren checks cache.

            // If we are 'loading', prevent toggle?
            if (node.loadState === "loading") return;

            // Optimistic expand
            const next = new Set(expanded);
            next.add(node.id);
            expanded = next;

            // Trigger fetch if needed
            // If loadState is 'idle' or 'partial', we usually want to ensure we have latest children for this level?
            // Actually, we trust the cache.
            // But if it's the *first* time (loadState === 'idle'), we might need to "listChildren" which might resolve virtuals.

            const provider = providers[node.provider as keyof typeof providers];
            if (!provider) return;

            try {
                // Get children (this is fast if cached/virtual)
                const children = await provider.listChildren(node);

                // Diff/Upsert
                // We only add new ones, or update existing?
                // Provider returns fresh snapshots.
                for (const child of children) {
                    nodes.upsert(child);
                }

                // If it's a dynamic node (database/schema) and state is idle, we might want to trigger a refresh for deeper levels?
                // Current design: 'listChildren' for S3/Schema mostly returns what we *know*.
                // Real data fetching is explicit "refresh" or triggered by `listChildren` internally if it misses cache.
                // But `PostgresProvider.listChildren` is purely cache-read.

                // Special Case: Database Expand -> needs Schema Introspection (L2) if not done
                // Schema Expand -> needs Table Introspection (L3) if not done

                if (node.kind === "database" && node.loadState === "idle") {
                    // Trigger L2
                    // Update current node to loading?
                    nodes.update(node.id, { loadState: "loading" });
                    await provider.refresh(node);
                } else if (
                    node.kind === "schema" &&
                    node.loadState === "idle"
                ) {
                    // Trigger L3
                    nodes.update(node.id, { loadState: "loading" });
                    await provider.refresh(node);
                }
            } catch (e) {
                console.error("Expand failed:", e);
                toast.error("Failed to load children");
                // Revert expand if failed?
                const revert = new Set(expanded);
                revert.delete(node.id);
                expanded = revert;
            }
        }
    }

    async function handleRefresh(node: ExplorerNode) {
        if (node.loadState === "loading") return;

        const provider = providers[node.provider as keyof typeof providers];
        if (!provider) return;

        nodes.update(node.id, { loadState: "loading" });
        try {
            await provider.refresh(node);
            // Refresh usually triggers backend events which update the store via `events.ts`
        } catch (e) {
            // Reset state on error
            nodes.update(node.id, { loadState: "error" });
        }
    }

    // Initialize with a connection for testing if empty
    $effect(() => {
        if (nodes.getAll().length === 0) {
            // Check if schemaStore has an active connection (Legacy Bridge)
            // Ideally we shouldn't depend on legacy schemaStore directly here to avoid circular dep,
            // but we need a bridge during migration.
            // For now, let's look at `activeConnection` if imported.
            // Dynamic import to avoid strict dependency loop at module level?
            import("$lib/stores/schema.svelte").then(({ schemaStore }) => {
                if (schemaStore.activeConnection) {
                    const conn = schemaStore.activeConnection;
                    nodes.upsert({
                        id: conn.id,
                        parentId: null,
                        provider: "postgres", // Assume postgres for now
                        connectionId: conn.id,
                        kind: "connection",
                        label: conn.name,
                        loadState: "ready",
                        meta: { database: conn.database },
                    });
                }
            });
        }
    });
</script>

<div
    class="h-full flex flex-col bg-sidebar border-r border-sidebar-border overflow-hidden"
>
    <!-- Header? Optional -->

    <!-- Tree List -->
    <div class="flex-1 overflow-y-auto overflow-x-hidden py-2" role="tree">
        {#each visibleNodes as { node, depth } (node.id)}
            <ExplorerRow
                {node}
                {depth}
                expanded={expanded.has(node.id)}
                onToggle={() => handleToggle(node)}
                onRefresh={() => handleRefresh(node)}
                {onAction}
            />
        {/each}

        {#if visibleNodes.length === 0}
            <div
                class="flex items-center justify-center h-20 text-muted-foreground text-sm"
            >
                No active connection
            </div>
        {/if}
    </div>
</div>
