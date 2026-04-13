<script lang="ts">
  import ResizableSplitPane from "$lib/components/ResizableSplitPane.svelte";
  import ExplorerContainer from "$lib/components/explorer/ExplorerContainer.svelte";
  import EditorTabs from "$lib/components/EditorTabs.svelte";
  import EditorHome from "$lib/components/EditorHome.svelte";
  import { windowState } from "$lib/stores/window.svelte"; // layout.showSqlEditor used here
  import SqlTestingEditor from "$lib/components/SqlTestingEditor.svelte";
  import TablePreview from "$lib/components/table/TablePreview.svelte";
  import QueryLogsPanel from "$lib/components/QueryLogsPanel.svelte";
  import PendingChangesPanel from "$lib/components/table/PendingChangesPanel.svelte";
  import AgentPanel from "$lib/components/agent/AgentPanel.svelte";
  import { logsStore } from "$lib/stores/logs.svelte";
  import BottomPanel from "$lib/components/BottomPanel.svelte";
  import { schemaStore } from "$lib/stores/schema.svelte";
  import ErdView from "$lib/components/erd/ErdView.svelte";

  const activeSession = $derived(windowState.activeSession);
  const editorRestoring = $derived(windowState.restoringSession);

  $effect(() => {
    console.log(`[Page] activeSession=${activeSession?.id ?? "null"}, restoringSession=${editorRestoring} at ${Date.now()}ms`);
  });
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
        <div id="explorer-sidebar" class="h-full w-full overflow-hidden">
          <ExplorerContainer />
        </div>
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
              <div id="main-content-area" class="relative h-full w-full">
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

                      {#if editorRestoring || (!activeSession && schemaStore.status === "connecting")}
                        <!-- Schema connecting or sessions hydrating — code skeleton shimmer -->
                        <div class="flex flex-1 flex-col gap-0 overflow-hidden">
                          <!-- fake toolbar -->
                          <div class="flex h-8 shrink-0 items-center gap-2 border-b border-border px-3">
                            <div class="shimmer h-4 w-16 rounded"></div>
                            <div class="shimmer h-4 w-10 rounded"></div>
                            <div class="ml-auto shimmer h-4 w-8 rounded"></div>
                            <div class="shimmer h-4 w-8 rounded"></div>
                          </div>
                          <!-- fake code lines -->
                          <div class="flex flex-1 flex-col gap-2.5 p-4">
                            {#each [40, 65, 55, 80, 0, 45, 70, 0, 60, 50, 75, 35] as w}
                              {#if w === 0}
                                <div class="h-2"></div>
                              {:else}
                                <div class="shimmer h-3.5 rounded" style="width: {w}%;"></div>
                              {/if}
                            {/each}
                          </div>
                        </div>
                      {:else if !activeSession || activeSession.views.length === 0}
                        <EditorHome />
                      {:else if windowState.layout.showSqlEditor}
                        <div class="flex-1 relative overflow-hidden">
                          {#each activeSession.views as view (view.id)}
                            {#if view.id === activeSession.activeViewId}
                              <SqlTestingEditor
                                id={view.id}
                                bind:context={view.data}
                                {view}
                              />
                            {/if}
                          {/each}
                        </div>
                      {:else}
                        <!-- ERD views: always mounted, CSS-hidden when inactive.
                             Avoids re-running the expensive ELK layout on every tab switch. -->
                        {#each activeSession.views as view (view.id)}
                          {#if view.type === 'erd'}
                            <div
                              class="absolute inset-0 overflow-hidden"
                              class:hidden={view.id !== activeSession.activeViewId}
                            >
                              <ErdView
                                tables={view.data?.tables ?? []}
                                connectionId={view.data?.connectionId ?? ''}
                                schema={view.data?.schema ?? 'public'}
                              />
                            </div>
                          {/if}
                        {/each}
                        <!-- Non-ERD views: standard mount/unmount on tab switch -->
                        {#each activeSession.views as view (view.id)}
                          {#if view.id === activeSession.activeViewId && view.type !== 'erd'}
                            <div class="flex-1 overflow-hidden relative">
                              {#if view.type === "editor"}
                                <SqlTestingEditor
                                  id={view.id}
                                  context={view.data}
                                  {view}
                                />
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

                  <!-- Bottom Panel (Mirroring Top Tabs + Results) -->
                  {#snippet right()}
                    <BottomPanel />
                  {/snippet}
                </ResizableSplitPane>
              </div>
            {/snippet}

            <!-- Right Sidebar -->
            {#snippet right()}
              {#if windowState.activeRightPanel === "logs"}
                <QueryLogsPanel />
              {:else if windowState.activeRightPanel === "pending-changes"}
                <PendingChangesPanel />
              {:else if windowState.activeRightPanel === "claude"}
                <AgentPanel />
              {:else}
                <div class="flex h-full flex-col bg-muted/10">
                  <div
                    class="flex h-8 flex-none items-center border-b border-border px-4"
                  ></div>
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

