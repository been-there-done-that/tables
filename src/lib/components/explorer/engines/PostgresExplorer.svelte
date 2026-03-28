<script lang="ts">
    import FileTree, {
        type NodeType,
        type TreeNode,
    } from "$lib/components/explorer/FileTree.svelte";
    import { windowState } from "$lib/stores/window.svelte";
    import { schemaStore } from "$lib/stores/schema.svelte";
    import { invoke } from "@tauri-apps/api/core";
    import IconLoader from "@tabler/icons-svelte/icons/loader";
    import { cn } from "$lib/utils";
    import IconDatabase from "@tabler/icons-svelte/icons/database";
    import PlaylistAdd from "@tabler/icons-svelte/icons/playlist-add";
    import ExplorerToolbar from "../ExplorerToolbar.svelte";
    import { getDefaultDatabase } from "$lib/engine-config";
    import { toast } from "svelte-sonner";

    let fileTree = $state<any>(null);
    let selectedNodeId = $state<string | null>(null);
    // Re-implementing the header locally for this component as per "independent components" plan.

    // Re-implementing the header locally for this component as per "independent components" plan.

    const activeSession = $derived(windowState.activeSession);

    // Cache for lazily-loaded table details
    let tableDetailsCache = $state<Map<string, any>>(new Map());
    let loadingTables = $state<Set<string>>(new Set());

    // Cache for schema-level objects (functions, sequences)
    let functionsCache = $state<Map<string, any[]>>(new Map()); // key: "dbName:schemaName"
    let sequencesCache = $state<Map<string, any[]>>(new Map());
    let loadingSchemaObjects = $state<Set<string>>(new Set()); // key: "dbName:schemaName:type"

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

        // Reset selection when connection changes
        if (!conn) {
            selectedNodeId = null;
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
                (t: any) => t.table_type === "table" || t.table_type === "BASE TABLE",
            );
            const views = schema.tables.filter((t: any) => t.table_type === "view");
            const matviews = schema.tables.filter((t: any) => t.table_type === "materialized_view" || t.table_type === "MATERIALIZED VIEW");

            const cacheKey = `${db.name}:${schema.name}`;
            const allFunctions = functionsCache.get(cacheKey) || [];
            const functions = allFunctions.filter((f: any) => f.kind !== "Procedure");
            const procedures = allFunctions.filter((f: any) => f.kind === "Procedure");
            const sequences = sequencesCache.get(cacheKey) || [];

            const children: TreeNode[] = [];

            if (tables.length > 0) {
                children.push({
                    id: `folder:tables:${db.name}:${schema.name}`,
                    name: "tables",
                    type: "folder" as NodeType,
                    count: tables.length,
                    children: tables.map((table: any) =>
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
                    children: views.map((v: any) => ({
                        id: `view:${db.name}:${schema.name}.${v.table_name}`,
                        name: v.table_name,
                        type: "view" as NodeType,
                        metadata: { dbName: db.name, schemaName: schema.name, objectName: v.table_name, objectType: "view" },
                    })),
                });
            }

            if (matviews.length > 0) {
                children.push({
                    id: `folder:matviews:${db.name}:${schema.name}`,
                    name: "materialized views",
                    type: "folder" as NodeType,
                    count: matviews.length,
                    children: matviews.map((v: any) => ({
                        id: `matview:${db.name}:${schema.name}.${v.table_name}`,
                        name: v.table_name,
                        type: "materialized_view" as NodeType,
                        metadata: { dbName: db.name, schemaName: schema.name, objectName: v.table_name, objectType: "matview" },
                    })),
                });
            }

            // Functions folder — lazy loaded when expanded
            children.push({
                id: `folder:functions:${db.name}:${schema.name}`,
                name: "functions",
                type: "folder" as NodeType,
                count: functions.length > 0 ? functions.length : undefined,
                children: functions.map((f: any) => ({
                    id: `function:${db.name}:${schema.name}.${f.name}`,
                    name: f.name,
                    type: "function" as NodeType,
                    detail: f.return_type || undefined,
                    metadata: { dbName: db.name, schemaName: schema.name, objectName: f.name, objectType: "function", language: f.language },
                })),
            });

            // Procedures folder — lazy loaded when expanded (shares functions cache)
            children.push({
                id: `folder:procedures:${db.name}:${schema.name}`,
                name: "procedures",
                type: "folder" as NodeType,
                count: procedures.length > 0 ? procedures.length : undefined,
                children: procedures.map((f: any) => ({
                    id: `procedure:${db.name}:${schema.name}.${f.name}`,
                    name: f.name,
                    type: "procedure" as NodeType,
                    detail: undefined,
                    metadata: { dbName: db.name, schemaName: schema.name, objectName: f.name, objectType: "procedure", language: f.language },
                })),
            });

            // Sequences folder — lazy loaded when expanded
            children.push({
                id: `folder:sequences:${db.name}:${schema.name}`,
                name: "sequences",
                type: "folder" as NodeType,
                count: sequences.length > 0 ? sequences.length : undefined,
                children: sequences.map((s: any) => ({
                    id: `sequence:${db.name}:${schema.name}.${s.name}`,
                    name: s.name,
                    type: "sequence" as NodeType,
                    detail: s.data_type,
                    metadata: { dbName: db.name, schemaName: schema.name, objectName: s.name, objectType: "sequence" },
                })),
            });

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
                        detail: [idx.index_type, idx.is_unique ? "unique" : ""].filter(Boolean).join(" · "),
                        metadata: { dbName, schemaName, tableName: table.table_name },
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
                {
                    id: `constraints:${tableId}`,
                    name: "Constraints",
                    type: "group" as NodeType,
                    count: cachedDetails.constraints?.length || 0,
                    children: (cachedDetails.constraints || []).map((c: any) => ({
                        id: `constraint:${tableId}.${c.name}`,
                        name: c.name,
                        type: "constraint" as NodeType,
                        detail: c.kind,
                        metadata: { dbName, schemaName, tableName: table.table_name, constraintName: c.name, definition: c.definition },
                    })),
                },
                {
                    id: `trgs:${tableId}`,
                    name: "Triggers",
                    type: "group" as NodeType,
                    count: cachedDetails.triggers?.length || 0,
                    children: (cachedDetails.triggers || []).map((t: any) => ({
                        id: `trigger:${tableId}.${t.trigger_name}`,
                        name: t.trigger_name,
                        type: "trigger" as NodeType,
                        detail: `${t.timing} ${t.event}`,
                        metadata: { dbName, schemaName, tableName: table.table_name },
                    })),
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
            const [details, constraints] = await Promise.all([
                invoke<any>("get_schema_table_details", {
                    connectionId: schemaStore.activeConnection?.id,
                    database: dbName,
                    schema: schemaName,
                    tableName: tableName,
                }),
                invoke<any[]>("get_constraints", {
                    connectionId: schemaStore.activeConnection?.id,
                    database: dbName,
                    schema: schemaName,
                    tableName: tableName,
                }).catch(() => []),
            ]);
            console.timeEnd(`[LazyLoad] ${tableName}`);

            tableDetailsCache = new Map(tableDetailsCache).set(
                cacheKey,
                { ...details, constraints },
            );
        } catch (e) {
            console.error(`Failed to load details for ${tableName}:`, e);
        } finally {
            loadingTables = new Set(
                [...loadingTables].filter((k) => k !== cacheKey),
            );
        }
    }

    async function loadFunctions(dbName: string, schemaName: string) {
        const cacheKey = `${dbName}:${schemaName}`;
        const loadKey = `${cacheKey}:functions`;
        if (functionsCache.has(cacheKey) || loadingSchemaObjects.has(loadKey)) return;

        loadingSchemaObjects = new Set([...loadingSchemaObjects, loadKey]);
        try {
            const fns = await invoke<any[]>("get_functions", {
                connectionId: schemaStore.activeConnection?.id,
                database: dbName,
                schema: schemaName,
            });
            functionsCache = new Map(functionsCache).set(cacheKey, fns);
        } catch (e) {
            console.error(`Failed to load functions for ${schemaName}:`, e);
        } finally {
            loadingSchemaObjects = new Set([...loadingSchemaObjects].filter(k => k !== loadKey));
        }
    }

    async function loadSequences(dbName: string, schemaName: string) {
        const cacheKey = `${dbName}:${schemaName}`;
        const loadKey = `${cacheKey}:sequences`;
        if (sequencesCache.has(cacheKey) || loadingSchemaObjects.has(loadKey)) return;

        loadingSchemaObjects = new Set([...loadingSchemaObjects, loadKey]);
        try {
            const seqs = await invoke<any[]>("get_sequences", {
                connectionId: schemaStore.activeConnection?.id,
                database: dbName,
                schema: schemaName,
            });
            sequencesCache = new Map(sequencesCache).set(cacheKey, seqs);
        } catch (e) {
            console.error(`Failed to load sequences for ${schemaName}:`, e);
        } finally {
            loadingSchemaObjects = new Set([...loadingSchemaObjects].filter(k => k !== loadKey));
        }
    }

    function handleExplorerAction(node: TreeNode) {
        if (!activeSession) return;

        if (node.type === "table" || node.type === "view") {
            // Open table preview with full metadata
            const metadata = node.metadata as
                | { dbName: string; schemaName: string; tableName: string }
                | undefined;
            activeSession.openView("table", node.name, {
                tableName: node.name,
                schemaName: metadata?.schemaName,
                databaseName: metadata?.dbName,
                connectionId: schemaStore.activeConnection?.id,
            });
        } else if (
            node.type === "column" ||
            node.type === "primary_key" ||
            node.type === "foreign_key"
        ) {
            // For column clicks, open the parent table's data preview
            // id format: col:table:db:schema.table.column or similar
            const parts = node.id?.split(":") || [];
            // Try to extract table info from the id - format varies
            // e.g., "col:table:mydb:public.users.id"
            const tableIdPart = parts.find((p) => p.includes("."));
            if (tableIdPart) {
                const tableParts = tableIdPart.split(".");
                if (tableParts.length >= 2) {
                    const schemaName = tableParts[0];
                    const tableName = tableParts[1];
                    const dbName =
                        schemaStore.selectedDatabase ||
                        getDefaultDatabase("postgres");

                    activeSession.openView("table", tableName, {
                        tableName,
                        schemaName,
                        databaseName: dbName,
                        connectionId: schemaStore.activeConnection?.id,
                    });
                    return;
                }
            }

            // Fallback: open query editor if we can't parse table info
            const dbSchemaTable = parts?.[parts.length - 1] || "";
            const tableRef = dbSchemaTable.split(".").slice(0, 2).join(".");
            activeSession.openView("editor", `Query: ${node.name}`, {
                initialValue: `SELECT * FROM ${tableRef} WHERE ${node.name} = ...`,
            });
        }
    }

    async function handleContextMenuAction(action: string, node: TreeNode) {
        if (!activeSession) {
            if (schemaStore.activeConnection) windowState.startSession(schemaStore.activeConnection);
            else return;
        }
        const session = windowState.activeSession;
        if (!session) return;

        const meta = node.metadata as any;
        const connId = schemaStore.activeConnection?.id;
        const db = meta?.dbName || schemaStore.selectedDatabase;
        const schema = meta?.schemaName || "public";

        switch (action) {
            case "query_console": {
                const title = node.type === "schema" ? `Console: ${node.name}` : `Query: ${node.name}`;
                session.openView("editor", title, node.metadata);
                break;
            }

            case "copy_name": {
                await navigator.clipboard.writeText(node.name);
                break;
            }

            case "view_data": {
                session.openView("table", node.name, {
                    tableName: node.name,
                    schemaName: schema,
                    databaseName: db,
                    connectionId: connId,
                });
                break;
            }

            case "copy_select": {
                await navigator.clipboard.writeText(`SELECT * FROM "${schema}"."${node.name}";`);
                break;
            }

            case "open_ddl": {
                try {
                    let ddl = "";
                    if (node.type === "table") {
                        ddl = await invoke<string>("get_table_ddl", {
                            connectionId: connId,
                            database: db,
                            schema,
                            tableName: node.name,
                        });
                    } else if (node.type === "sequence") {
                        ddl = await invoke<string>("get_sequence_ddl", {
                            connectionId: connId,
                            database: db,
                            schema,
                            sequenceName: node.name,
                        });
                    }
                    session.openDdlTab(`DDL: ${schema}.${node.name}`, ddl);
                } catch (e) {
                    console.error("DDL fetch failed:", e);
                    toast.error(`Failed to load ${node.name}`, { description: String(e) });
                }
                break;
            }

            case "open_definition": {
                try {
                    let ddl = "";
                    if (node.type === "view") {
                        ddl = await invoke<string>("get_view_definition", {
                            connectionId: connId,
                            database: db,
                            schema,
                            viewName: node.name,
                        });
                    } else if (node.type === "materialized_view") {
                        ddl = await invoke<string>("get_matview_definition", {
                            connectionId: connId,
                            database: db,
                            schema,
                            viewName: node.name,
                        });
                    } else if (node.type === "function" || node.type === "procedure") {
                        ddl = await invoke<string>("get_function_ddl", {
                            connectionId: connId,
                            database: db,
                            schema,
                            functionName: node.name,
                        });
                    } else if (node.type === "index") {
                        ddl = await invoke<string>("get_index_ddl", {
                            connectionId: connId,
                            database: db,
                            schema,
                            tableName: meta?.tableName || "",
                            indexName: node.name,
                        });
                    } else if (node.type === "trigger") {
                        ddl = await invoke<string>("get_trigger_definition", {
                            connectionId: connId,
                            database: db,
                            schema,
                            tableName: meta?.tableName || "",
                            triggerName: node.name,
                        });
                    }
                    session.openDdlTab(`DDL: ${schema}.${node.name}`, ddl);
                } catch (e) {
                    console.error("Definition fetch failed:", e);
                    toast.error(`Failed to load ${node.name}`, { description: String(e) });
                }
                break;
            }

            case "copy_function_ddl":
            case "copy_sequence_ddl":
            case "copy_index_ddl":
            case "copy_trigger_ddl": {
                try {
                    let ddl = "";
                    if (node.type === "function" || node.type === "procedure") {
                        ddl = await invoke<string>("get_function_ddl", {
                            connectionId: connId,
                            database: db,
                            schema,
                            functionName: node.name,
                        });
                    } else if (node.type === "sequence") {
                        ddl = await invoke<string>("get_sequence_ddl", {
                            connectionId: connId,
                            database: db,
                            schema,
                            sequenceName: node.name,
                        });
                    } else if (node.type === "index") {
                        ddl = await invoke<string>("get_index_ddl", {
                            connectionId: connId,
                            database: db,
                            schema,
                            tableName: meta?.tableName || "",
                            indexName: node.name,
                        });
                    } else if (node.type === "trigger") {
                        ddl = await invoke<string>("get_trigger_definition", {
                            connectionId: connId,
                            database: db,
                            schema,
                            tableName: meta?.tableName || "",
                            triggerName: node.name,
                        });
                    }
                    await navigator.clipboard.writeText(ddl);
                } catch (e) {
                    console.error("Copy DDL failed:", e);
                    toast.error(`Failed to load ${node.name}`, { description: String(e) });
                }
                break;
            }

            case "copy_constraint_ddl": {
                const def = meta?.definition || "";
                await navigator.clipboard.writeText(`CONSTRAINT "${node.name}" ${def}`);
                break;
            }

            case "open_schema_ddl": {
                const schemaName = meta?.schemaName || node.name;
                session.openDdlTab(`DDL: ${schemaName}`, `CREATE SCHEMA "${schemaName}";`);
                break;
            }

            case "refresh_schema": {
                await schemaStore.refresh();
                tableDetailsCache = new Map();
                functionsCache = new Map();
                sequencesCache = new Map();
                break;
            }

            default:
                console.log(`[handleContextMenuAction] Unhandled action: ${action}`);
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

        // NEW: lazy load schema-level objects when folder expanded
        if (isOpen && node.type === "folder" && node.id) {
            if (node.id.startsWith("folder:functions:") || node.id.startsWith("folder:procedures:")) {
                const parts = node.id.split(":");
                // id format: folder:functions:dbName:schemaName  or  folder:procedures:dbName:schemaName
                const dbName = parts[2];
                const schemaName = parts.slice(3).join(":"); // handle schema names with colons
                loadFunctions(dbName, schemaName);
            } else if (node.id.startsWith("folder:sequences:")) {
                const parts = node.id.split(":");
                const dbName = parts[2];
                const schemaName = parts.slice(3).join(":");
                loadSequences(dbName, schemaName);
            }
        }

        if (activeSession) {
            activeSession.persistExpandedNodes();
        }
    }
</script>

<div class="flex h-full flex-col bg-muted/20">
    <ExplorerToolbar {treeData} {selectedNodeId} />

    <div class="flex-1 overflow-auto p-2">
        {#if schemaStore.status === "connecting"}
            <div class="flex flex-col items-center justify-center h-40 gap-3">
                <IconLoader
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
                    bind:selectedNodeId
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
                    bind:selectedNodeId
                    onAction={handleExplorerAction}
                    onContextMenuAction={handleContextMenuAction}
                    onExpand={handleNodeExpand}
                />
            </div>
        {/if}
    </div>
</div>

<style>
    /* Auto-hide scrollbar: transparent by default, visible on hover */
    .explorer-scroll::-webkit-scrollbar-thumb {
        background-color: transparent;
        border-color: transparent;
    }

    .explorer-scroll:hover::-webkit-scrollbar-thumb {
        background-color: var(--theme-border-default);
    }
</style>
