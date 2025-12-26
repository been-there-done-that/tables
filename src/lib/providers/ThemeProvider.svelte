<script lang="ts">
  import { listen } from "@tauri-apps/api/event";
  import { applyTheme } from "$lib/theme/applyTheme";
  import type { ThemeRecord } from "$lib/theme/types";
  import type { Theme } from "$lib/commands/types";
  import { setThemeContext } from "$lib/theme/context";
  import { onDestroy } from "svelte";
  import type { ThemeState } from "$lib/theme/context";
  import { themeStore, DELAYS } from '$lib/commands/stores.svelte';

  let { children } = $props();

  const delay = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

  // Track current theme state to detect changes
  let currentThemeState = $state({
    themes: themeStore.state.themes,
    activeId: themeStore.state.activeId,
    loading: themeStore.state.loading,
    error: themeStore.state.error,
  });

  // Reactive effect to sync with store changes
  $effect(() => {
    const storeState = themeStore.state;
    currentThemeState = {
      themes: storeState.themes,
      activeId: storeState.activeId,
      loading: storeState.loading,
      error: storeState.error,
    };
  });

  async function loadThemes() {
    try {
      await themeStore.refresh();
      const active = themeStore.state.active;
      applyTheme(active ?? themeStore.state.themes.find((t: ThemeRecord) => t.is_active), false);
      // Additional delay after theme application for smooth transition
      await delay(DELAYS.THEME_TRANSITION);
    } catch (e) {
      console.error("Failed to load themes:", e);
    }
  }

  async function setActive(id: string) {
    console.log("setActive called with:", id);
    try {
      await themeStore.setActiveTheme(id);
      console.log("setActiveTheme completed successfully");
    } catch (e) {
      console.error("Failed to set active theme:", e);
    }
  }

  // Create adapter that uses reactive state
  setThemeContext({
    subscribe: (fn) => {
      // Subscribe to reactive state changes
      let unsubscribe: (() => void) | undefined;
      
      const updateSubscriber = () => {
        fn(currentThemeState);
      };
      
      // Initial call
      updateSubscriber();
      
      // Use $effect to track changes
      $effect(() => {
        updateSubscriber();
      });
      
      unsubscribe = () => {
        // No cleanup needed for $effect
      };
      
      return unsubscribe;
    },
    setActive,
  });

  loadThemes();

  const unlisten = listen<Theme>("current-theme", (event) => {
    console.log("Theme change event received:", event.payload);
    const theme = event.payload;
    console.log("Applying theme:", theme);
    applyTheme(theme);
    // Update the store state when theme changes via backend
    const storeState = themeStore.state;
    if (storeState.activeId !== theme.id) {
      console.log("Syncing store with backend theme:", theme.id);
      // Force refresh to sync with backend
      themeStore.loadActiveTheme();
    }
  });

  onDestroy(() => {
    unlisten.then((off) => off());
  });
</script>

{@render children()}
