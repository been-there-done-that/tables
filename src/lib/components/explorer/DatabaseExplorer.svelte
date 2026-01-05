<!--
  DatabaseExplorer.svelte - Lazy Loading Database Tree
  
  This component composes LazyTree with TanStack Query hooks to provide
  a fully lazy-loaded database explorer. It handles:
  - Fetching databases on mount
  - Fetching schemas when database is expanded
  - Fetching tables when schema is expanded
  
  Uses the new architectural pattern: Logic (LazyTree) + Data (TanStack Query)
-->
<script lang="ts">
    import LazyTree, {
        type TreeNode,
        type NodeContext,
    } from "../tree/LazyTree.svelte";
    import {
        useDatabases,
        useSchemas,
        useTables,
        type MetaDatabase,
        type MetaSchema,
        type MetaTable,
    } from "$lib/query/schemaQueries";
    import {
        IconDatabase,
        IconSchema,
        IconTable,
        IconColumns,
        IconChevronRight,
        IconChevronDown,
        IconLoader2,
    } from "@tabler/icons-svelte";

    // Props
    let {
        connectionId,
        expanded = $bindable(new Set<string>()),
        selected = $bindable<string | null>(null),
        onTableSelect,
    }: {
        connectionId: string | null;
        expanded?: Set<string>;
        selected?: string | null;
        onTableSelect?: (table: MetaTable) => void;
    } = $props();

    // Node type discriminator
    type NodeType = "database" | "schema" | "table" | "column";

    interface ExplorerNodeData {
        type: NodeType;
        name: string;
        database?: MetaDatabase;
        schema?: MetaSchema;
        table?: MetaTable;
        parentDb?: string;
        parentSchema?: string;
    }

    // Queries - using accessor pattern
    // TanStack Svelte Query returns a store-like object
    const databasesQuery = useDatabases(() => connectionId);

    // Track which nodes are being loaded
    let loadingNodes = $state(new Set<string>());

    // Build flat tree from hierarchical data
    // Access query result via .current in Svelte 5 with TanStack Query
    let treeNodes = $derived.by(() => {
        const nodes: TreeNode<ExplorerNodeData>[] = [];
        const queryResult = databasesQuery;
        const dbData = queryResult.data ?? [];

        for (const db of dbData) {
            const dbId = `db:${db.name}`;
            nodes.push({
                id: dbId,
                data: { type: "database", name: db.name, database: db },
                hasChildren: true,
                level: 0,
            });

            // If expanded, add schemas
            if (expanded.has(dbId)) {
                // Get schemas for this database
                const schemasForDb = getSchemas(db.name);
                for (const schema of schemasForDb) {
                    const schemaId = `schema:${db.name}:${schema.name}`;
                    nodes.push({
                        id: schemaId,
                        data: {
                            type: "schema",
                            name: schema.name,
                            schema,
                            parentDb: db.name,
                        },
                        hasChildren: true,
                        level: 1,
                    });

                    // If schema expanded, add tables
                    if (expanded.has(schemaId)) {
                        const tablesForSchema = getTables(db.name, schema.name);
                        for (const table of tablesForSchema) {
                            const tableId = `table:${db.name}:${schema.name}:${table.table_name}`;
                            nodes.push({
                                id: tableId,
                                data: {
                                    type: "table",
                                    name: table.table_name,
                                    table,
                                    parentDb: db.name,
                                    parentSchema: schema.name,
                                },
                                hasChildren: table.columns?.length > 0,
                                level: 2,
                            });
                        }
                    }
                }
            }
        }

        return nodes;
    });

    // Schema queries cache - keyed by database name
    let schemaQueries = $state(
        new Map<string, ReturnType<typeof useSchemas>>(),
    );

    function getSchemas(dbName: string): MetaSchema[] {
        if (!schemaQueries.has(dbName)) {
            const query = useSchemas(
                () => connectionId,
                () => dbName,
            );
            schemaQueries.set(dbName, query);
            schemaQueries = new Map(schemaQueries); // Trigger reactivity
        }
        const query = schemaQueries.get(dbName);
        return query?.data ?? [];
    }

    // Table queries cache - keyed by db:schema
    let tableQueries = $state(new Map<string, ReturnType<typeof useTables>>());

    function getTables(dbName: string, schemaName: string): MetaTable[] {
        const key = `${dbName}:${schemaName}`;
        if (!tableQueries.has(key)) {
            const query = useTables(
                () => connectionId,
                () => dbName,
                () => schemaName,
            );
            tableQueries.set(key, query);
            tableQueries = new Map(tableQueries);
        }
        const query = tableQueries.get(key);
        return query?.data ?? [];
    }

    // Handle expansion - trigger data loading
    function handleExpand(node: TreeNode<ExplorerNodeData>) {
        const { data } = node;

        if (data.type === "database") {
            // Expanding a database - schemas will be fetched via getSchemas
            loadingNodes.add(node.id);
            loadingNodes = new Set(loadingNodes);

            // Schemas should load automatically via the query
            setTimeout(() => {
                loadingNodes.delete(node.id);
                loadingNodes = new Set(loadingNodes);
            }, 100);
        }

        if (data.type === "schema" && data.parentDb) {
            // Expanding a schema - tables will be fetched via getTables
            loadingNodes.add(node.id);
            loadingNodes = new Set(loadingNodes);

            setTimeout(() => {
                loadingNodes.delete(node.id);
                loadingNodes = new Set(loadingNodes);
            }, 100);
        }
    }

    // Handle selection
    function handleSelect(node: TreeNode<ExplorerNodeData>) {
        if (node.data.type === "table" && node.data.table) {
            onTableSelect?.(node.data.table);
        }
    }

    // Get icon for node type
    function getIcon(type: NodeType) {
        switch (type) {
            case "database":
                return IconDatabase;
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

{#if !connectionId}
    <div class="p-4 text-muted-foreground text-sm">No connection selected</div>
{:else if databasesQuery.isLoading}
    <div class="p-4 flex items-center gap-2 text-muted-foreground text-sm">
        <IconLoader2 class="w-4 h-4 animate-spin" />
        Loading databases...
    </div>
{:else if databasesQuery.isError}
    <div class="p-4 text-destructive text-sm">
        Error: {databasesQuery.error?.message}
    </div>
{:else}
    <LazyTree
        nodes={treeNodes}
        {expanded}
        {selected}
        {loadingNodes}
        onExpand={handleExpand}
        onSelect={handleSelect}
        class="database-explorer"
    >
        {#snippet renderNode(ctx: NodeContext<ExplorerNodeData>)}
            {@const Icon = getIcon(ctx.node.data.type)}
            <div
                class="tree-node"
                class:selected={ctx.isSelected}
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
                        class="expand-button"
                        onclick={(e) => handleToggleClick(e, ctx.toggle)}
                        aria-label={ctx.isExpanded ? "Collapse" : "Expand"}
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
                    <span class="expand-spacer"></span>
                {/if}

                <!-- Icon -->
                <Icon class="w-4 h-4 shrink-0 text-muted-foreground" />

                <!-- Name -->
                <span class="node-name truncate">{ctx.node.data.name}</span>
            </div>
        {/snippet}
    </LazyTree>
{/if}

<style>
    .database-explorer {
        font-size: 13px;
    }

    .tree-node {
        display: flex;
        align-items: center;
        gap: 4px;
        padding: 4px 8px;
        cursor: pointer;
        border-radius: 4px;
        transition: background-color 0.1s;
    }

    .tree-node:hover {
        background-color: var(--accent);
    }

    .tree-node.selected {
        background-color: var(--accent);
        color: var(--accent-foreground);
    }

    .expand-button {
        display: flex;
        align-items: center;
        justify-content: center;
        width: 16px;
        height: 16px;
        padding: 0;
        background: none;
        border: none;
        cursor: pointer;
        opacity: 0.6;
        color: inherit;
    }

    .expand-button:hover {
        opacity: 1;
    }

    .expand-spacer {
        width: 16px;
        height: 16px;
    }

    .node-name {
        flex: 1;
        min-width: 0;
    }
</style>
