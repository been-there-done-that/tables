<script lang="ts">
  let { children } = $props();
  import "../app.css";
  import ThemeProvider from "$lib/providers/ThemeProvider.svelte";
  import LoadingOverlay from "$lib/LoadingOverlay.svelte";
</script>

<ThemeProvider>
  <LoadingOverlay />
  {@render children()}
</ThemeProvider>
