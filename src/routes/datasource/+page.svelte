<script lang="ts">
  import ResizableWindow from "$lib/components/ResizableWindow.svelte";
  import DataSource from "$lib/components/datasource/DataSource.svelte";

  let open = $state(true);
</script>

<DataSource />
