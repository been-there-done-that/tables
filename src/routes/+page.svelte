<script lang="ts">
  import ResizableSplitPane from "$lib/components/ResizableSplitPane.svelte";
  import SystemMetricsWidget from "$lib/components/SystemMetricsWidget.svelte";
  import FileTree, {
    type NodeType,
    type TreeNode,
  } from "$lib/components/explorer/FileTree.svelte";
  import EditorTabs from "$lib/components/EditorTabs.svelte";
  import EditorHome from "$lib/components/EditorHome.svelte";
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

  import SqlTestingEditor from "$lib/components/SqlTestingEditor.svelte";
  import SchemaVisualizer from "$lib/components/visualizer/SchemaVisualizer.svelte";

  let fileTree = $state<any>(null);
  let showSqlEditor = $state(false);

  const activeSession = $derived(windowState.activeSession);

  // Cache for lazily-loaded table details
  let tableDetailsCache = $state<Map<string, any>>(new Map());
  let loadingTables = $state<Set<string>>(new Set());
  let fetchedSchemas = new Set<string>(); // Non-reactive set to track auto-fetches per session

  // Ensure a session exists when schemaStore has an active connection
  $effect(() => {
    const conn = schemaStore.activeConnection;
    const hasSession = !!windowState.activeSession;

    if (conn && !hasSession && schemaStore.status === "idle") {
      console.log(`[AutoSession] Creating session for connection ${conn.id}`);
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
      const tables = schema.tables.filter((t) => t.table_type === "table");
      const views = schema.tables.filter((t) => t.table_type === "view");

      let children: TreeNode[] = [];

      if (tables.length > 0 || views.length > 0) {
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
      } else {
        // Check if schema has been introspected (loaded) but is empty
        if (schema.is_introspected) {
          children = [
            {
              id: `empty:${db.name}:${schema.name}`,
              name: "No tables found",
              type: "column" as NodeType, // Generic leaf type
            },
          ];
        } else {
          // Placeholder for lazy loading tables
          children = [
            {
              id: `placeholder:tables:${db.name}:${schema.name}`,
              name: "Loading tables...",
              type: "column" as NodeType,
            },
          ];
        }
      }

      return {
        id: `schema:${db.name}:${schema.name}`,
        name: schema.name,
        type: "schema" as NodeType,
        children,
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
            type: (col.is_primary_key ? "primary_key" : "column") as NodeType,
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
          children: (cachedDetails.foreign_keys || []).map((fk: any) => ({
            id: `fk:${tableId}.${fk.column_name}`,
            name: fk.column_name,
            type: "foreign_key" as NodeType,
            detail: `-> ${fk.ref_table}.${fk.ref_column}`,
          })),
        },
        {
          id: `triggers:${tableId}`,
          name: "Triggers",
          type: "group" as NodeType,
          count: cachedDetails.triggers?.length || 0,
          children: (cachedDetails.triggers || []).map((t: any) => ({
            id: `trigger:${tableId}.${t.trigger_name}`,
            name: t.trigger_name,
            type: "trigger" as NodeType,
            detail: `${t.timing} ${t.event}`,
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

    console.log(
      `[Effect] Checking pre-expanded tables: session=${!!session}, expanded=${expanded?.size || "null"}, hasConnection=${hasConnection}`,
    );

    if (!expanded || !schemaStore.activeConnection) return;

    console.log(`[Effect] Expanded keys:`, [...expanded]);

    // Find expanded table nodes and load their details
    for (const key of expanded) {
      // Check if this is a table key (format: table:db:schema.tableName or table:db:schema.tableName-index)
      // Check if this is a table key logic...
      if (key.startsWith("table:")) {
        console.log(`[Effect] Found table key: ${key}`);
        const match = key.match(/^table:([^:]+):([^.]+)\.([^-]+)/);
        if (match) {
          const [, dbName, schemaName, tableName] = match;
          const cacheKey = `${dbName}:${schemaName}:${tableName}`;

          // Load if not already cached or loading
          if (
            !tableDetailsCache.has(cacheKey) &&
            !loadingTables.has(cacheKey)
          ) {
            console.log(`[LazyLoad] Pre-expanded table detected: ${tableName}`);
            loadTableDetails(dbName, schemaName, tableName);
          }
        }
      }
      // NEW: Handle pre-expanded schemas
      else if (key.startsWith("schema:")) {
        // Format: schema:dbName:schemaName
        const parts = key.split(":");
        if (parts.length >= 3) {
          const dbName = parts[1];
          const schemaName = parts[2];

          const schemaKey = `schema-load:${dbName}:${schemaName}`;

          if (!fetchedSchemas.has(schemaKey)) {
            console.log(
              `[Effect] Pre-expanded schema detected: ${schemaName} in ${dbName}. Fetching tables...`,
            );
            fetchedSchemas.add(schemaKey);
            schemaStore.fetchTables(dbName, schemaName);
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
    if (!schemaStore.activeConnection) return;
    const cacheKey = `${dbName}:${schemaName}:${tableName}`;

    if (tableDetailsCache.has(cacheKey) || loadingTables.has(cacheKey)) return;

    loadingTables = new Set([...loadingTables, cacheKey]);

    try {
      console.time(`[LazyLoad] ${tableName}`);
      const details = await invoke<any>("get_cached_table_details", {
        connectionId: schemaStore.activeConnection?.id,
        database: dbName,
        schema: schemaName,
        tableName: tableName,
      });
      console.timeEnd(`[LazyLoad] ${tableName}`);

      tableDetailsCache = new Map(tableDetailsCache).set(cacheKey, details);
    } catch (e) {
      console.error(`Failed to load details for ${tableName}:`, e);
    } finally {
      loadingTables = new Set([...loadingTables].filter((k) => k !== cacheKey));
    }
  }

  function handleExplorerAction(node: TreeNode) {
    if (node.metadata?.schemaName) {
      schemaStore.activeSchema = node.metadata.schemaName;
    }
    if (!activeSession) return;

    if (node.type === "table") {
      activeSession.openView("table", node.name, { tableName: node.name });
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
    console.log("[handleContextMenuAction] Triggered", {
      action,
      nodeName: node.name,
      nodeType: node.type,
      hasSession: !!activeSession,
    });

    if (!activeSession) {
      if (schemaStore.activeConnection) {
        console.log(
          "[handleContextMenuAction] No session, starting one for",
          schemaStore.activeConnection.name,
        );
        windowState.startSession(schemaStore.activeConnection);
      } else {
        console.error(
          "[handleContextMenuAction] No active session or connection found!",
        );
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
        console.log("[handleContextMenuAction] Opening View", {
          type: "editor",
          title,
        });
        session.openView("editor", title, node.metadata);
        break;
      case "view_diagram":
        const diagramTitle =
          node.type === "schema"
            ? `Diagram: ${node.name}`
            : `Diagram: ${node.name} (Related)`;

        // Prepare metadata for visualizer
        const vizData = {
          database: node.metadata?.dbName || schemaStore.selectedDatabase,
          schema: node.metadata?.schemaName || schemaStore.activeSchema,
          focusedTable:
            node.type === "table" || node.type === "column"
              ? node.name
              : undefined,
        };

        // For schema nodes, they might store dbName/schemaName differently or just be 'schema' type
        // The FileTree creation logic sets metadata: { dbName: ..., schemaName: ... } for schemas.
        // For tables, it sets { dbName, schemaName, tableName }.

        session.openView("schema-visualizer", diagramTitle, vizData);
        break;
      case "refresh":
        const dbName = node.metadata?.dbName || schemaStore.selectedDatabase;
        const schemaName = node.metadata?.schemaName;
        if (dbName && schemaName) {
          schemaStore.refresh(dbName, schemaName);
        } else {
          schemaStore.refresh();
        }
        break;
      // Stubs for other actions
      default:
        console.log(
          `[handleContextMenuAction] Action "${action}" not implemented for node ${node.name}`,
        );
    }
  }

  async function handleNodeExpand(node: TreeNode, isOpen: boolean) {
    console.log(
      `[handleNodeExpand] type=${node.type}, name=${node.name}, isOpen=${isOpen}, metadata=`,
      node.metadata,
    );

    if (isOpen && node.type === "database") {
      schemaStore.fetchSchemas(node.name);
    }

    if (isOpen && node.type === "schema" && node.metadata?.dbName) {
      console.log(
        `[handleNodeExpand] Fetching tables for schema: ${node.name} in db: ${node.metadata.dbName}`,
      );
      schemaStore.fetchTables(node.metadata.dbName, node.name);
    } else if (isOpen && node.type === "schema") {
      console.warn(
        `[handleNodeExpand] Schema node expanded but missing dbName metadata!`,
        node,
      );
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

<div class="flex h-full w-full flex-col bg-background text-foreground">
  <div class="relative flex-1 overflow-hidden">
    <!-- Level 1: Left Sidebar vs Everything Else -->
    <ResizableSplitPane
      defaultRatio={0.2}
      minLeft="200px"
      minRight="300px"
      leftVisible={windowState.layout.left}
    >
      <!-- Left Panel: Sidebar -->
      {#snippet left()}
        <div class="flex h-full flex-col bg-muted/20">
          <div
            class="flex h-8 flex-none items-center border-b border-border bg-background/50 px-4"
          >
            <h2 class="text-sm font-semibold">
              Explorer
              <a href="/visualizer">Visualizer</a>
            </h2>
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
                class:text-primary={showSqlEditor}
                class:bg-accent={showSqlEditor}
                title="Toggle SQL Playground"
                onclick={() => (showSqlEditor = !showSqlEditor)}
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
          {#if schemaStore.status === "refreshing"}
            <div
              class="absolute inset-0 z-50 flex items-center justify-center bg-background/50 backdrop-blur-[1px]"
            >
              <IconLoader2 class="size-5 animate-spin text-muted-foreground" />
            </div>
          {/if}
          <div
            class={cn(
              "flex-1 overflow-auto p-2 transition-all duration-300",
              schemaStore.status === "refreshing" &&
                "opacity-50 pointer-events-none grayscale-[0.5]",
            )}
          >
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
      {/snippet}

      <!-- Right Panel: Contains Center Area + Right Sidebar -->
      {#snippet right()}
        <div class="relative h-full w-full">
          <!-- Level 2: Center Area vs Right Sidebar -->
          <ResizableSplitPane
            defaultRatio={0.75}
            minLeft="300px"
            minRight="200px"
            rightVisible={windowState.layout.right}
          >
            <!-- Center Area (Editor + Bottom Panel) -->
            {#snippet left()}
              <div class="relative h-full w-full">
                <!-- Level 3: Vertical Split (Editor / Bottom) -->
                <ResizableSplitPane
                  orientation="vertical"
                  defaultRatio={0.7}
                  minLeft="100px"
                  minRight="50px"
                  rightVisible={windowState.layout.bottom}
                >
                  <!-- Editor -->
                  {#snippet left()}
                    <div class="flex h-full flex-col bg-background">
                      <EditorTabs />

                      {#if !activeSession || activeSession.views.length === 0}
                        <EditorHome />
                      {:else if showSqlEditor}
                        <div class="flex-1 relative overflow-hidden">
                          <SqlTestingEditor
                            context={activeSession.views.find(
                              (v) => v.id === activeSession.activeViewId,
                            )?.data}
                          />
                        </div>
                      {:else}
                        {@const activeView = activeSession.views.find(
                          (v) => v.id === activeSession.activeViewId,
                        )}
                        <div class="flex-1 overflow-hidden relative">
                          {#if activeView?.type === "editor"}
                            <SqlTestingEditor context={activeView.data} />
                          {:else if activeView?.type === "schema-visualizer"}
                            <div class="h-full w-full">
                              <SchemaVisualizer
                                database={activeView.data?.database}
                                schema={activeView.data?.schema}
                                focusedTable={activeView.data?.focusedTable}
                              />
                            </div>
                          {:else if activeView?.type === "table"}
                            <div
                              class="flex items-center justify-center h-full text-muted-foreground italic text-sm"
                            >
                              Table Browser: {activeView.title} (Coming Soon)
                            </div>
                          {:else}
                            <!-- Default Fallback -->
                            <div class="flex-1 overflow-auto p-4 space-y-4">
                              <!-- ... (previous demo content) ... -->
                              <pre
                                class="p-4 bg-muted/30 rounded border border-border text-xs">View ID: {activeSession.activeViewId}</pre>
                            </div>
                          {/if}
                        </div>
                      {/if}
                    </div>
                  {/snippet}

                  <!-- Bottom Panel (Clean, no title) -->
                  {#snippet right()}
                    <div class="flex h-full flex-col bg-background">
                      <!-- Removed redundant border -->
                      <div class="flex-1 overflow-auto p-2">
                        <pre class="text-xs font-mono">System ready.</pre>
                      </div>
                    </div>
                  {/snippet}
                </ResizableSplitPane>
              </div>
            {/snippet}

            <!-- Right Sidebar -->
            {#snippet right()}
              <div class="flex h-full flex-col bg-muted/10">
                <div
                  class="flex h-8 flex-none items-center border-b border-border px-4"
                >
                  <h2 class="text-sm font-semibold">Properties</h2>
                </div>
                <div class="flex-1 overflow-auto p-4">
                  <p class="text-xs">Property details...</p>
                </div>
              </div>
            {/snippet}
          </ResizableSplitPane>
        </div>
      {/snippet}
    </ResizableSplitPane>
  </div>

  <footer
    class="flex h-6 flex-none items-center border-t border-border bg-muted/5 px-4"
  >
    <div class="flex items-center gap-2">
      <div
        class="size-1.5 rounded-full bg-primary animate-pulse {schemaStore.status ===
        'idle'
          ? 'hidden'
          : ''}"
      ></div>
      <div
        class="text-[10px] text-muted-foreground font-medium uppercase tracking-wider"
      >
        {schemaStore.statusMessage}
      </div>
    </div>
  </footer>
</div>
