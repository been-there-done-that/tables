<script lang="ts">
  import ResizableSplitPane from "$lib/components/ResizableSplitPane.svelte";
  import SystemMetricsWidget from "$lib/components/SystemMetricsWidget.svelte";
  import FileTree, {
    type NodeType,
  } from "$lib/components/explorer/FileTree.svelte";
  import { windowState } from "$lib/stores/window.svelte";
  import { schemaStore } from "$lib/stores/schema.svelte";
  import IconLoader2 from "@tabler/icons-svelte/icons/loader-2";
  import { cn } from "$lib/utils";
  import IconRefresh from "@tabler/icons-svelte/icons/refresh";
  import IconDatabase from "@tabler/icons-svelte/icons/database";
  import Compact from "$lib/svg/Compact.svelte";
  import Expand from "$lib/svg/Expand.svelte";

  import SqlTestingEditor from "$lib/components/SqlTestingEditor.svelte";

  let fileTree: any;
  let showSqlEditor = $state(false);

  const treeData = $derived.by(() => {
    // ... (existing treeData logic) ...
    return schemaStore.schemas.map((schema) => {
      const isSqlite = schemaStore.activeConnection?.engine === "sqlite";

      let children: any[] = [];

      if (isSqlite) {
        const tables = schema.tables.filter((t) => t.table_type === "table");
        const views = schema.tables.filter((t) => t.table_type === "view");

        children = [
          {
            id: `folder:tables:${schema.name}`,
            name: "tables",
            type: "folder" as NodeType,
            count: tables.length,
            children: tables.map((table) => mapTableToNode(table, schema.name)),
          },
          {
            id: `folder:views:${schema.name}`,
            name: "views",
            type: "folder" as NodeType,
            count: views.length,
            children: views.map((table) => ({
              ...mapTableToNode(table, schema.name),
              detail: undefined,
            })),
          },
        ];
      } else {
        children = schema.tables.map((table) =>
          mapTableToNode(table, schema.name),
        );
      }

      return {
        id: `schema:${schema.name}`,
        name: schema.name,
        type: "schema" as NodeType,
        children,
      };
    });
  });

  function mapTableToNode(table: any, schemaName: string) {
    return {
      id: `table:${schemaName}.${table.table_name}`,
      name: table.table_name,
      type: "table" as NodeType,
      detail: table.table_type === "table" ? undefined : table.table_type,
      children: [
        {
          id: `cols:${schemaName}.${table.table_name}`,
          name: "Columns",
          type: "group" as NodeType,
          count: table.columns.length,
          children: table.columns.map((col: any) => ({
            id: `col:${schemaName}.${table.table_name}.${col.column_name}`,
            name: col.column_name,
            type: (col.is_primary_key ? "primary_key" : "column") as NodeType,
            detail: col.logical_type,
          })),
        },
        {
          id: `idxs:${schemaName}.${table.table_name}`,
          name: "Indexes",
          type: "group" as NodeType,
          count: table.indexes.length,
          children: table.indexes.map((idx: any) => ({
            id: `idx:${schemaName}.${table.table_name}.${idx.index_name}`,
            name: idx.index_name,
            type: "index" as NodeType,
            detail: idx.is_unique ? "Unique" : "",
          })),
        },
        {
          id: `fks:${schemaName}.${table.table_name}`,
          name: "Foreign Keys",
          type: "group" as NodeType,
          count: table.foreign_keys.length,
          children: table.foreign_keys.map((fk: any) => ({
            id: `fk:${schemaName}.${table.table_name}.${fk.column_name}`,
            name: fk.column_name,
            type: "foreign_key" as NodeType,
            detail: `-> ${fk.ref_table}.${fk.ref_column}`,
          })),
        },
      ],
    };
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
            <h2 class="text-sm font-semibold">Explorer</h2>
            <div class="ml-auto flex items-center gap-1">
              <button
                class="p-1 hover:bg-accent rounded-sm text-muted-foreground hover:text-foreground transition-colors"
                title="Collapse All"
                onclick={() => fileTree?.collapseAll()}
              >
                <Compact />
              </button>
              <button
                class="p-1 hover:bg-accent rounded-sm text-muted-foreground hover:text-foreground transition-colors"
                title="Expand All"
                onclick={() => fileTree?.expandAll()}
              >
                <Expand />
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
              "flex-1 overflow-auto p-2 transition-opacity duration-200",
              schemaStore.status === "refreshing" &&
                "opacity-50 pointer-events-none",
            )}
          >
            {#if schemaStore.schemas.length === 0 && schemaStore.status !== "connecting" && schemaStore.status !== "refreshing"}
              <div class="p-4 text-center text-xs text-muted-foreground">
                {#if schemaStore.activeConnection}
                  <p>No schema found.</p>
                  <button
                    class="mt-2 text-primary hover:underline"
                    onclick={() => schemaStore.refresh()}>Refresh Schema</button
                  >
                {:else}
                  <p>Select a connection to view schema.</p>
                {/if}
              </div>
            {:else}
              <FileTree items={treeData} bind:this={fileTree} />
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
                      <div
                        class="flex h-8 flex-none items-center border-b border-border px-4"
                      >
                        <h2 class="text-sm font-semibold">
                          {showSqlEditor ? "SQL Playground" : "Main Editor"}
                        </h2>
                      </div>

                      {#if showSqlEditor}
                        <div class="flex-1 relative overflow-hidden">
                          <SqlTestingEditor />
                        </div>
                      {:else}
                        <div class="flex-1 overflow-auto p-4 space-y-4">
                          <div class="flex flex-col gap-2">
                            <a
                              href="/demo"
                              class="inline-flex items-center px-4 py-2 rounded-md bg-primary text-primary-foreground hover:bg-accent hover:text-accent-foreground transition-colors w-fit"
                              >Demo</a
                            >
                            <a
                              href="/tree-test"
                              class="inline-flex items-center px-4 py-2 rounded-md bg-primary text-primary-foreground hover:bg-accent hover:text-accent-foreground transition-colors w-fit"
                              >Tree Test</a
                            >
                            <a
                              href="/table-test"
                              class="inline-flex items-center px-4 py-2 rounded-md bg-primary text-primary-foreground hover:bg-accent hover:text-accent-foreground transition-colors w-fit"
                              >Table Test</a
                            >
                            <a
                              href="/debug-monaco"
                              class="inline-flex items-center px-4 py-2 rounded-md bg-primary text-primary-foreground hover:bg-accent hover:text-accent-foreground transition-colors w-fit"
                              >Debug Monaco</a
                            >
                            <a
                              href="/monaco-raw"
                              class="inline-flex items-center px-4 py-2 rounded-md bg-primary text-primary-foreground hover:bg-accent hover:text-accent-foreground transition-colors w-fit"
                              >Raw Monaco</a
                            >
                            <a
                              href="/debug-schema"
                              class="inline-flex items-center px-4 py-2 rounded-md bg-primary text-primary-foreground hover:bg-accent hover:text-accent-foreground transition-colors w-fit"
                              >Debug Schema</a
                            >
                          </div>
                          <div
                            class="rounded-lg border border-border bg-muted/30 p-4"
                          >
                            <h3 class="mb-2 text-sm font-medium">
                              Editor Content
                            </h3>
                            <p class="text-xs text-muted-foreground">
                              Select a tool from the list above to begin
                              testing.
                            </p>
                          </div>
                          <SystemMetricsWidget />
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
</div>
