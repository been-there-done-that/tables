<script lang="ts">
  import { getThemeContext } from "$lib/theme/context";
  import Loader from "$lib/svg/Loader.svelte";

  let themeLoading = $state(true);
  let visible = $derived(themeLoading);

  const { subscribe } = getThemeContext();

  $effect(() => {
    const unsub = subscribe((s) => {
      themeLoading = s.loading;
      console.log(`[LoadingOverlay] themeLoading=${s.loading} at ${Date.now()}ms`);
    });
    return () => unsub();
  });
</script>


{#if visible}
  <div class="fixed inset-0 z-50 flex items-center justify-center bg-[rgba(0,0,0,0.82)] backdrop-blur-md overflow-hidden">
    <!-- page-level accent glows -->
    <div class="absolute -left-16 -top-20 h-48 w-48 animate-pulse-slow rounded-full bg-[conic-gradient(from_90deg_at_50%_50%,#ffffff,transparent_60%)] opacity-45 blur-3xl"></div>
    <div class="absolute -right-20 -bottom-32 h-52 w-52 animate-pulse-slow rounded-full bg-[conic-gradient(from_210deg_at_50%_50%,#d4d4d4,transparent_55%)] opacity-40 blur-3xl"></div>

    <div class="relative flex items-center justify-center">
      <Loader speed="1.5s" />
    </div>
  </div>
{/if}

<style>
  .animate-pulse-slow {
    animation: pulse 2.4s ease-in-out infinite;
  }
  @keyframes pulse {
    0%, 100% { opacity: 0.35; transform: scale(0.95); }
    50% { opacity: 0.9; transform: scale(1.05); }
  }
</style>
