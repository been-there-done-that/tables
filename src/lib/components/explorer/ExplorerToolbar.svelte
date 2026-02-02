<script lang="ts">
    import { type TreeNode } from "./FileTree.svelte";
    import { windowState } from "$lib/stores/window.svelte";
    import { schemaStore } from "$lib/stores/schema.svelte";
    import { cn } from "$lib/utils";
    import Compact from "$lib/svg/Compact.svelte";
    import Expand from "$lib/svg/Expand.svelte";
    import IconDatabase from "@tabler/icons-svelte/icons/database";
    import IconRefresh from "@tabler/icons-svelte/icons/refresh";
    import IconSqlFile from "$lib/svg/IconSqlFile.svelte";

    interface Props {
        treeData: TreeNode[];
        maxLevel?: number;
        selectedNodeId?: string | null;
        onRefresh?: () => void;
    }

    let {
        treeData,
        maxLevel = 4,
        selectedNodeId = null,
        onRefresh = () => schemaStore.refresh(),
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

<div
    class="flex h-8 flex-none items-center border-b border-border bg-background/50 px-4"
>
    <h2 class="text-sm font-semibold">Explorer</h2>
    <div class="ml-auto flex items-center gap-1">
        <button
            class="p-1 hover:bg-accent rounded-sm text-muted-foreground hover:text-foreground transition-colors"
            title="Expand Level"
            onclick={() => progressiveExpand()}
        >
            <Expand />
        </button>
        <button
            class="p-1 hover:bg-accent rounded-sm text-muted-foreground hover:text-foreground transition-colors"
            title="Collapse Level"
            onclick={() => progressiveCollapse()}
        >
            <Compact />
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
            class="p-1 flex items-center justify-center gap-1 hover:bg-accent rounded-sm text-muted-foreground hover:text-foreground transition-colors"
            title="New SQL Editor"
            onclick={() => activeSession?.openView("editor", "New Query")}
        >
            <IconSqlFile class="size-4" />
        </button>
    </div>
</div>
