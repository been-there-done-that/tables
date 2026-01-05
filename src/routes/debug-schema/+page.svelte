<script lang="ts">
    import { invoke } from "@tauri-apps/api/core";
    import { listen } from "@tauri-apps/api/event";
    import { onMount, type Snippet } from "svelte";
    import FileTree, {
        type TreeNode,
    } from "$lib/components/explorer/FileTree.svelte";

    interface Connection {
        id: string;
        name: string;
        engine: string;
    }

    interface MetaTable {
        connection_id: string;
        schema: string;
        table_name: string;
        table_type: string;
        classification: string;
        last_introspected_at: number;
    }

    let connections = $state<Connection[]>([]);
    let selectedId = $state<string | null>(null);
    let tables = $state<MetaTable[]>([]);
    let status = $state<"idle" | "busy" | "success" | "error">("idle");
    let error = $state<string | null>(null);
    let selectedTableDetails = $state<any | null>(null);

    onMount(async () => {
        try {
            connections = await invoke("list_connections");
        } catch (e) {
            console.error("Failed to load connections:", e);
        }
    });

    // Load cache immediately when connection changes
    $effect(() => {
        if (selectedId) {
            loadCachedTables(selectedId);
        } else {
            tables = [];
        }
    });

    async function loadCachedTables(id: string) {
        try {
            status = "busy";
            tables = await invoke("get_cached_all_tables", {
                connectionId: id,
            });
            status = "success";
        } catch (e) {
            console.error("Failed to load cache:", e);
            status = "error";
        }
    }

    async function refreshSchema() {
        if (!selectedId) return;
        status = "busy";
        // Clear tables to show fresh stream? Or keep them and update?
        // User request implied stream 'as soon as we start finding them'.
        // To avoid duplicates, let's clear or upsert.
        // For visual clarity of "freshness", let's clear.
        tables = [];

        try {
            await invoke("refresh_schema_unified", {
                connectionId: selectedId,
                options: { scope: { type: "global" } },
            });
            // Introspection complete, but we were listening to events mostly.
            // A final fetch ensures we have everything if events failed?
            // But let's rely on events + final fetch.
            await loadCachedTables(selectedId);
            status = "success";
        } catch (e) {
            console.error("Failed to introspect:", e);
            status = "error";
            error = String(e);
        }
    }

    onMount(() => {
        const unlisten = listen<MetaTable>(
            "introspection://table-found",
            (event) => {
                const newTable = event.payload;
                if (newTable.connection_id === selectedId) {
                    // Upsert logic to prevent duplicates if any
                    const idx = tables.findIndex(
                        (t) =>
                            t.table_name === newTable.table_name &&
                            t.schema === newTable.schema,
                    );
                    if (idx >= 0) {
                        tables[idx] = newTable;
                    } else {
                        tables = [...tables, newTable];
                    }
                }
            },
        );

        // Cleanup
        return () => {
            unlisten.then((f) => f());
        };
    });

    async function viewTableDetails(tableName: string, schema: string) {
        if (!selectedId) return;
        try {
            selectedTableDetails = await invoke("get_cached_table_details", {
                connectionId: selectedId,
                database: "main", // Default or extract from table object
                schema,
                tableName,
            });
        } catch (e) {
            console.error("Failed to load table details:", e);
        }
    }

    // Transform flat tables list into tree structure
    let treeData = $derived.by(() => {
        if (!selectedId || tables.length === 0) return [];

        const conn = connections.find((c) => c.id === selectedId);
        const connectionName = conn?.name || "Database";

        // Group by schema
        const schemaMap = new Map<string, MetaTable[]>();
        for (const t of tables) {
            const schema = t.schema || "main";
            if (!schemaMap.has(schema)) {
                schemaMap.set(schema, []);
            }
            schemaMap.get(schema)?.push(t);
        }

        const schemaNodes: TreeNode[] = [];

        for (const [schemaName, schemaTables] of schemaMap) {
            // Sort tables by name
            schemaTables.sort((a, b) =>
                a.table_name.localeCompare(b.table_name),
            );

            const tableNodes: TreeNode[] = schemaTables.map((t) => {
                const isSelected =
                    selectedTableDetails?.table_name === t.table_name;
                const children: TreeNode[] = [];
                const tableId = `conn-${selectedId}-schema-${schemaName}-table-${t.table_name}`;

                if (isSelected && selectedTableDetails) {
                    // Add Columns
                    if (selectedTableDetails.columns?.length) {
                        children.push({
                            id: `${tableId}-columns`,
                            name: "columns",
                            type: "folder",
                            children: selectedTableDetails.columns.map(
                                (c: any) => ({
                                    id: `${tableId}-col-${c.column_name}`,
                                    name: c.column_name,
                                    type: c.is_primary_key
                                        ? "primary_key"
                                        : "column",
                                    detail: `${c.logical_type}`,
                                }),
                            ),
                        });
                    }
                    // Add Indexes
                    if (selectedTableDetails.indexes?.length) {
                        children.push({
                            id: `${tableId}-indexes`,
                            name: "indexes",
                            type: "folder",
                            children: selectedTableDetails.indexes.map(
                                (idx: any) => ({
                                    id: `${tableId}-idx-${idx.name}`,
                                    name: idx.name,
                                    type: "index",
                                    detail: idx.is_unique ? "UNIQUE" : "",
                                }),
                            ),
                        });
                    }
                    // Add Foreign Keys
                    if (selectedTableDetails.foreign_keys?.length) {
                        children.push({
                            id: `${tableId}-fks`,
                            name: "foreign keys",
                            type: "folder",
                            children: selectedTableDetails.foreign_keys.map(
                                (fk: any) => ({
                                    id: `${tableId}-fk-${fk.column_name}`,
                                    name: fk.column_name,
                                    type: "key",
                                    detail: `-> ${fk.ref_table}`,
                                }),
                            ),
                        });
                    }
                }

                return {
                    id: tableId,
                    name: t.table_name,
                    type: "table",
                    detail: t.table_type,
                    children: children.length ? children : undefined, // Leaf if no details loaded
                };
            });

            schemaNodes.push({
                id: `conn-${selectedId}-schema-${schemaName}`,
                name: schemaName,
                type: "schema",
                children: [
                    {
                        id: `conn-${selectedId}-schema-${schemaName}-tables`,
                        name: "tables",
                        type: "folder",
                        children: tableNodes,
                    },
                ],
            });
        }

        // Sort schemas (main first, then alphabetical)
        schemaNodes.sort((a, b) => {
            if (a.name === "main") return -1;
            if (b.name === "main") return 1;
            return a.name.localeCompare(b.name);
        });

        return [
            {
                id: `conn-${selectedId}-root`,
                name: connectionName,
                type: "database",
                children: schemaNodes,
            },
        ] as TreeNode[];
    });

    const handleNodeClick = (node: TreeNode) => {
        if (node.type === "table") {
            const table = tables.find(
                (t) =>
                    `conn-${selectedId}-schema-${t.schema || "main"}-table-${t.table_name}` ===
                    node.id,
            );
            if (table) {
                viewTableDetails(node.name, table.schema || "main");
            }
        }
    };
</script>

<div class="h-screen flex flex-col bg-background overflow-hidden">
    <header class="p-6 border-b border-border shrink-0">
        <h1 class="text-2xl font-bold text-foreground">
            Schema Introspection Debug
        </h1>
        <p class="text-muted-foreground text-sm">
            List data sources and trigger normalized schema caching.
        </p>
    </header>

    <div class="flex-1 grid grid-cols-1 md:grid-cols-3 gap-0 overflow-hidden">
        <!-- Connections List -->
        <section
            class="h-full overflow-y-auto p-6 border-r border-border bg-card/10"
        >
            <h2
                class="text-xs font-bold uppercase tracking-widest text-muted-foreground mb-4"
            >
                Data Sources
            </h2>
            <div class="space-y-2">
                {#each connections as conn}
                    <div
                        class="p-4 border rounded-lg bg-card hover:border-accent transition-colors flex justify-between items-center {selectedId ===
                        conn.id
                            ? 'border-accent ring-1 ring-accent'
                            : 'border-border'}"
                    >
                        <div>
                            <div class="font-medium text-foreground">
                                {conn.name}
                            </div>
                            <div
                                class="text-xs text-muted-foreground uppercase"
                            >
                                {conn.engine}
                            </div>
                        </div>
                        <button
                            class="px-3 py-1.5 bg-accent text-white rounded text-xs font-bold hover:bg-accent/80 disabled:opacity-50 transition-all"
                            disabled={status === "busy"}
                            onclick={() => {
                                selectedId = conn.id; // Set selectedId first
                                refreshSchema(); // Then refresh
                            }}
                        >
                            {status === "busy" && selectedId === conn.id
                                ? "RUNNING..."
                                : "REFRESH SCHEMA"}
                        </button>
                    </div>
                {/each}
                {#if connections.length === 0}
                    <div
                        class="p-8 border border-dashed rounded-lg text-center text-muted-foreground text-sm"
                    >
                        No connections found.
                    </div>
                {/if}
            </div>
        </section>

        <!-- Status & Tables -->
        <section
            class="md:col-span-2 h-full overflow-y-auto p-8 space-y-6 bg-background"
        >
            {#if status === "busy"}
                <div
                    class="p-12 border border-dashed rounded-xl flex flex-col items-center justify-center space-y-4 animate-pulse"
                >
                    <div
                        class="w-8 h-8 rounded-full border-2 border-accent border-t-transparent animate-spin"
                    ></div>
                    <div class="text-sm font-medium">
                        Introspecting engine...
                    </div>
                </div>
            {:else if status === "error"}
                <div
                    class="p-4 bg-destructive/10 border border-destructive/20 rounded-lg text-destructive text-sm"
                >
                    <strong>Error:</strong>
                    {error}
                </div>
            {/if}

            {#if tables.length > 0}
                <div class="space-y-4 h-full flex flex-col">
                    <div class="flex justify-between items-end shrink-0">
                        <h2
                            class="text-sm font-semibold uppercase tracking-wider text-muted-foreground"
                        >
                            Schema Explorer
                        </h2>
                        <span
                            class="text-[10px] text-muted-foreground px-2 py-0.5 bg-muted rounded-full"
                        >
                            {tables.length}
                            {tables.length === 1 ? "Object" : "Objects"} FOUND
                        </span>
                    </div>

                    <div
                        class="border rounded-xl bg-card/30 flex-1 overflow-hidden p-2"
                    >
                        <FileTree
                            items={treeData}
                            onNodeClick={handleNodeClick}
                            indent={16}
                        />
                    </div>
                </div>
            {:else if status === "success" && selectedId}
                <div
                    class="p-12 border border-dashed rounded-xl flex flex-col items-center justify-center space-y-4 text-center"
                >
                    <div
                        class="w-12 h-12 rounded-full bg-muted flex items-center justify-center text-muted-foreground"
                    >
                        <svg
                            xmlns="http://www.w3.org/2000/svg"
                            width="24"
                            height="24"
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="2"
                            stroke-linecap="round"
                            stroke-linejoin="round"
                            ><path
                                d="M14.5 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7.5L14.5 2z"
                            /><polyline points="14 2 14 8 20 8" /><path
                                d="M9 15h6"
                            /><path d="M9 12h6" /></svg
                        >
                    </div>
                    <div>
                        <div class="font-medium text-foreground">
                            No schema data found
                        </div>
                        <p class="text-sm text-muted-foreground">
                            The database might be empty or contains no readable
                            tables/views.
                        </p>
                    </div>
                </div>
            {:else if !selectedId}
                <div
                    class="p-12 border border-dashed rounded-xl flex flex-col items-center justify-center space-y-2 text-muted-foreground"
                >
                    <p class="text-sm font-medium">
                        Select a data source to begin introspection
                    </p>
                    <p class="text-[11px] opacity-60">
                        Metadata will be cached in the backend for rapid access.
                    </p>
                </div>
            {/if}

            <!-- Table Details Explorer -->
            {#if selectedTableDetails}
                <div
                    class="mt-8 space-y-8 animate-in fade-in slide-in-from-bottom-4 duration-300"
                >
                    <header
                        class="flex justify-between items-center border-b border-border pb-2"
                    >
                        <div class="flex items-center space-x-3">
                            <h2 class="text-lg font-bold text-foreground">
                                {selectedTableDetails.table_name}
                            </h2>
                            <span
                                class="text-[10px] px-2 py-0.5 bg-emerald-500/10 text-emerald-500 rounded font-mono border border-emerald-500/20 uppercase tracking-tighter"
                                >Normalised</span
                            >
                        </div>
                        <button
                            class="text-xs text-muted-foreground hover:text-foreground hover:underline px-2 py-1 transition-all"
                            onclick={() => (selectedTableDetails = null)}
                            >CLOSE EXPLORER</button
                        >
                    </header>

                    <div class="grid grid-cols-1 gap-8">
                        <!-- Columns Section -->
                        <section class="space-y-4">
                            <h3
                                class="text-xs font-bold uppercase tracking-widest text-muted-foreground/80 flex items-center space-x-2"
                            >
                                <svg
                                    xmlns="http://www.w3.org/2000/svg"
                                    width="14"
                                    height="14"
                                    viewBox="0 0 24 24"
                                    fill="none"
                                    stroke="currentColor"
                                    stroke-width="2"
                                    stroke-linecap="round"
                                    stroke-linejoin="round"
                                    ><rect
                                        width="18"
                                        height="18"
                                        x="3"
                                        y="3"
                                        rx="2"
                                    /><path d="M7 3v18" /><path
                                        d="M12 3v18"
                                    /><path d="M17 3v18" /><path
                                        d="M3 7h18"
                                    /><path d="M3 12h18" /><path
                                        d="M3 17h18"
                                    /></svg
                                >
                                <span>Columns</span>
                            </h3>
                            <div
                                class="border border-border rounded-xl overflow-hidden bg-card/50 backdrop-blur-sm shadow-sm"
                            >
                                <table
                                    class="w-full text-left text-xs border-collapse"
                                >
                                    <thead>
                                        <tr
                                            class="bg-muted/50 border-b border-border"
                                        >
                                            <th
                                                class="px-4 py-3 font-semibold text-muted-foreground"
                                                >#</th
                                            >
                                            <th
                                                class="px-4 py-3 font-semibold text-muted-foreground"
                                                >NAME</th
                                            >
                                            <th
                                                class="px-4 py-3 font-semibold text-muted-foreground"
                                                >LOGICAL TYPE</th
                                            >
                                            <th
                                                class="px-4 py-3 font-semibold text-muted-foreground"
                                                >RAW TYPE</th
                                            >
                                            <th
                                                class="px-4 py-3 font-semibold text-muted-foreground text-center"
                                                >PK</th
                                            >
                                            <th
                                                class="px-4 py-3 font-semibold text-muted-foreground text-center"
                                                >NULL</th
                                            >
                                            <th
                                                class="px-4 py-3 font-semibold text-muted-foreground"
                                                >DEFAULT</th
                                            >
                                        </tr>
                                    </thead>
                                    <tbody class="divide-y divide-border/50">
                                        {#each selectedTableDetails.columns as col}
                                            <tr
                                                class="hover:bg-accent/5 transition-colors"
                                            >
                                                <td
                                                    class="px-4 py-3 font-mono text-muted-foreground/60"
                                                    >{col.ordinal_position}</td
                                                >
                                                <td
                                                    class="px-4 py-3 font-bold text-foreground"
                                                    >{col.column_name}</td
                                                >
                                                <td class="px-4 py-3">
                                                    <span
                                                        class="px-2 py-0.5 rounded-full bg-accent/10 text-accent font-medium"
                                                        >{col.logical_type}</span
                                                    >
                                                </td>
                                                <td
                                                    class="px-4 py-3 font-mono text-muted-foreground italic text-[10px]"
                                                    >{col.raw_type || "N/A"}</td
                                                >
                                                <td
                                                    class="px-4 py-3 text-center"
                                                >
                                                    {#if col.is_primary_key}
                                                        <span
                                                            class="text-amber-500 font-bold"
                                                            title="Primary Key"
                                                            >🔑</span
                                                        >
                                                    {:else}
                                                        <span
                                                            class="text-muted-foreground opacity-20"
                                                            >—</span
                                                        >
                                                    {/if}
                                                </td>
                                                <td
                                                    class="px-4 py-3 text-center"
                                                >
                                                    {#if !col.nullable}
                                                        <span
                                                            class="text-red-500/80 text-[10px] font-bold px-1.5 py-0.5 bg-red-500/5 rounded border border-red-500/10"
                                                            >NOT NULL</span
                                                        >
                                                    {:else}
                                                        <span
                                                            class="text-muted-foreground opacity-30 text-[10px]"
                                                            >null</span
                                                        >
                                                    {/if}
                                                </td>
                                                <td
                                                    class="px-4 py-3 font-mono text-muted-foreground truncate max-w-[150px]"
                                                    >{col.default_value ||
                                                        "—"}</td
                                                >
                                            </tr>
                                        {/each}
                                    </tbody>
                                </table>
                            </div>
                        </section>

                        <div class="grid grid-cols-1 lg:grid-cols-2 gap-8">
                            <!-- Indexes Section -->
                            <section class="space-y-4">
                                <h3
                                    class="text-xs font-bold uppercase tracking-widest text-muted-foreground/80 flex items-center space-x-2"
                                >
                                    <svg
                                        xmlns="http://www.w3.org/2000/svg"
                                        width="14"
                                        height="14"
                                        viewBox="0 0 24 24"
                                        fill="none"
                                        stroke="currentColor"
                                        stroke-width="2"
                                        stroke-linecap="round"
                                        stroke-linejoin="round"
                                        ><path
                                            d="M21 12V7a2 2 0 0 0-2-2H5a2 2 0 0 0-2 2v10a2 2 0 0 0 2 2h7"
                                        /><path d="m11 15 3 3 6-6" /><path
                                            d="M3 9h16"
                                        /></svg
                                    >
                                    <span>Indexes</span>
                                </h3>
                                <div class="space-y-3">
                                    {#each selectedTableDetails.indexes as idx}
                                        <div
                                            class="p-4 border border-border rounded-xl bg-card/50 shadow-sm flex flex-col space-y-3"
                                        >
                                            <div
                                                class="flex justify-between items-start"
                                            >
                                                <div class="space-y-1">
                                                    <div
                                                        class="font-bold text-foreground text-sm tracking-tight"
                                                    >
                                                        {idx.name}
                                                    </div>
                                                    <div
                                                        class="flex items-center space-x-2"
                                                    >
                                                        {#if idx.is_unique}
                                                            <span
                                                                class="text-[9px] px-1.5 py-0.5 bg-emerald-500/10 text-emerald-500 rounded font-bold uppercase border border-emerald-500/10"
                                                                >Unique</span
                                                            >
                                                        {/if}
                                                        <span
                                                            class="text-[9px] px-1.5 py-0.5 bg-muted text-muted-foreground rounded font-medium uppercase"
                                                            >{idx.columns
                                                                .length} columns</span
                                                        >
                                                    </div>
                                                </div>
                                                <div
                                                    class="text-muted-foreground/30"
                                                >
                                                    <svg
                                                        xmlns="http://www.w3.org/2000/svg"
                                                        width="16"
                                                        height="16"
                                                        viewBox="0 0 24 24"
                                                        fill="none"
                                                        stroke="currentColor"
                                                        stroke-width="2"
                                                        stroke-linecap="round"
                                                        stroke-linejoin="round"
                                                        ><path
                                                            d="m21 21-4.3-4.3"
                                                        /><circle
                                                            cx="10"
                                                            cy="10"
                                                            r="7"
                                                        /></svg
                                                    >
                                                </div>
                                            </div>
                                            <div class="flex flex-wrap gap-1.5">
                                                {#each idx.columns as col}
                                                    <span
                                                        class="px-2 py-1 bg-muted rounded font-mono text-[10px] border border-border/50 text-foreground/80"
                                                    >
                                                        {col.column_name}
                                                    </span>
                                                {/each}
                                            </div>
                                        </div>
                                    {:else}
                                        <div
                                            class="px-4 py-8 border border-dashed border-border rounded-xl text-center text-muted-foreground text-[11px] italic"
                                        >
                                            No explicit indexes found.
                                        </div>
                                    {/each}
                                </div>
                            </section>

                            <!-- Foreign Keys Section -->
                            <section class="space-y-4">
                                <h3
                                    class="text-xs font-bold uppercase tracking-widest text-muted-foreground/80 flex items-center space-x-2"
                                >
                                    <svg
                                        xmlns="http://www.w3.org/2000/svg"
                                        width="14"
                                        height="14"
                                        viewBox="0 0 24 24"
                                        fill="none"
                                        stroke="currentColor"
                                        stroke-width="2"
                                        stroke-linecap="round"
                                        stroke-linejoin="round"
                                        ><path d="m15 12-3-3-3 3" /><path
                                            d="m9 18 3 3 3-3"
                                        /><path d="M12 3v18" /></svg
                                    >
                                    <span>Foreign Keys</span>
                                </h3>
                                <div class="space-y-3">
                                    {#each selectedTableDetails.foreign_keys as fk}
                                        <div
                                            class="p-4 border border-border rounded-xl bg-card/50 shadow-sm flex items-center justify-between"
                                        >
                                            <div
                                                class="flex flex-col space-y-1"
                                            >
                                                <span
                                                    class="text-[10px] text-muted-foreground uppercase font-bold tracking-widest"
                                                    >Source</span
                                                >
                                                <span
                                                    class="font-mono text-xs font-bold text-foreground"
                                                    >{fk.column_name}</span
                                                >
                                            </div>
                                            <div
                                                class="px-4 text-muted-foreground/30 flex flex-col items-center"
                                            >
                                                <svg
                                                    xmlns="http://www.w3.org/2000/svg"
                                                    width="20"
                                                    height="20"
                                                    viewBox="0 0 24 24"
                                                    fill="none"
                                                    stroke="currentColor"
                                                    stroke-width="2"
                                                    stroke-linecap="round"
                                                    stroke-linejoin="round"
                                                    ><path d="M5 12h14" /><path
                                                        d="m12 5 7 7-7 7"
                                                    /></svg
                                                >
                                            </div>
                                            <div
                                                class="flex flex-col space-y-1 text-right"
                                            >
                                                <span
                                                    class="text-[10px] text-muted-foreground uppercase font-bold tracking-widest"
                                                    >Reference</span
                                                >
                                                <span
                                                    class="font-mono text-xs font-bold text-accent"
                                                    >{fk.ref_table}({fk.ref_column})</span
                                                >
                                            </div>
                                        </div>
                                    {:else}
                                        <div
                                            class="px-4 py-8 border border-dashed border-border rounded-xl text-center text-muted-foreground text-[11px] italic"
                                        >
                                            No foreign key relations.
                                        </div>
                                    {/each}
                                </div>
                            </section>
                        </div>

                        <!-- Raw Payload Debug (Now hidden behind a toggle) -->
                        <div class="pt-4 border-t border-border/40">
                            <details class="group">
                                <summary
                                    class="text-[10px] text-muted-foreground uppercase font-bold tracking-widest cursor-pointer hover:text-foreground transition-colors flex items-center space-x-1"
                                >
                                    <svg
                                        xmlns="http://www.w3.org/2000/svg"
                                        width="10"
                                        height="10"
                                        viewBox="0 0 24 24"
                                        fill="none"
                                        stroke="currentColor"
                                        stroke-width="2"
                                        stroke-linecap="round"
                                        stroke-linejoin="round"
                                        class="group-open:rotate-90 transition-transform"
                                        ><path d="m9 18 6-6-6-6" /></svg
                                    >
                                    <span>Raw JSON Payload</span>
                                </summary>
                                <div
                                    class="mt-4 bg-black/90 rounded-xl p-5 font-mono text-[10px] overflow-auto max-h-[300px] border border-border shadow-inner"
                                >
                                    <pre
                                        class="text-emerald-500/80">{JSON.stringify(
                                            selectedTableDetails,
                                            null,
                                            2,
                                        )}</pre>
                                </div>
                            </details>
                        </div>
                    </div>
                </div>
            {/if}
        </section>
    </div>
</div>

<style>
    /* Add any custom styles here */
</style>
