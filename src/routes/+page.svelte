<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import Container from "$lib/Container.svelte";
  import { applyTheme } from "$lib/theme/applyTheme";
  import type { ThemeRecord } from "$lib/theme/types";

  let themes = $state<ThemeRecord[]>([]);
  let activeId = $state<string>("");
  let loading = $state(true);
  let error = $state("");


  async function loadThemes() {
    try {
      loading = true;
      error = "";
      themes = await invoke<ThemeRecord[]>("get_all_themes");
      const active = await invoke<ThemeRecord | null>("get_active_theme");
      activeId = active?.id ?? "";
      applyTheme(active ?? themes.find((t) => t.is_active), false);
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  async function handleSetActive(id: string) {
    if (id === activeId) return;
    try {
      await invoke("set_active_theme", { themeId: id });
    } catch (e) {
      error = String(e);
    }
  }

  loadThemes();
  listen<ThemeRecord>("theme-changed", (event) => {
    const theme = event.payload;
    activeId = theme.id;
    applyTheme(theme);
    const idx = themes.findIndex((t) => t.id === theme.id);
    if (idx >= 0) {
      themes[idx] = theme;
    }
  });
</script>

<main class="p-4">
  <Container className="space-y-6">
    <div class="flex items-center justify-between gap-4 flex-wrap">
      <div class="space-y-1">
        <div class="inline-flex items-center gap-2 rounded-lg border px-3 py-1 text-sm" style="border-color: var(--theme-border-default); background: color-mix(in srgb, var(--theme-bg-tertiary) 75%, transparent); color: var(--theme-fg-secondary);">
          Built-in themes
        </div>
        <h1 class="text-2xl font-semibold leading-tight" style="color: var(--theme-fg-primary);">Pick a theme</h1>
      </div>
      {#if loading}
        <div class="inline-flex items-center gap-2 rounded-lg border px-3 py-1 text-sm" style="border-color: var(--theme-border-default); background: color-mix(in srgb, var(--theme-bg-tertiary) 75%, transparent); color: var(--theme-fg-secondary);">
          Loading…
        </div>
      {:else if error}
        <div class="inline-flex items-center gap-2 rounded-lg border px-3 py-1 text-sm" style="border-color: var(--theme-border-default); background: color-mix(in srgb, var(--theme-bg-tertiary) 75%, transparent); color: #fca5a5;">
          {error}
        </div>
      {/if}
    </div>

    <div class="grid grid-cols-[repeat(auto-fill,minmax(220px,1fr))] gap-4">
      {#each themes as theme}
        <div
          role="button"
          tabindex="0"
          class="group flex w-full flex-col gap-3 rounded-xl border px-4 py-3 transition duration-150"
          style={`background: color-mix(in srgb, var(--theme-bg-secondary) 92%, transparent); border-color: var(--theme-border-default); box-shadow: ${
            theme.id === activeId
              ? "0 0 0 1px color-mix(in srgb, var(--theme-accent-primary) 35%, transparent)"
              : "none"
          };`}
          onclick={() => handleSetActive(theme.id)}
          onkeydown={(e) => {
            if (e.key === "Enter" || e.key === " ") {
              e.preventDefault();
              handleSetActive(theme.id);
            }
          }}
        >
          <div class="flex items-center justify-between gap-3">
            <div>
              <div class="font-semibold" style="color: var(--theme-fg-primary);">{theme.name}</div>
              <div class="text-xs" style="color: var(--theme-fg-secondary);">{theme.author}</div>
            </div>
            {#if theme.id === activeId}
              <div class="inline-flex items-center gap-2 rounded-lg border px-3 py-1 text-xs" style="border-color: var(--theme-border-default); background: color-mix(in srgb, var(--theme-bg-tertiary) 75%, transparent); color: var(--theme-fg-secondary);">
                Active
              </div>
            {/if}
          </div>
          {#if theme.description}
            <div class="text-sm" style="color: var(--theme-fg-secondary); margin-top: 6px;">{theme.description}</div>
          {/if}
          <div class="flex items-center gap-2 mt-2">
            {#each (() => {
              try {
                const ui = JSON.parse(theme.theme_data).ui;
                return [ui.background?.primary, ui.background?.secondary, ui.accent?.primary, ui.background?.hover, ui.background?.active].filter(Boolean);
              } catch {
                return [];
              }
            })() as color}
              <span class="h-4 w-4 rounded-md border" style={`background:${color}; border-color: var(--theme-border-subtle);`}></span>
            {/each}
          </div>
        </div>
      {/each}
    </div>
  </Container>
</main>
