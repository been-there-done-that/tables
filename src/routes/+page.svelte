<script lang="ts">
  import Container from "$lib/Container.svelte";
  import { getThemeContext } from "$lib/theme/context";
  import type { ThemeRecord } from "$lib/theme/types";
  import { connectionForm } from "$lib/components/datasource/connectionStore.svelte";
  import { invoke } from "@tauri-apps/api/core";

  let themes = $state<ThemeRecord[]>([]);
  let activeId = $state<string>("");
  let loading = $state(true);
  let error = $state("");
  let showDatasource = $state(false);

  const { subscribe, setActive } = getThemeContext();

  $effect(() => {
    const unsubscribe = subscribe(
      (s: {
        themes: ThemeRecord[];
        activeId: string;
        loading: boolean;
        error: string;
      }) => {
        themes = s.themes;
        activeId = s.activeId;
        loading = s.loading;
        error = s.error;
      },
    );
    return () => unsubscribe();
  });

  const handleSetActive = (id: string) => setActive(id);

  const openSettingsWindow = async () => {
    try {
      await invoke("open_appearance_window");
    } catch (e) {
      console.error("Failed to open appearance window:", e);
    }
  };
</script>

<main class="p-4">
  <div class="flex items-center gap-3 mb-4">
    <a href="/demo">Components Demo</a>
    <a href="/test">Test Route</a>
    <button
      class="px-3 py-2 text-sm rounded border border-(--theme-border-default) bg-(--theme-bg-secondary) text-(--theme-fg-primary) hover:bg-(--theme-bg-primary) transition"
      onclick={openSettingsWindow}
      type="button"
    >
      Appearance
    </button>
    <button
      class="px-3 py-2 text-sm rounded border border-(--theme-border-default) bg-(--theme-bg-secondary) text-(--theme-fg-primary) hover:bg-(--theme-bg-primary) transition"
      onclick={() => (showDatasource = true)}
      type="button"
    >
      Configure datasource
    </button>
  </div>
</main>
