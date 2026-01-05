<script lang="ts">
  import ResizableSplitPane from "$lib/components/ResizableSplitPane.svelte";
  import SystemMetricsWidget from "$lib/components/SystemMetricsWidget.svelte";
  import DatabaseExplorer from "$lib/components/explorer/DatabaseExplorer.svelte";
  import type { ExplorerNode } from "$lib/explorer/types";
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

  import { nodes } from "$lib/explorer/stores/nodes.svelte";

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

  function handleExplorerAction(action: string, node: ExplorerNode) {
    console.log("[ExplorerAction]", action, node);

    // Handle context menu actions OR click actions
    if (action === "query_console") {
      const title =
        node.kind === "schema"
          ? `Console: ${node.label}`
          : `Query: ${node.label}`;

      activeSession?.openView("editor", title, {
        dbName: node.meta?.database,
        schemaName: node.meta?.schema,
        tableName: node.meta?.table,
      });
    } else if (action === "view_diagram") {
      const diagramTitle = `Diagram: ${node.label}`;
      const vizData = {
        database: node.meta?.database || schemaStore.selectedDatabase,
        schema: node.meta?.schema || schemaStore.activeSchema,
        focusedTable:
          node.kind === "table" || node.kind === "column"
            ? node.label
            : undefined,
      };
      activeSession?.openView("schema-visualizer", diagramTitle, vizData);
    } else if (action === "refresh") {
      // Logic is handled in ExplorerRow usually, but if context menu triggers it:
      // We might need to call provider refresh?
      // ExplorerRow already has onRefresh binding.
      // If we trigger it here, we need access to provider?
      // Actually, DatabaseExplorer handles refresh internally via ExplorerRow.
      // But if ContextMenu emits "refresh", we should handle it here or pass it back?
      // DatabaseExplorer passed 'onAction' which is THIS function.
      // So if we want to refresh, we should probably call a method on DatabaseExplorer or
      // just let DatabaseExplorer handle 'refresh' action if it did intercept it.
      // Current DatabaseExplorer implementation delegates ALL onAction to this callback.
      // So we need to call `nodes.refresh(node)` (if it existed) or similar.
      // But `nodes` store doesn't have refresh logic (provider has).

      // Solution: Let's trigger a re-fetch via schemaStore for legacy compat or ignore for now?
      // Better: Let's assume refreshing via twistie is sufficient for now, or implement a global `refreshNode` helper.
      schemaStore.refresh(); // Global refresh fallback
    } else if (action === "open") {
      // Clicked a file (table/view)
      if (node.kind === "table") {
        activeSession?.openView("table", node.label, {
          tableName: node.label,
          schemaName: node.meta?.schema,
          dbName: node.meta?.database,
        });
      } else if (node.kind === "column") {
        // Open query with column
        const tableRef = `${node.meta?.schema}.${node.meta?.table}`;
        activeSession?.openView("editor", `Query: ${node.label}`, {
          initialValue: `SELECT * FROM ${tableRef} WHERE ${node.label} = ...`,
        });
      }
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
                    "size-5 opacity-70 transition-all group-hover:opacity-100",
                    schemaStore.status === "refreshing" &&
                      "animate-spin-reverse text-primary",
                  )}
                  style={schemaStore.status === "refreshing"
                    ? "animation-direction: reverse;"
                    : ""}
                />
              </button>
            </div>
          </div>

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
              <div
                class="animate-in fade-in slide-in-from-left-2 duration-300 h-full"
              >
                <DatabaseExplorer onAction={handleExplorerAction} />
              </div>
            {:else}
              <div
                class="animate-in fade-in slide-in-from-left-2 duration-300 h-full"
              >
                <DatabaseExplorer onAction={handleExplorerAction} />
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
