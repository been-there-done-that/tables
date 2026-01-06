<script lang="ts">
    import FileTree, {
        type NodeType,
        type TreeNode,
    } from "$lib/components/explorer/FileTree.svelte";
    import { windowState } from "$lib/stores/window.svelte";
    import { schemaStore } from "$lib/stores/schema.svelte";
    import { invoke } from "@tauri-apps/api/core";
    import IconLoader2 from "@tabler/icons-svelte/icons/loader-2";
    import { cn } from "$lib/utils";
    import IconRefresh from "@tabler/icons-svelte/icons/refresh";
    import IconDatabase from "@tabler/icons-svelte/icons/database";
    import Compact from "$lib/svg/Compact.svelte";
    import Expand from "$lib/svg/Expand.svelte";
    import PlaylistAdd from "@tabler/icons-svelte/icons/playlist-add";

    let fileTree = $state<any>(null);
    // Re-implementing the header locally for this component as per "independent components" plan.

    // Re-implementing the header locally for this component as per "independent components" plan.

    const activeSession = $derived(windowState.activeSession);

    // Cache for lazily-loaded table details
    let tableDetailsCache = $state<Map<string, any>>(new Map());
    let loadingTables = $state<Set<string>>(new Set());

    // Ensure a session exists when schemaStore has an active connection
    $effect(() => {
        const conn = schemaStore.activeConnection;
        const hasSession = !!windowState.activeSession;

        if (conn && !hasSession && schemaStore.status === "idle") {
            console.log(
                `[AutoSession] Creating session for connection ${conn.id}`,
            );
            windowState.startSession(conn);
        }
    });

    const treeData = $derived.by(() => {
        const activeConn = schemaStore.activeConnection;
        const selectedDbName = schemaStore.selectedDatabase;

        if (!activeConn || !selectedDbName) return [];

        const db = schemaStore.databases.find((d) => d.name === selectedDbName);
        if (!db) return [];

        // Map schemas directly to root nodes
        return db.schemas.map((schema) => {
            const tables = schema.tables.filter(
                (t) => t.table_type === "table",
            );
            const views = schema.tables.filter((t) => t.table_type === "view");

            const children: TreeNode[] = [];

            if (tables.length > 0) {
                children.push({
                    id: `folder:tables:${db.name}:${schema.name}`,
                    name: "tables",
                    type: "folder" as NodeType,
                    count: tables.length,
                    children: tables.map((table) =>
                        mapTableToNode(table, db.name, schema.name),
                    ),
                });
            }

            if (views.length > 0) {
                children.push({
                    id: `folder:views:${db.name}:${schema.name}`,
                    name: "views",
                    type: "folder" as NodeType,
                    count: views.length,
                    children: views.map((table) => ({
                        ...mapTableToNode(table, db.name, schema.name),
                        detail: undefined,
                    })),
                });
            }

            return {
                id: `schema:${db.name}:${schema.name}`,
                name: schema.name,
                type: "schema" as NodeType,
                children,
                isLoading: db.is_loading,
                metadata: { dbName: db.name, schemaName: schema.name },
            };
        });
    });

    function mapTableToNode(table: any, dbName: string, schemaName: string) {
        const tableId = `table:${dbName}:${schemaName}.${table.table_name}`;
        const cacheKey = `${dbName}:${schemaName}:${table.table_name}`;
        const cachedDetails = tableDetailsCache.get(cacheKey);
        const isLoading = loadingTables.has(cacheKey);

        // If details are cached, show them; otherwise show placeholder children
        let children: TreeNode[] = [];

        if (cachedDetails) {
            // Use cached details
            children = [
                {
                    id: `cols:${tableId}`,
                    name: "Columns",
                    type: "group" as NodeType,
                    count: cachedDetails.columns?.length || 0,
                    children: (cachedDetails.columns || []).map((col: any) => ({
                        id: `col:${tableId}.${col.column_name}`,
                        name: col.column_name,
                        type: (col.is_primary_key
                            ? "primary_key"
                            : "column") as NodeType,
                        detail: col.logical_type,
                    })),
                },
                {
                    id: `idxs:${tableId}`,
                    name: "Indexes",
                    type: "group" as NodeType,
                    count: cachedDetails.indexes?.length || 0,
                    children: (cachedDetails.indexes || []).map((idx: any) => ({
                        id: `idx:${tableId}.${idx.name}`,
                        name: idx.name,
                        type: "index" as NodeType,
                        detail: idx.is_unique ? "Unique" : "",
                    })),
                },
                {
                    id: `fks:${tableId}`,
                    name: "Foreign Keys",
                    type: "group" as NodeType,
                    count: cachedDetails.foreign_keys?.length || 0,
                    children: (cachedDetails.foreign_keys || []).map(
                        (fk: any) => ({
                            id: `fk:${tableId}.${fk.column_name}`,
                            name: fk.column_name,
                            type: "foreign_key" as NodeType,
                            detail: `-> ${fk.ref_table}.${fk.ref_column}`,
                        }),
                    ),
                },
            ];
        } else {
            // Show placeholder - will be replaced when expanded
            children = [
                {
                    id: `placeholder:${tableId}`,
                    name: isLoading ? "Loading..." : "Expand to load details",
                    type: "column" as NodeType,
                },
            ];
        }

        return {
            id: tableId,
            name: table.table_name,
            type: "table" as NodeType,
            detail: table.table_type === "table" ? undefined : table.table_type,
            children,
            // Store metadata for lazy loading
            metadata: { dbName, schemaName, tableName: table.table_name },
        };
    }

    // Effect to load details for pre-expanded tables
    $effect(() => {
        const session = activeSession;
        const expanded = session?.explorerState?.expanded;
        const hasConnection = !!schemaStore.activeConnection;

        if (!expanded || !schemaStore.activeConnection) return;

        // Find expanded table nodes and load their details
        for (const key of expanded) {
            if (key.startsWith("table:")) {
                const match = key.match(/^table:([^:]+):([^.]+)\.([^-]+)/);
                if (match) {
                    const [, dbName, schemaName, tableName] = match;
                    const cacheKey = `${dbName}:${schemaName}:${tableName}`;

                    if (
                        !tableDetailsCache.has(cacheKey) &&
                        !loadingTables.has(cacheKey)
                    ) {
                        loadTableDetails(dbName, schemaName, tableName);
                    }
                }
            }
        }
    });

    async function loadTableDetails(
        dbName: string,
        schemaName: string,
        tableName: string,
    ) {
        const cacheKey = `${dbName}:${schemaName}:${tableName}`;

        if (tableDetailsCache.has(cacheKey) || loadingTables.has(cacheKey))
            return;

        loadingTables = new Set([...loadingTables, cacheKey]);

        try {
            console.time(`[LazyLoad] ${tableName}`);
            const details = await invoke<any>("get_schema_table_details", {
                connectionId: schemaStore.activeConnection?.id,
                database: dbName,
                schema: schemaName,
                tableName: tableName,
            });
            console.timeEnd(`[LazyLoad] ${tableName}`);

            tableDetailsCache = new Map(tableDetailsCache).set(
                cacheKey,
                details,
            );
        } catch (e) {
            console.error(`Failed to load details for ${tableName}:`, e);
        } finally {
            loadingTables = new Set(
                [...loadingTables].filter((k) => k !== cacheKey),
            );
        }
    }

    function handleExplorerAction(node: TreeNode) {
        if (!activeSession) return;

        if (node.type === "table") {
            activeSession.openView("table", node.name, {
                tableName: node.name,
            });
        } else if (
            node.type === "column" ||
            node.type === "primary_key" ||
            node.type === "foreign_key"
        ) {
            // id format: col:table:db:schema.table.column
            const parts = node.id?.split(":");
            const dbSchemaTable = parts?.[parts.length - 1] || "";
            const tableRef = dbSchemaTable.split(".").slice(0, 2).join("."); // schema.table

            activeSession.openView("editor", `Query: ${node.name}`, {
                initialValue: `SELECT * FROM ${tableRef} WHERE ${node.name} = ...`,
            });
        }
    }

    function handleContextMenuAction(action: string, node: TreeNode) {
        if (!activeSession) {
            if (schemaStore.activeConnection) {
                windowState.startSession(schemaStore.activeConnection);
            } else {
                return;
            }
        }

        // Re-evaluate session after potentially starting it
        const session = windowState.activeSession;
        if (!session) return;

        switch (action) {
            case "query_console":
                const title =
                    node.type === "schema"
                        ? `Console: ${node.name}`
                        : `Query: ${node.name}`;
                session.openView("editor", title, node.metadata);
                break;
            default:
                console.log(
                    `[handleContextMenuAction] Action "${action}" not implemented for node ${node.name}`,
                );
        }
    }

    async function handleNodeExpand(node: TreeNode, isOpen: boolean) {
        if (isOpen && node.type === "database") {
            schemaStore.loadDatabase(node.name);
        }

        // Lazy load table details when table is expanded
        if (isOpen && node.type === "table" && node.metadata) {
            const { dbName, schemaName, tableName } = node.metadata as {
                dbName: string;
                schemaName: string;
                tableName: string;
            };
            loadTableDetails(dbName, schemaName, tableName);
        }
    }
</script>

<div class="flex h-full flex-col bg-muted/20">
    <div
        class="flex h-8 flex-none items-center border-b border-border bg-background/50 px-4"
    >
        <h2 class="text-sm font-semibold">Explorer (Postgres)</h2>
        <div class="ml-auto flex items-center gap-1">
            <button
                class="p-1 hover:bg-accent rounded-sm text-muted-foreground hover:text-foreground transition-colors"
                title="Expand All"
                onclick={() => fileTree?.expandAll()}
            >
                <Expand />
            </button>
            <button
                class="p-1 hover:bg-accent rounded-sm text-muted-foreground hover:text-foreground transition-colors"
                title="Collapse All"
                onclick={() => fileTree?.collapseAll()}
            >
                <Compact />
            </button>
            <button
                class="p-1 hover:bg-accent rounded-sm text-muted-foreground hover:text-foreground transition-colors"
                class:text-primary={windowState.layout.showSqlEditor}
                class:bg-accent={windowState.layout.showSqlEditor}
                title="Toggle SQL Playground"
                onclick={() =>
                    (windowState.layout.showSqlEditor =
                        !windowState.layout.showSqlEditor)}
            >
                <IconDatabase class="size-4" />
            </button>
            <button
                class="p-1 hover:bg-accent rounded-sm text-muted-foreground hover:text-foreground transition-colors"
                title={schemaStore.lastRefreshed
                    ? `Last refreshed: ${schemaStore.lastRefreshed.toLocaleTimeString()}`
                    : "Refresh Schema"}
                onclick={() => schemaStore.refresh()}
            >
                <IconRefresh
                    class={cn(
                        "size-4",
                        schemaStore.status === "refreshing" && "animate-spin",
                    )}
                />
            </button>
        </div>
    </div>

    <div class="flex-1 overflow-auto p-2 transition-all duration-300">
        {#if schemaStore.status === "connecting"}
            <div class="flex flex-col items-center justify-center h-40 gap-3">
                <IconLoader2
                    class="size-6 animate-spin text-(--theme-accent-primary)"
                />
                <p
                    class="text-[10px] text-muted-foreground animate-pulse font-medium"
                >
                    Connecting...
                </p>
            </div>
        {:else if schemaStore.databases.length === 0 && schemaStore.status !== "refreshing"}
            <div
                class="flex flex-col items-center justify-center p-8 text-center h-full max-h-[400px]"
            >
                <div class="mb-4 rounded-full bg-muted/30 p-4">
                    <IconDatabase
                        class="size-8 text-muted-foreground opacity-20"
                    />
                </div>
                {#if schemaStore.activeConnection}
                    <h3 class="text-sm font-medium mb-1">No Schemas Found</h3>
                    <p class="text-xs text-muted-foreground mb-4 max-w-[180px]">
                        Successfully connected to <b
                            >{schemaStore.activeConnection.name}</b
                        >, but no schemas were detected.
                    </p>
                    <button
                        class="px-4 py-1.5 rounded-md bg-(--theme-bg-active) border border-(--theme-border-subtle) text-xs font-medium hover:bg-(--theme-bg-hover) transition-colors"
                        onclick={() => schemaStore.refresh()}
                    >
                        Refresh Schema
                    </button>
                {:else}
                    <h3 class="text-sm font-medium mb-1">Explorer</h3>
                    <p class="text-xs text-muted-foreground mb-4 max-w-[180px]">
                        Select a database connection from the titlebar to browse
                        your data.
                    </p>
                    <div
                        class="flex items-center gap-2 text-[10px] text-primary bg-primary/5 px-2 py-1 rounded-full border border-primary/10"
                    >
                        <PlaylistAdd class="size-3" />
                        <span>Quick Select (Meta+P)</span>
                    </div>
                {/if}
            </div>
        {:else if activeSession}
            <div class="animate-in fade-in slide-in-from-left-2 duration-300">
                <FileTree
                    items={treeData}
                    bind:this={fileTree}
                    bind:expanded={activeSession.explorerState.expanded}
                    onAction={handleExplorerAction}
                    onContextMenuAction={handleContextMenuAction}
                    onExpand={handleNodeExpand}
                />
            </div>
        {:else}
            <div class="animate-in fade-in slide-in-from-left-2 duration-300">
                <FileTree
                    items={treeData}
                    bind:this={fileTree}
                    onAction={handleExplorerAction}
                    onContextMenuAction={handleContextMenuAction}
                    onExpand={handleNodeExpand}
                />
            </div>
        {/if}
    </div>
</div>
