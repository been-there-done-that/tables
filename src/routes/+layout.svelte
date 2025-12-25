<script lang="ts">
  let { children } = $props();
  import "../app.css";
  import ThemeProvider from "$lib/providers/ThemeProvider.svelte";
</script>

<ThemeProvider>
  {@render children()}
</ThemeProvider>
