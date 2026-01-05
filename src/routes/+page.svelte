<script lang="ts">
  import ResizableSplitPane from "$lib/components/ResizableSplitPane.svelte";
  import SystemMetricsWidget from "$lib/components/SystemMetricsWidget.svelte";
  import DatabaseExplorer from "$lib/components/explorer/DatabaseExplorer.svelte";
  import {
    type ExplorerNode,
    type DatabaseDriver,
    createDriver,
  } from "$lib/components/explorer/drivers";
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

  // Create driver reactively based on active connection and database
  let explorerDriver = $derived.by(() => {
    const conn = schemaStore.activeConnection;
    const db = schemaStore.selectedDatabase;
    if (!conn || !db) return null;

    try {
      return createDriver(conn.engine, conn.id, db);
    } catch (e) {
      console.warn(`[Driver] Unsupported engine: ${conn.engine}`);
      return null;
    }
  });

  let explorer = $state<any>(null);
  let showSqlEditor = $state(false);

  const activeSession = $derived(windowState.activeSession);

  // Ensure a session exists when schemaStore has an active connection
  $effect(() => {
    const conn = schemaStore.activeConnection;
    const hasSession = !!windowState.activeSession;

    if (conn && !hasSession && schemaStore.status === "idle") {
      console.log(`[AutoSession] Creating session for connection ${conn.id}`);
      windowState.startSession(conn);
    }
  });

  function handleNodeSelect(node: ExplorerNode) {
    if (node.type === "schema" && node.metadata.schema) {
      schemaStore.activeSchema = node.metadata.schema;
    }

    if (!activeSession) return;

    if (node.type === "table" && node.metadata.tableName) {
      activeSession.openView("table", node.label, { tableName: node.label });
    } else if (node.type === "column" && node.metadata.columnName) {
      const tableRef = `${node.metadata.schema}.${node.metadata.tableName}`;

      activeSession.openView("editor", `Query: ${node.label}`, {
        initialValue: `SELECT * FROM ${tableRef} WHERE ${node.label} = ...`,
      });
    }
  }

  function handleContextMenuAction(action: string, node: any) {
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

  // NOTE: Context menu logic would need to be re-integrated into DatabaseExplorer
  // or handled via a wrapper. For now, we focus on the main interaction.
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
            <h2 class="text-sm font-semibold">Explorer</h2>
            <div class="ml-auto flex items-center gap-1">
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
            {:else if !schemaStore.activeConnection}
              <div
                class="flex flex-col items-center justify-center p-8 text-center h-full max-h-[400px]"
              >
                <h3 class="text-sm font-medium mb-1">Explorer</h3>
                <p class="text-xs text-muted-foreground mb-4 max-w-[180px]">
                  Select a database connection from the titlebar to browse your
                  data.
                </p>
                <div
                  class="flex items-center gap-2 text-[10px] text-primary bg-primary/5 px-2 py-1 rounded-full border border-primary/10"
                >
                  <PlaylistAdd class="size-3" />
                  <span>Quick Select (Meta+P)</span>
                </div>
              </div>
            {:else if activeSession}
              <div
                class="animate-in fade-in slide-in-from-left-2 duration-300 h-full"
              >
                <DatabaseExplorer
                  driver={explorerDriver}
                  onNodeSelect={handleNodeSelect}
                  onContextMenuAction={handleContextMenuAction}
                />
              </div>
            {:else}
              <div
                class="animate-in fade-in slide-in-from-left-2 duration-300 h-full"
              >
                <DatabaseExplorer
                  driver={explorerDriver}
                  onNodeSelect={handleNodeSelect}
                  onContextMenuAction={handleContextMenuAction}
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
