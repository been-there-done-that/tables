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

  const treeData = $derived(
    schemaStore.schemas.map((schema) => ({
      id: `schema:${schema.name}`,
      name: schema.name,
      type: "schema" as NodeType,
      children: schema.tables.map((table) => ({
        id: `table:${schema.name}.${table.table_name}`,
        name: table.table_name,
        type: "table" as NodeType,
        detail: table.table_type === "table" ? undefined : table.table_type,
        children: [
          {
            id: `cols:${schema.name}.${table.table_name}`,
            name: `Columns (${table.columns.length})`,
            type: "group" as NodeType,
            children: table.columns.map((col) => ({
              id: `col:${schema.name}.${table.table_name}.${col.column_name}`,
              name: col.column_name,
              type: (col.is_primary_key ? "primary_key" : "column") as NodeType,
              detail: col.logical_type,
            })),
          },
          {
            id: `idxs:${schema.name}.${table.table_name}`,
            name: `Indexes (${table.indexes.length})`,
            type: "group" as NodeType,
            children: table.indexes.map((idx) => ({
              id: `idx:${schema.name}.${table.table_name}.${idx.index_name}`,
              name: idx.index_name,
              type: "index" as NodeType,
              detail: idx.is_unique ? "Unique" : "",
            })),
          },
          {
            id: `fks:${schema.name}.${table.table_name}`,
            name: `Foreign Keys (${table.foreign_keys.length})`,
            type: "group" as NodeType,
            children: table.foreign_keys.map((fk) => ({
              id: `fk:${schema.name}.${table.table_name}.${fk.column_name}`,
              name: fk.column_name,
              type: "foreign_key" as NodeType,
              detail: `-> ${fk.ref_table}.${fk.ref_column}`,
            })),
          },
        ],
      })),
    })),
  );
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
                {#if schemaStore.activeConnectionId}
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
              <FileTree items={treeData} />
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
                        <h2 class="text-sm font-semibold">Main Editor</h2>
                      </div>
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
                            Select a tool from the list above to begin testing.
                          </p>
                        </div>
                        <SystemMetricsWidget />
                      </div>
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
