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

    const activeSession = $derived(windowState.activeSession);

    // Cache for lazily-loaded table details
    let tableDetailsCache = $state<Map<string, any>>(new Map());
    let loadingTables = $state<Set<string>>(new Set());

    // Progressive expand/collapse state
    // Level 0 = nothing expanded, 1 = folders only, 2 = tables, 3 = all
    let currentExpandLevel = $state(0);
    const MAX_EXPAND_LEVEL = 3;

    /**
     * Get all node IDs at a specific depth level from the tree
     */
    function getNodeIdsAtLevel(
        nodes: TreeNode[],
        targetLevel: number,
        currentLevel: number = 0,
    ): string[] {
        const ids: string[] = [];
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
     * Progressive expand - each click expands one more level
     */
    function progressiveExpand() {
        if (!activeSession) return;

        const nextLevel = Math.min(currentExpandLevel + 1, MAX_EXPAND_LEVEL);
        const expanded =
            activeSession.explorerState?.expanded || new Set<string>();
        const newExpanded = new Set(expanded);

        // Expand all nodes up to the next level
        for (let level = 0; level < nextLevel; level++) {
            const idsAtLevel = getNodeIdsAtLevel(treeData, level);
            for (const id of idsAtLevel) {
                newExpanded.add(id);
            }
        }

        if (activeSession.explorerState) {
            activeSession.explorerState.expanded = newExpanded;
        }
        currentExpandLevel = nextLevel;

        // If we've reached max, wrap around to 0 for next click
        if (currentExpandLevel >= MAX_EXPAND_LEVEL) {
            // On next click, fileTree.expandAll() won't do anything new
        }
    }

    /**
     * Progressive collapse - each click collapses one level (deepest first)
     */
    function progressiveCollapse() {
        if (!activeSession) return;

        const expanded =
            activeSession.explorerState?.expanded || new Set<string>();
        if (expanded.size === 0) {
            currentExpandLevel = 0;
            return;
        }

        // Find the deepest level that has expanded nodes
        let deepestLevel = 0;
        for (let level = MAX_EXPAND_LEVEL; level >= 0; level--) {
            const idsAtLevel = getNodeIdsAtLevel(treeData, level);
            if (idsAtLevel.some((id) => expanded.has(id))) {
                deepestLevel = level;
                break;
            }
        }

        // Remove all nodes at the deepest level
        const newExpanded = new Set(expanded);
        const idsToRemove = getNodeIdsAtLevel(treeData, deepestLevel);
        for (const id of idsToRemove) {
            newExpanded.delete(id);
        }

        if (activeSession.explorerState) {
            activeSession.explorerState.expanded = newExpanded;
        }
        currentExpandLevel = Math.max(0, deepestLevel);
    }

    // Ensure a session exists when schemaStore has an active connection
    $effect(() => {
        const conn = schemaStore.activeConnection;
        const hasSession = !!windowState.activeSession;

        if (conn && !hasSession && schemaStore.status === "idle") {
            windowState.startSession(conn);
        }
    });

    const treeData = $derived.by(() => {
        const activeConn = schemaStore.activeConnection;
        const selectedDbName = schemaStore.selectedDatabase;

        if (!activeConn || !selectedDbName) return [];

        const db = schemaStore.databases.find((d) => d.name === selectedDbName);
        if (!db) return [];

        // For SQLite, we flatten. We expect a 'main' schema, or just combine all.
        // Usually SQLite has one schema 'main'.
        const allTables = db.schemas.flatMap((s) =>
            s.tables.filter((t) => t.table_type === "table"),
        );
        const allViews = db.schemas.flatMap((s) =>
            s.tables.filter((t) => t.table_type === "view"),
        );

        // We need to know which schema a table belongs to for querying, even if we hide it in UI.
        // mapTableToNode handles this if we pass the correct schema name.

        const tableNodes = allTables.map((t) =>
            mapTableToNode(t, db.name, t.schema || "main"),
        );
        const viewNodes = allViews.map((t) => ({
            ...mapTableToNode(t, db.name, t.schema || "main"),
            detail: undefined, // Views don't have the same detail type usually
        }));

        const roots: TreeNode[] = [];

        if (tableNodes.length > 0) {
            roots.push({
                id: `folder:tables:${db.name}`,
                name: "Tables",
                type: "folder",
                count: tableNodes.length,
                children: tableNodes,
                // Keep expanded by default?
            });
        }

        if (viewNodes.length > 0) {
            roots.push({
                id: `folder:views:${db.name}`,
                name: "Views",
                type: "folder",
                count: viewNodes.length,
                children: viewNodes,
            });
        }

        return roots;
    });

    function mapTableToNode(table: any, dbName: string, schemaName: string) {
        const tableId = `table:${dbName}:${schemaName}.${table.table_name}`;
        const cacheKey = `${dbName}:${schemaName}:${table.table_name}`;
        const cachedDetails = tableDetailsCache.get(cacheKey);
        const isLoading = loadingTables.has(cacheKey);

        let children: TreeNode[] = [];

        if (cachedDetails) {
            children = [
                {
                    id: `cols:${tableId}`,
                    name: "Columns",
                    type: "group" as NodeType,
                    count: cachedDetails.columns?.length || 0,
                    children: (cachedDetails.columns || []).map((col: any) => {
                        // Build rich detail string with semantic hints
                        let detail = col.logical_type;

                        // Check for semantic hints from engine_type
                        const meta = col.engine_type?.metadata;
                        if (meta?.engine === "sqlite" && meta.meta) {
                            const hint = meta.meta.semantic_hint;
                            if (hint && hint.kind !== "none") {
                                // Show semantic hint as a badge, e.g., "text [UUID]"
                                detail = `${col.logical_type} [${hint.kind.toUpperCase()}]`;
                            }
                            if (meta.meta.is_generated) {
                                detail += " (gen)";
                            }
                            if (meta.meta.is_strict_table) {
                                detail += " ⚡"; // STRICT indicator
                            }
                        }

                        return {
                            id: `col:${tableId}.${col.column_name}`,
                            name: col.column_name,
                            type: (col.is_primary_key
                                ? "primary_key"
                                : "column") as NodeType,
                            detail,
                        };
                    }),
                },
                // Squeezing indexes/fks could be optionally hidden for "Simple" sqlite view?
                // Let's keep them for now.
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
            metadata: { dbName, schemaName, tableName: table.table_name },
        };
    }

    $effect(() => {
        const session = activeSession;
        const expanded = session?.explorerState?.expanded;
        if (!expanded || !schemaStore.activeConnection) return;

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
            const details = await invoke<any>("get_schema_table_details", {
                connectionId: schemaStore.activeConnection?.id,
                database: dbName,
                schema: schemaName,
                tableName: tableName,
            });
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

        if (node.type === "table" || node.type === "view") {
            // Open table preview with full metadata
            const metadata = node.metadata as
                | { dbName: string; schemaName: string; tableName: string }
                | undefined;
            activeSession.openView("table", node.name, {
                tableName: node.name,
                schemaName: metadata?.schemaName || "main",
                databaseName:
                    metadata?.dbName || schemaStore.selectedDatabase || "main",
            });
        } else if (
            node.type === "column" ||
            node.type === "primary_key" ||
            node.type === "foreign_key"
        ) {
            // For column clicks, open the parent table's data preview
            const parts = node.id?.split(":") || [];
            const tableIdPart = parts.find((p) => p.includes("."));
            if (tableIdPart) {
                const tableParts = tableIdPart.split(".");
                if (tableParts.length >= 2) {
                    const schemaName = tableParts[0];
                    const tableName = tableParts[1];
                    const dbName = schemaStore.selectedDatabase || "main";

                    activeSession.openView("table", tableName, {
                        tableName,
                        schemaName,
                        databaseName: dbName,
                    });
                    return;
                }
            }

            // Fallback: open query editor
            const dbSchemaTable = parts?.[parts.length - 1] || "";
            const tableRef = dbSchemaTable.split(".").slice(0, 2).join(".");
            activeSession.openView("editor", `Query: ${node.name}`, {
                initialValue: `SELECT * FROM ${tableRef} WHERE ${node.name} = ...`,
            });
        }
    }

    function handleContextMenuAction(action: string, node: TreeNode) {
        if (!activeSession) {
            if (schemaStore.activeConnection) {
                windowState.startSession(schemaStore.activeConnection);
            } else return;
        }
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
                console.log(`Action "${action}" not implemented`);
        }
    }

    async function handleNodeExpand(node: TreeNode, isOpen: boolean) {
        if (isOpen && node.type === "table" && node.metadata) {
            const { dbName, schemaName, tableName } = node.metadata as {
                dbName: string;
                schemaName: string;
                tableName: string;
            };
            loadTableDetails(dbName, schemaName, tableName);
        }

        if (activeSession) {
            activeSession.persistExpandedNodes();
        }
    }
</script>

<div class="flex h-full flex-col bg-muted/20">
    <div
        class="flex h-8 flex-none items-center border-b border-border bg-background/50 px-4"
    >
        <h2 class="text-sm font-semibold">Explorer</h2>
        <div class="ml-auto flex items-center gap-1">
            <button
                class="p-1 hover:bg-accent rounded-sm text-muted-foreground hover:text-foreground transition-colors"
                title="Expand Level"
                onclick={progressiveExpand}
            >
                <Expand />
            </button>
            <button
                class="p-1 hover:bg-accent rounded-sm text-muted-foreground hover:text-foreground transition-colors"
                title="Collapse Level"
                onclick={progressiveCollapse}
            >
                <Compact />
            </button>
            <button
                class="p-1 hover:bg-accent rounded-sm text-muted-foreground hover:text-foreground transition-colors"
                title="Refresh Schema"
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
                <p class="text-xs text-muted-foreground mb-4 max-w-[180px]">
                    No tables found.
                </p>
                <button
                    class="px-4 py-1.5 rounded-md bg-(--theme-bg-active) border border-(--theme-border-subtle) text-xs font-medium hover:bg-(--theme-bg-hover) transition-colors"
                    onclick={() => schemaStore.refresh()}
                >
                    Refresh
                </button>
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
