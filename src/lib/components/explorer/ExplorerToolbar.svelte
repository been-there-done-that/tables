<script lang="ts">
    import { type TreeNode } from "./FileTree.svelte";
    import { windowState } from "$lib/stores/window.svelte";
    import { schemaStore } from "$lib/stores/schema.svelte";
    import { cn } from "$lib/utils";
    import IconRefresh from "@tabler/icons-svelte/icons/refresh";
    import IconSearch from "@tabler/icons-svelte/icons/search";
    import IconX from "@tabler/icons-svelte/icons/x";

    interface Props {
        treeData: TreeNode[];
        maxLevel?: number;
        selectedNodeId?: string | null;
        onRefresh?: () => void;
        searchQuery?: string;
    }

    let {
        treeData,
        maxLevel = 4,
        selectedNodeId = null,
        onRefresh = () => schemaStore.refresh(),
        searchQuery = $bindable(""),
    }: Props = $props();

    const activeSession = $derived(windowState.activeSession);

    /**
     * Get all node IDs at a specific depth level from the tree
     */
    function getNodeIdsAtLevel(
        nodes: TreeNode[],
        targetLevel: number,
        currentLevel: number = 0,
    ): string[] {
        const ids: string[] = [];
        if (!nodes) return ids;

        for (const node of nodes) {
            if (currentLevel === targetLevel) {
                if (node.id && node.children?.length) {
                    ids.push(node.id);
                }
            } else if (currentLevel < targetLevel && node.children?.length) {
                ids.push(
                    ...getNodeIdsAtLevel(
                        node.children,
                        targetLevel,
                        currentLevel + 1,
                    ),
                );
            }
        }
        return ids;
    }

    /**
     * Get the root nodes for expansion/collapse based on selection
     */
    function getScopedNodes(): TreeNode[] {
        if (selectedNodeId) {
            // IDs often follow formats like "schema:db:schema", "folder:tables:db:schema", "table:db:schema.table"
            const parts = selectedNodeId.split(":");
            let schemaId = "";

            if (selectedNodeId.startsWith("schema:")) {
                schemaId = selectedNodeId;
            } else if (
                selectedNodeId.startsWith("folder:") ||
                selectedNodeId.startsWith("table:")
            ) {
                // folder:type:db:schema or table:db:schema.table
                const dbName = parts[2];
                const schemaPart = parts[3]?.split(".")[0];
                if (dbName && schemaPart) {
                    schemaId = `schema:${dbName}:${schemaPart}`;
                }
            }

            if (schemaId) {
                // Search in top level first (Postgres style)
                const schemaNode = treeData.find((s) => s.id === schemaId);
                if (schemaNode) return [schemaNode];
            }
        }
        return treeData;
    }

    /**
     * Progressive expand - find first level NOT fully expanded and expand it
     */
    function progressiveExpand() {
        if (!activeSession) return;

        const scopedNodes = getScopedNodes();
        const expanded =
            activeSession.explorerState?.expanded || new Set<string>();
        const newExpanded = new Set(expanded);

        // Find the first level that has nodes not yet expanded
        for (let level = 0; level < maxLevel; level++) {
            const idsAtLevel = getNodeIdsAtLevel(scopedNodes, level);
            const hiddenIds = idsAtLevel.filter((id) => !expanded.has(id));

            if (hiddenIds.length > 0) {
                // Expand all nodes at this level
                for (const id of idsAtLevel) {
                    newExpanded.add(id);
                }
                activeSession.explorerState.expanded = newExpanded;
                return;
            }
        }
    }

    /**
     * Progressive collapse - find deepest level WITH expanded nodes and collapse it
     */
    function progressiveCollapse() {
        if (!activeSession) return;

        const scopedNodes = getScopedNodes();
        const expanded =
            activeSession.explorerState?.expanded || new Set<string>();
        const newExpanded = new Set(expanded);

        // Find the deepest level that has expanded nodes
        for (let level = maxLevel - 1; level >= 0; level--) {
            const idsAtLevel = getNodeIdsAtLevel(scopedNodes, level);
            const visibleIds = idsAtLevel.filter((id) => expanded.has(id));

            if (visibleIds.length > 0) {
                // Collapse all expanded nodes at this deepest level
                for (const id of visibleIds) {
                    newExpanded.delete(id);
                }
                activeSession.explorerState.expanded = newExpanded;
                return;
            }
        }
    }
</script>

<div class="flex-none border-b border-border">
    <!-- Header row -->
    <div class="flex h-8 items-center bg-background/50 px-3">
        <h2 class="text-sm font-semibold">Explorer</h2>
        <div class="ml-auto flex items-center gap-1">
            <button
                class="px-1.5 py-0.5 text-[11px] rounded-sm text-muted-foreground hover:text-foreground hover:bg-accent transition-colors"
                title="Expand Level"
                onclick={() => progressiveExpand()}
            >
                Expand
            </button>
            <button
                class="px-1.5 py-0.5 text-[11px] rounded-sm text-muted-foreground hover:text-foreground hover:bg-accent transition-colors"
                title="Collapse Level"
                onclick={() => progressiveCollapse()}
            >
                Collapse
            </button>

            <div class="mx-1 h-4 w-px bg-border/50"></div>

            <button
                class="p-1 hover:bg-accent rounded-sm text-muted-foreground hover:text-foreground transition-colors"
                title={schemaStore.lastRefreshed
                    ? `Last refreshed: ${schemaStore.lastRefreshed.toLocaleTimeString()}`
                    : "Refresh Schema"}
                onclick={onRefresh}
            >
                <IconRefresh
                    class={cn(
                        "size-4",
                        schemaStore.status === "refreshing"
                            ? "animate-spin-reverse"
                            : "",
                    )}
                />
            </button>

            <div class="mx-1 h-4 w-px bg-border/50"></div>

            <button
                class="px-1.5 py-0.5 text-[11px] font-medium rounded-sm text-muted-foreground hover:text-foreground hover:bg-accent transition-colors"
                title="New SQL Editor"
                onclick={() => activeSession?.openView("editor", "New Query")}
            >
                + New Query
            </button>
        </div>
    </div>

    <!-- Search row -->
    <div class="flex items-center w-full px-3 py-1.5">
        <IconSearch class="size-3.5 shrink-0 text-muted-foreground/60 mr-1.5" />
        <input
            type="text"
            bind:value={searchQuery}
            class="w-full bg-transparent text-xs outline-none placeholder:text-muted-foreground/60"
            placeholder="Filter tables, views, functions..."
        />
        {#if searchQuery}
            <button
                class="shrink-0 text-muted-foreground/60 hover:text-foreground transition-colors"
                onclick={() => (searchQuery = "")}
                title="Clear filter"
            >
                <IconX class="size-3.5" />
            </button>
        {/if}
    </div>
</div>
