<script lang="ts" module>
    import * as ContextMenu from "$lib/components/ui/context-menu";
    import ExplorerContextMenu from "./ExplorerContextMenu.svelte";
</script>

<!--
  DatabaseExplorer.svelte - Lazy Loading Schema Tree
  
  Shows schemas for the currently selected database at the root level.
  Hierarchy: Schema → Table → Column
  
  Uses $effect-based lazy loading for reliable reactivity.
-->
<script lang="ts">
    import LazyTree, {
        type TreeNode,
        type NodeContext,
    } from "../tree/LazyTree.svelte";
    import { cn } from "$lib/utils";
    import {
        type MetaSchema,
        type MetaTable,
        type MetaColumn,
    } from "$lib/query/schemaQueries";
    import {
        IconSchema,
        IconTable,
        IconColumns,
        IconChevronRight,
        IconChevronDown,
        IconLoader2,
        IconKey,
    } from "@tabler/icons-svelte";
    import { invoke } from "@tauri-apps/api/core";
    import { schemaStore } from "$lib/stores/schema.svelte";

    // Props
    let {
        connectionId,
        database,
        expanded = $bindable(new Set<string>()),
        selected = $bindable<string | null>(null),
        onNodeSelect,
        onContextMenuAction,
    }: {
        connectionId: string | null;
        database: string | null;
        expanded?: Set<string>;
        selected?: string | null;
        onNodeSelect?: (node: ExplorerNodeData) => void;
        onContextMenuAction?: (action: string, node: any) => void;
    } = $props();

    // Node type discriminator
    type NodeType = "schema" | "table" | "column";

    export interface ExplorerNodeData {
        type: NodeType;
        name: string;
        schema?: MetaSchema;
        table?: MetaTable;
        column?: MetaColumn;
        parentSchema?: string;
        parentTable?: string;
    }

    // Data state
    let schemas = $state<MetaSchema[]>([]);
    let tablesCache = $state<Map<string, MetaTable[]>>(new Map());
    let columnsCache = $state<Map<string, MetaColumn[]>>(new Map());

    // Loading state
    let loadingNodes = $state(new Set<string>());
    let isLoadingSchemas = $state(false);
    let errorMessage = $state<string | null>(null);

    // Helper to parse expanded IDs
    const expandedSchemaNames = $derived(
        [...expanded]
            .filter((id) => id.startsWith("schema:"))
            .map((id) => id.split(":")[1]),
    );

    const expandedTableKeys = $derived(
        [...expanded]
            .filter((id) => id.startsWith("table:"))
            .map((id) => {
                const parts = id.split(":");
                return {
                    schema: parts[1],
                    table: parts[2],
                    key: `${parts[1]}:${parts[2]}`,
                };
            }),
    );

    // Fetch schemas when connection/database changes
    $effect(() => {
        if (!connectionId || !database) {
            schemas = [];
            return;
        }

        isLoadingSchemas = true;
        errorMessage = null;

        invoke<MetaSchema[]>("get_schemas_lazy", { connectionId, database })
            .then((result) => {
                schemas = result;
                isLoadingSchemas = false;
            })
            .catch((err) => {
                errorMessage = String(err);
                isLoadingSchemas = false;
            });
    });

    // Fetch tables when schemas are expanded
    $effect(() => {
        if (!connectionId || !database) return;

        for (const schemaName of expandedSchemaNames) {
            // Skip if already cached
            if (tablesCache.has(schemaName)) continue;

            const nodeId = `schema:${schemaName}`;
            loadingNodes.add(nodeId);
            loadingNodes = new Set(loadingNodes);

            invoke<MetaTable[]>("get_tables_lazy", {
                connectionId,
                database,
                schema: schemaName,
            })
                .then((result) => {
                    tablesCache.set(schemaName, result);
                    tablesCache = new Map(tablesCache); // Trigger reactivity
                    loadingNodes.delete(nodeId);
                    loadingNodes = new Set(loadingNodes);
                })
                .catch((err) => {
                    console.error(
                        `Failed to fetch tables for ${schemaName}:`,
                        err,
                    );
                    loadingNodes.delete(nodeId);
                    loadingNodes = new Set(loadingNodes);
                });
        }
    });

    // Fetch columns when tables are expanded
    $effect(() => {
        if (!connectionId || !database) return;

        for (const { schema, table, key } of expandedTableKeys) {
            // Skip if already cached
            if (columnsCache.has(key)) continue;

            const nodeId = `table:${schema}:${table}`;
            loadingNodes.add(nodeId);
            loadingNodes = new Set(loadingNodes);

            invoke<MetaColumn[]>("get_columns_lazy", {
                connectionId,
                database,
                schema,
                table,
            })
                .then((result) => {
                    columnsCache.set(key, result);
                    columnsCache = new Map(columnsCache);
                    loadingNodes.delete(nodeId);
                    loadingNodes = new Set(loadingNodes);

                    // Sync to completion engine
                    schemaStore.cacheColumns(database!, schema, table, result);
                })
                .catch((err) => {
                    console.error(`Failed to fetch columns for ${table}:`, err);
                    loadingNodes.delete(nodeId);
                    loadingNodes = new Set(loadingNodes);
                });
        }
    });

    // Clear caches when connection/database changes
    $effect(() => {
        // This runs when connectionId or database changes
        const _ = connectionId;
        const __ = database;

        // Clear caches (but not on initial mount)
        return () => {
            tablesCache = new Map();
            columnsCache = new Map();
        };
    });

    // Helper functions to get data
    function getTables(schemaName: string): MetaTable[] {
        return tablesCache.get(schemaName) ?? [];
    }

    function getColumns(schemaName: string, tableName: string): MetaColumn[] {
        return columnsCache.get(`${schemaName}:${tableName}`) ?? [];
    }

    // Build flat tree from hierarchical data
    let treeNodes = $derived.by(() => {
        const nodes: TreeNode<ExplorerNodeData>[] = [];

        for (const schema of schemas) {
            const schemaId = `schema:${schema.name}`;
            nodes.push({
                id: schemaId,
                data: { type: "schema", name: schema.name, schema },
                hasChildren: true,
                level: 0,
            });

            // If schema expanded, add tables
            if (expanded.has(schemaId)) {
                const tables = getTables(schema.name);
                for (const table of tables) {
                    const tableId = `table:${schema.name}:${table.table_name}`;
                    nodes.push({
                        id: tableId,
                        data: {
                            type: "table",
                            name: table.table_name,
                            table,
                            parentSchema: schema.name,
                        },
                        hasChildren: true,
                        level: 1,
                    });

                    // If table expanded, add columns
                    if (expanded.has(tableId)) {
                        const columns = getColumns(
                            schema.name,
                            table.table_name,
                        );
                        for (const col of columns) {
                            const colId = `col:${schema.name}:${table.table_name}:${col.column_name}`;
                            nodes.push({
                                id: colId,
                                data: {
                                    type: "column",
                                    name: col.column_name,
                                    column: col,
                                    parentSchema: schema.name,
                                    parentTable: table.table_name,
                                },
                                hasChildren: false,
                                level: 2,
                            });
                        }
                    }
                }
            }
        }

        return nodes;
    });

    // Handle expansion (no-op, data loading is handled by effects)
    function handleExpand(node: TreeNode<ExplorerNodeData>) {
        // Effects will handle loading
    }

    // Handle selection
    function handleSelect(node: TreeNode<ExplorerNodeData>) {
        onNodeSelect?.(node.data);
    }

    // Get icon for node type
    function getIcon(data: ExplorerNodeData) {
        if (data.type === "column" && data.column?.is_primary_key) {
            return IconKey;
        }

        switch (data.type) {
            case "schema":
                return IconSchema;
            case "table":
                return IconTable;
            case "column":
                return IconColumns;
        }
    }

    // Helper to stop propagation
    function handleToggleClick(e: MouseEvent, toggle: () => void) {
        e.stopPropagation();
        toggle();
    }
</script>

{#if !connectionId || !database}
    <div class="p-4 text-muted-foreground text-sm">No database selected</div>
{:else if isLoadingSchemas}
    <div class="p-4 flex items-center gap-2 text-muted-foreground text-sm">
        <IconLoader2 class="w-4 h-4 animate-spin" />
        Loading schemas...
    </div>
{:else if errorMessage}
    <div class="p-4 text-destructive text-sm">
        Error: {errorMessage}
    </div>
{:else}
    <LazyTree
        nodes={treeNodes}
        {expanded}
        {selected}
        {loadingNodes}
        onExpand={handleExpand}
        onSelect={handleSelect}
        class="text-[13px]"
    >
        {#snippet renderNode(ctx: NodeContext<ExplorerNodeData>)}
            {@const Icon = getIcon(ctx.node.data)}
            {@const contextMenuNode = {
                id: ctx.node.id,
                name: ctx.node.data.name,
                type: ctx.node.data.type,
                metadata: {
                    dbName: database,
                    schemaName:
                        ctx.node.data.schema?.name ||
                        ctx.node.data.parentSchema,
                    tableName:
                        ctx.node.data.table?.table_name ||
                        ctx.node.data.parentTable,
                },
            }}

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
                                ctx.node.data.column?.is_primary_key &&
                                    "text-yellow-500",
                            )}
                        />

                        <!-- Name -->
                        <span class="flex-1 min-w-0 truncate">
                            {ctx.node.data.name}
                            {#if ctx.node.data.type === "column" && ctx.node.data.column}
                                <span
                                    class="ml-2 text-[10px] text-muted-foreground/60"
                                    >{ctx.node.data.column.logical_type}</span
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
