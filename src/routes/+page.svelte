<script>
  import ResizableSplitPane from "$lib/components/ResizableSplitPane.svelte";
</script>

<div class="h-full w-full bg-background text-foreground flex flex-col">
  <div class="flex-1 overflow-hidden relative">
    <ResizableSplitPane defaultRatio={0.2} minLeft="200px" minRight="300px">
      <!-- Left Panel -->
      <div slot="left" class="h-full flex flex-col bg-muted/20">
        <!-- Header -->
        <div
          class="h-8 border-b border-(--theme-border-default) flex items-center px-4 bg-background/50 flex-none"
        >
          <h2 class="font-semibold text-sm">Explorer</h2>
        </div>
        <!-- Scrollable Content -->
        <div class="flex-1 overflow-auto p-4">
          <p class="text-xs text-muted-foreground">List items...</p>
        </div>
      </div>

      <!-- Right Panel -->
      <div slot="right" class="h-full flex flex-col bg-background">
        <!-- Header -->
        <div
          class="h-8 border-b border-(--theme-border-default) flex items-center px-4 flex-none"
        >
          <h2 class="font-semibold text-sm">Database Explorer</h2>
        </div>
        <!-- Scrollable Content -->
        <div class="flex-1 overflow-auto p-4">
          <div class="p-2 border rounded">Main View Content</div>
        </div>
      </div>
    </ResizableSplitPane>
  </div>
</div>
