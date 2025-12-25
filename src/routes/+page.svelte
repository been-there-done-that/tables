<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";

  type Theme = {
    id: string;
    name: string;
    author?: string;
    description?: string;
    theme_data: string;
    is_builtin: boolean;
    is_active: boolean;
  };

  let themes = $state<Theme[]>([]);
  let activeId = $state<string>("");
  let loading = $state(true);
  let error = $state("");

  function applyTheme(theme: Theme | undefined) {
    if (!theme) return;
    try {
      const data = JSON.parse(theme.theme_data);
      console.log("Theme data:", {data});
      const root = document.documentElement;
      root.style.cssText = "";
      if (data.ui?.background) {
        root.style.setProperty("--theme-bg-primary", data.ui.background.primary);
        root.style.setProperty("--theme-bg-secondary", data.ui.background.secondary);
        root.style.setProperty("--theme-bg-tertiary", data.ui.background.tertiary);
        root.style.setProperty("--theme-bg-hover", data.ui.background.hover);
        root.style.setProperty("--theme-bg-active", data.ui.background.active);
      }
      if (data.ui?.foreground) {
        root.style.setProperty("--theme-fg-primary", data.ui.foreground.primary);
        root.style.setProperty("--theme-fg-secondary", data.ui.foreground.secondary);
        root.style.setProperty("--theme-fg-tertiary", data.ui.foreground.tertiary);
      }
      if (data.ui?.accent) {
        root.style.setProperty("--theme-accent-primary", data.ui.accent.primary);
        root.style.setProperty("--theme-accent-hover", data.ui.accent.hover);
        root.style.setProperty("--theme-accent-active", data.ui.accent.active);
      }
      if (data.ui?.border) {
        root.style.setProperty("--theme-border-default", data.ui.border.default);
        root.style.setProperty("--theme-border-subtle", data.ui.border.subtle);
        root.style.setProperty("--theme-border-focus", data.ui.border.focus);
      }
    } catch (e) {
      console.error("Failed to apply theme", e);
    }
  }

  async function loadThemes() {
    try {
      loading = true;
      error = "";
      themes = await invoke<Theme[]>("get_all_themes");
      const active = await invoke<Theme | null>("get_active_theme");
      activeId = active?.id ?? "";
      applyTheme(active ?? themes.find((t) => t.is_active));
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
  listen<Theme>("theme-changed", (event) => {
    const theme = event.payload;
    activeId = theme.id;
    applyTheme(theme);
    const idx = themes.findIndex((t) => t.id === theme.id);
    if (idx >= 0) {
      themes[idx] = theme;
    }
  });
</script>

<main class="container">
  <div class="row" style="justify-content: space-between">
    <div>
      <div class="pill">Built-in themes</div>
      <h1>Pick a theme</h1>
    </div>
    {#if loading}
      <div class="pill">Loading…</div>
    {:else if error}
      <div class="pill" style="color: #fca5a5;">{error}</div>
    {/if}
  </div>

  <div class="themes-grid">
    {#each themes as theme}
      <div
        class={`theme-card ${theme.id === activeId ? "active" : ""}`}
        role="button"
        tabindex="0"
        onclick={() => handleSetActive(theme.id)}
        onkeydown={(e) => e.key === "Enter" && handleSetActive(theme.id)}
      >
        <div class="row" style="justify-content: space-between">
          <div>
            <div style="font-weight: 600">{theme.name}</div>
            <div style="font-size: 12px; color: var(--muted);">{theme.author}</div>
          </div>
          {#if theme.id === activeId}
            <div class="pill">Active</div>
          {/if}
        </div>
        {#if theme.description}
          <div style="margin-top: 6px; color: var(--muted); font-size: 13px;">{theme.description}</div>
        {/if}
        <div class="swatch-row">
          {#if theme.theme_data}
            {#if (() => { try {
              const ui = JSON.parse(theme.theme_data).ui;
              return ui?.background ? [ui.background.primary, ui.background.secondary, ui.background.accent?.primary ?? ui.accent?.primary, ui.background.hover, ui.background.active].filter(Boolean) : [];
            } catch { return []; }})().length}
              {#each (() => {
                try {
                  const ui = JSON.parse(theme.theme_data).ui;
                  return [ui.background?.primary, ui.background?.secondary, ui.accent?.primary, ui.background?.hover, ui.background?.active].filter(Boolean);
                } catch {
                  return [];
                }
              })() as color}
                <span class="swatch" style={`background:${color};`}></span>
              {/each}
            {/if}
          {/if}
        </div>
      </div>
    {/each}
  </div>
</main>
