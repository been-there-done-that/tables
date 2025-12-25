<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { applyTheme } from "$lib/theme/applyTheme";
  import type { ThemeRecord } from "$lib/theme/types";
  import { setThemeContext } from "$lib/theme/context";
  import { onDestroy } from "svelte";
  import type { ThemeState } from "$lib/theme/context";

  let { children } = $props();

  let state = $state<ThemeState>({
    themes: [],
    activeId: "",
    loading: true,
    error: "",
  });
  const subscribers = new Set<(s: ThemeState) => void>();

  const notify = () => {
    subscribers.forEach((fn) => fn(state));
  };

  const setState = (updater: (s: ThemeState) => ThemeState) => {
    state = updater(state);
    notify();
  };

  async function loadThemes() {
    try {
      setState((s) => ({ ...s, loading: true, error: "" }));
      const themes = await invoke<ThemeRecord[]>("get_all_themes");
      const active = await invoke<ThemeRecord | null>("get_active_theme");
      const activeId = active?.id ?? "";
      applyTheme(active ?? themes.find((t) => t.is_active), false);
      setState(() => ({
        themes,
        activeId,
        loading: false,
        error: "",
      }));
    } catch (e) {
      setState((s) => ({ ...s, loading: false, error: String(e) }));
    }
  }

  async function setActive(id: string) {
    if (id === state.activeId) return;
    try {
      await invoke("set_active_theme", { themeId: id });
    } catch (e) {
      setState((s) => ({ ...s, error: String(e) }));
    }
  }

  setThemeContext({
    subscribe: (fn) => {
      subscribers.add(fn);
      fn(state);
      return () => subscribers.delete(fn);
    },
    setActive,
  });

  loadThemes();

  const unlisten = listen<ThemeRecord>("current-theme", (event) => {
    const theme = event.payload;
    applyTheme(theme);
    setState((s) => {
      const idx = s.themes.findIndex((t) => t.id === theme.id);
      const themes =
        idx >= 0
          ? Object.assign([...s.themes], { [idx]: theme })
          : [...s.themes, theme];
      return { ...s, themes, activeId: theme.id };
    });
  });

  onDestroy(() => {
    unlisten.then((off) => off());
  });
</script>

{@render children()}
