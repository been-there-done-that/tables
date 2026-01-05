<script lang="ts" module>
    import * as ContextMenu from "$lib/components/ui/context-menu";
    import ExplorerContextMenu from "./ExplorerContextMenu.svelte";
    import type { TreeNode as FileTreeNode } from "./FileTree.svelte";
</script>

<!--
  DatabaseExplorer.svelte - Driver-Based Lazy Loading Schema Tree
  
  Uses the Driver Pattern for database-agnostic tree rendering.
  Uses global explorerStateStore for reactivity (Svelte 5 $bindable is broken for Sets/Maps).
-->
<script lang="ts">
    import LazyTree, {
        type TreeNode,
        type NodeContext,
    } from "../tree/LazyTree.svelte";
    import { cn } from "$lib/utils";
    import {
        type DatabaseDriver,
        type ExplorerNode,
        type NodeType,
        type MetaColumn,
    } from "./drivers";
    import {
        IconSchema,
        IconTable,
        IconColumns,
        IconChevronRight,
        IconChevronDown,
        IconLoader2,
        IconKey,
        IconDatabase,
        IconFolder,
        IconEye,
        IconLink,
        IconListDetails,
        IconBolt,
    } from "@tabler/icons-svelte";
    import { schemaStore } from "$lib/stores/schema.svelte";
    import { explorerStateStore } from "$lib/stores/explorerState.svelte";

    // Props
    let {
        driver,
        selected = $bindable<string | null>(null),
        onNodeSelect,
        onContextMenuAction,
    }: {
        driver: DatabaseDriver | null;
        selected?: string | null;
        onNodeSelect?: (node: ExplorerNode) => void;
        onContextMenuAction?: (action: string, node: any) => void;
    } = $props();

    // Re-export for external use
    export type { ExplorerNode };

    // Data state (only rootNodes is local, everything else uses global store)
    let rootNodes = $state<ExplorerNode[]>([]);
    let isLoadingRoots = $state(false);
    let errorMessage = $state<string | null>(null);

    // Fetch root nodes when driver changes
    $effect(() => {
        if (!driver) {
            rootNodes = [];
            explorerStateStore.clearCache();
            return;
        }

        const currentDriver = driver;
        isLoadingRoots = true;
        errorMessage = null;

        currentDriver
            .getRoots()
            .then((result) => {
                rootNodes = result;
                isLoadingRoots = false;
            })
            .catch((err) => {
                errorMessage = String(err);
                isLoadingRoots = false;
            });
    });

    // Clear cache when driver changes
    $effect(() => {
        const _ = driver;
        return () => {
            explorerStateStore.clearCache();
        };
    });

    // Get node type from ID prefix
    function getNodeTypeFromId(id: string): NodeType | null {
        if (id.startsWith("schema:")) return "schema";
        if (id.startsWith("folder:")) return "folder";
        if (id.startsWith("table:")) return "table";
        if (id.startsWith("view:")) return "view";
        if (id.startsWith("column:")) return "column";
        if (id.startsWith("foreign_key:")) return "foreign_key";
        if (id.startsWith("index:")) return "index";
        if (id.startsWith("trigger:")) return "trigger";
        if (id.startsWith("database:")) return "database";
        return null;
    }

    // Build flat tree from hierarchical data
    // Uses explorerStateStore for reactivity
    let treeNodes = $derived.by(() => {
        // Force dependency tracking via store version
        const version = explorerStateStore.version;
        const roots = rootNodes;

        console.log(
            "[DatabaseExplorer] Rebuilding treeNodes, version:",
            version,
            "roots:",
            roots.length,
        );

        const nodes: TreeNode<ExplorerNode>[] = [];

        function addNode(node: ExplorerNode, level: number) {
            nodes.push({
                id: node.id,
                data: node,
                hasChildren: node.hasChildren,
                level,
            });

            if (explorerStateStore.isExpanded(node.id)) {
                const children = explorerStateStore.getChildren(node.id);
                for (const child of children) {
                    addNode(child, level + 1);
                }
            }
        }

        for (const root of roots) {
            addNode(root, 0);
        }

        return nodes;
    });

    // Handle expansion - directly load children when node is expanded
    function handleExpand(node: TreeNode<ExplorerNode>) {
        if (!driver) return;

        const nodeId = node.id;
        const nodeType = getNodeTypeFromId(nodeId);

        console.log(
            "[DatabaseExplorer] handleExpand called for:",
            nodeId,
            "type:",
            nodeType,
        );

        // Skip if already cached or loading
        if (explorerStateStore.hasChildren(nodeId)) {
            console.log("[DatabaseExplorer] Already cached, skipping load");
            return;
        }
        if (explorerStateStore.isLoading(nodeId)) {
            console.log("[DatabaseExplorer] Already loading, skipping");
            return;
        }
        if (!nodeType) {
            console.log("[DatabaseExplorer] Unknown node type, skipping");
            return;
        }

        console.log("[DatabaseExplorer] Loading children for:", nodeId);

        // Mark as loading
        explorerStateStore.setLoading(nodeId, true);

        driver
            .getChildren(nodeId, nodeType)
            .then((children) => {
                console.log(
                    "[DatabaseExplorer] Loaded",
                    children.length,
                    "children for",
                    nodeId,
                );
                explorerStateStore.setChildren(nodeId, children);
                explorerStateStore.setLoading(nodeId, false);

                // Sync columns to completion engine
                if (nodeType === "table" && children.length > 0) {
                    const firstChild = children[0];
                    if (
                        firstChild.metadata.schema &&
                        firstChild.metadata.tableName
                    ) {
                        const columns = children
                            .map((c) => c.metadata.raw as MetaColumn)
                            .filter(Boolean);
                        schemaStore.cacheColumns(
                            driver!.database,
                            firstChild.metadata.schema,
                            firstChild.metadata.tableName,
                            columns,
                        );
                    }
                }
            })
            .catch((err) => {
                console.error(
                    "[DatabaseExplorer] Failed to fetch children for",
                    nodeId,
                    err,
                );
                explorerStateStore.setLoading(nodeId, false);
            });
    }

    // Handle selection
    function handleSelect(node: TreeNode<ExplorerNode>) {
        onNodeSelect?.(node.data);
    }

    // Map icon type to Tabler icon component
    function getIcon(node: ExplorerNode) {
        // Special case for primary key columns
        if (node.type === "column" && node.metadata.raw) {
            const column = node.metadata.raw as MetaColumn;
            if (column.is_primary_key) return IconKey;
        }

        switch (node.icon) {
            case "database":
                return IconDatabase;
            case "schema":
                return IconSchema;
            case "table":
                return IconTable;
            case "view":
                return IconEye;
            case "column":
                return IconColumns;
            case "key":
                return IconKey;
            case "folder":
                return IconFolder;
            case "foreign_key":
                return IconLink;
            case "index":
                return IconListDetails;
            case "trigger":
                return IconBolt;
            default:
                return IconFolder;
        }
    }

    // Check if column is primary key
    function isPrimaryKey(node: ExplorerNode): boolean {
        if (node.type !== "column" || !node.metadata.raw) return false;
        const column = node.metadata.raw as MetaColumn;
        return column.is_primary_key;
    }

    // Helper to stop propagation
    function handleToggleClick(e: MouseEvent, toggle: () => void) {
        e.stopPropagation();
        toggle();
    }
</script>

{#if !driver}
    <div class="p-4 text-muted-foreground text-sm">No database selected</div>
{:else if isLoadingRoots}
    <div class="p-4 flex items-center gap-2 text-muted-foreground text-sm">
        <IconLoader2 class="w-4 h-4 animate-spin" />
        Loading...
    </div>
{:else if errorMessage}
    <div class="p-4 text-destructive text-sm">
        Error: {errorMessage}
    </div>
{:else}
    <LazyTree
        nodes={treeNodes}
        {selected}
        onExpand={handleExpand}
        onSelect={handleSelect}
        class="text-[13px]"
    >
        {#snippet renderNode(ctx: NodeContext<ExplorerNode>)}
            {@const Icon = getIcon(ctx.node.data)}
            {@const contextMenuNode = {
                id: ctx.node.id,
                name: ctx.node.data.label,
                type: ctx.node.data.type,
                metadata: {
                    dbName: ctx.node.data.metadata.database,
                    schemaName: ctx.node.data.metadata.schema,
                    tableName: ctx.node.data.metadata.tableName,
                },
            } as FileTreeNode}

            <ContextMenu.Root>
                <ContextMenu.Trigger>
                    <div
                        class={cn(
                            "flex items-center gap-1 px-2 py-1 cursor-pointer rounded transition-colors hover:bg-hover",
                            ctx.isSelected &&
                                "bg-accent text-accent-foreground",
                        )}
                        style="padding-left: {ctx.node.level * 16 + 8}px"
                        role="treeitem"
                        tabindex="0"
                        aria-expanded={ctx.node.hasChildren
                            ? ctx.isExpanded
                            : undefined}
                        aria-selected={ctx.isSelected}
                        onclick={ctx.select}
                        ondblclick={() => ctx.node.hasChildren && ctx.toggle()}
                    >
                        <!-- Expansion arrow -->
                        {#if ctx.node.hasChildren}
                            <button
                                class="flex items-center justify-center w-4 h-4 p-0 bg-transparent border-none cursor-pointer opacity-60 hover:opacity-100 text-inherit"
                                onclick={(e) =>
                                    handleToggleClick(e, ctx.toggle)}
                                aria-label={ctx.isExpanded
                                    ? "Collapse"
                                    : "Expand"}
                            >
                                {#if ctx.isLoading}
                                    <IconLoader2 class="w-3 h-3 animate-spin" />
                                {:else if ctx.isExpanded}
                                    <IconChevronDown class="w-3 h-3" />
                                {:else}
                                    <IconChevronRight class="w-3 h-3" />
                                {/if}
                            </button>
                        {:else}
                            <span class="w-4 h-4"></span>
                        {/if}

                        <!-- Icon -->
                        <Icon
                            class={cn(
                                "w-4 h-4 shrink-0 text-muted-foreground",
                                isPrimaryKey(ctx.node.data) &&
                                    "text-yellow-500",
                            )}
                        />

                        <!-- Label -->
                        <span class="flex-1 min-w-0 truncate">
                            {ctx.node.data.label}
                            {#if ctx.node.data.secondaryLabel}
                                <span
                                    class="ml-2 text-[10px] text-muted-foreground/60"
                                    >{ctx.node.data.secondaryLabel}</span
                                >
                            {/if}
                        </span>
                    </div>
                </ContextMenu.Trigger>
                <ExplorerContextMenu
                    node={contextMenuNode}
                    onAction={onContextMenuAction}
                />
            </ContextMenu.Root>
        {/snippet}
    </LazyTree>
{/if}
