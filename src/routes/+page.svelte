<script lang="ts">
  import ResizableSplitPane from "$lib/components/ResizableSplitPane.svelte";
  import ExplorerContainer from "$lib/components/explorer/ExplorerContainer.svelte";
  import EditorTabs from "$lib/components/EditorTabs.svelte";
  import EditorHome from "$lib/components/EditorHome.svelte";
  import { windowState } from "$lib/stores/window.svelte"; // layout.showSqlEditor used here
  import SqlTestingEditor from "$lib/components/SqlTestingEditor.svelte";
  import TablePreview from "$lib/components/table/TablePreview.svelte";
  import QueryLogsPanel from "$lib/components/QueryLogsPanel.svelte";
  import { logsStore } from "$lib/stores/logs.svelte";

  const activeSession = $derived(windowState.activeSession);
</script>

<div class="flex h-full w-full flex-col bg-background text-foreground">
  <div class="relative flex-1 overflow-hidden">
    <!-- Level 1: Left Sidebar vs Everything Else -->
    <ResizableSplitPane
      defaultRatio={0.2}
      controlledRatio={windowState.layoutRatios.left}
      onRatioChange={(r) => windowState.setLeftRatio(r)}
      minLeft="200px"
      minRight="300px"
      leftVisible={windowState.layout.left}
    >
      <!-- Left Panel: Sidebar -->
      {#snippet left()}
        <ExplorerContainer />
      {/snippet}

      <!-- Right Panel: Contains Center Area + Right Sidebar -->
      {#snippet right()}
        <div class="relative h-full w-full">
          <!-- Level 2: Center Area vs Right Sidebar -->
          <ResizableSplitPane
            defaultRatio={0.75}
            controlledRatio={windowState.layoutRatios.right}
            onRatioChange={(r) => windowState.setRightRatio(r)}
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
                  controlledRatio={windowState.layoutRatios.bottom}
                  onRatioChange={(r) => windowState.setBottomRatio(r)}
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
                      {:else if windowState.layout.showSqlEditor}
                        <div class="flex-1 relative overflow-hidden">
                          {#each activeSession.views as view (view.id)}
                            {#if view.id === activeSession.activeViewId}
                              <SqlTestingEditor bind:context={view.data} />
                            {/if}
                          {/each}
                        </div>
                      {:else}
                        {#each activeSession.views as view (view.id)}
                          {#if view.id === activeSession.activeViewId}
                            <div class="flex-1 overflow-hidden relative">
                              {#if view.type === "editor"}
                                <SqlTestingEditor context={view.data} />
                              {:else if view.type === "table"}
                                <TablePreview bind:context={view.data} />
                              {:else}
                                <!-- Default Fallback -->
                                <div class="flex-1 overflow-auto p-4 space-y-4">
                                  <pre
                                    class="p-4 bg-muted/30 rounded border border-border text-xs">View ID: {view.id}</pre>
                                </div>
                              {/if}
                            </div>
                          {/if}
                        {/each}
                      {/if}
                    </div>
                  {/snippet}

                  <!-- Bottom Panel (Clean, no title) -->
                  {#snippet right()}
                    <div class="flex h-full flex-col bg-background">
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
              {#if logsStore.isOpen}
                <QueryLogsPanel />
              {:else}
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
              {/if}
            {/snippet}
          </ResizableSplitPane>
        </div>
      {/snippet}
    </ResizableSplitPane>
  </div>
</div>
