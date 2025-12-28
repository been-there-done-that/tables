<script>
  import ResizableSplitPane from "$lib/components/ResizableSplitPane.svelte";
</script>

<div class="flex h-full w-full flex-col bg-background text-foreground">
  <div class="relative flex-1 overflow-hidden">
    <!-- Level 1: Left Sidebar vs Everything Else -->
    <ResizableSplitPane defaultRatio={0.2} minLeft="200px" minRight="300px">
      <!-- Left Panel: Sidebar -->
      {#snippet left()}
        <div class="flex h-full flex-col bg-muted/20">
          <div
            class="flex h-8 flex-none items-center border-b border-(--theme-border-default) bg-background/50 px-4"
          >
            <h2 class="text-sm font-semibold">Explorer</h2>
          </div>
          <div class="flex-1 overflow-auto p-4">
            <p class="text-xs text-muted-foreground">List items...</p>
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
                >
                  <!-- Editor -->
                  {#snippet left()}
                    <div class="flex h-full flex-col bg-background">
                      <div
                        class="flex h-8 flex-none items-center border-b border-(--theme-border-default) px-4"
                      >
                        <h2 class="text-sm font-semibold">Main Editor</h2>
                      </div>
                      <div class="flex-1 overflow-auto p-4">
                        <div class="rounded border p-2">Editor Content</div>
                      </div>
                    </div>
                  {/snippet}

                  <!-- Bottom Panel (Clean, no title) -->
                  {#snippet right()}
                    <div class="flex h-full flex-col bg-background">
                      <!-- Just the top border for separation, no text/height -->
                      <div
                        class="h-px w-full flex-none border-t border-(--theme-border-default)"
                      ></div>
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
                  class="flex h-8 flex-none items-center border-b border-(--theme-border-default) px-4"
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
