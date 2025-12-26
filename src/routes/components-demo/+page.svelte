<script lang="ts">
  import Button from "$lib/components/Button.svelte";
  import Select, { type Option } from "$lib/components/Select.svelte";
  import FloatingWindow from "$lib/components/FloatingWindow.svelte";
  import { cn } from "$lib/utils";
  import { getThemeContext } from "$lib/theme/context";
  import type { ThemeRecord, ThemeData } from "$lib/theme/types";

  const selectOptions: Option[] = [
    { value: "", label: "(none)" },
    { value: "alpha", label: "Alpha" },
    { value: "bravo", label: "Bravo" },
    { value: "charlie", label: "Charlie" },
  ];

  let btnVariant = $state<"solid" | "ghost" | "outline" | "subtle">("solid");
  let btnRadius = $state<"none" | "sm" | "md" | "lg" | "full">("md");
  let btnHeight = $state<"8" | "10" | "12">("10");
  let selectRadius = $state<"sm" | "md" | "lg">("md");
  let selectHeight = $state<"sm" | "md" | "lg">("md");
  let selected = $state("alpha");
  let windowOpen = $state(false);

  const themeCtx = getThemeContext();
  let themes = $state<ThemeRecord[]>([]);
  const themeCards = $derived(
    themes
      .map((t) => {
        try {
          const parsed = JSON.parse(t.theme_data) as ThemeData;
          return { record: t, data: parsed };
        } catch {
          return null;
        }
      })
      .filter(Boolean) as { record: ThemeRecord; data: ThemeData }[]
  );

  $effect(() => {
    const unsub = themeCtx.subscribe((s) => {
      themes = s.themes;
    });
    return () => unsub();
  });

  const varMap: { key: string; path: (ui: ThemeData["ui"]) => string }[] = [
    { key: "--theme-bg-primary", path: (ui) => ui.background.primary },
    { key: "--theme-bg-secondary", path: (ui) => ui.background.secondary },
    { key: "--theme-bg-tertiary", path: (ui) => ui.background.tertiary },
    { key: "--theme-bg-hover", path: (ui) => ui.background.hover },
    { key: "--theme-bg-active", path: (ui) => ui.background.active },
    { key: "--theme-fg-primary", path: (ui) => ui.foreground.primary },
    { key: "--theme-fg-secondary", path: (ui) => ui.foreground.secondary },
    { key: "--theme-fg-tertiary", path: (ui) => ui.foreground.tertiary },
    { key: "--theme-accent-primary", path: (ui) => ui.accent.primary },
    { key: "--theme-accent-hover", path: (ui) => ui.accent.hover },
    { key: "--theme-accent-active", path: (ui) => ui.accent.active },
    { key: "--theme-border-default", path: (ui) => ui.border.default },
    { key: "--theme-border-subtle", path: (ui) => ui.border.subtle },
    { key: "--theme-border-focus", path: (ui) => ui.border.focus },
  ];

  function isColor(val: string) {
    return (
      /^#([0-9a-f]{3}|[0-9a-f]{6}|[0-9a-f]{8})$/i.test(val) ||
      /^rgb(a)?\(/i.test(val) ||
      /^hsl(a)?\(/i.test(val)
    );
  }
</script>

<svelte:head>
  <title>Components Demo</title>
</svelte:head>

<div class="p-6 space-y-6 text-(--theme-fg-primary) bg-(--theme-bg-primary)">
  <section class="space-y-3 p-4 rounded-lg border border-(--theme-border-default) bg-(--theme-bg-secondary)">
    <div class="flex items-center justify-between flex-wrap gap-3">
      <h2 class="font-semibold text-sm">Button</h2>
      <div class="flex items-center gap-2 text-xs">
        <label class="flex items-center gap-1">
          <span>Variant</span>
          <Select
            options={["solid", "ghost", "outline", "subtle"]}
            value={btnVariant}
            onCommit={(v: string) => (btnVariant = v as typeof btnVariant)}
            radius="sm"
            class="min-w-[130px]"
          />
        </label>
        <label class="flex items-center gap-1">
          <span>Radius</span>
          <Select
            options={[
              { value: "none", label: "None" },
              { value: "sm", label: "Sm" },
              { value: "md", label: "Md" },
              { value: "lg", label: "Lg" },
              { value: "full", label: "Full" },
            ]}
            value={btnRadius}
            onCommit={(v: string) => (btnRadius = v as typeof btnRadius)}
            radius="sm"
            class="min-w-[130px]"
          />
        </label>
        <label class="flex items-center gap-1">
          <span>Height</span>
          <Select
            options={["8", "10", "12"]}
            value={btnHeight}
            onCommit={(v: string) => (btnHeight = v as typeof btnHeight)}
            radius="sm"
            class="min-w-[90px]"
          />
        </label>
      </div>
    </div>
    <div class="flex items-center gap-3 flex-wrap">
      <Button variant={btnVariant} radius={btnRadius} height={btnHeight}>Primary</Button>
      <Button variant={btnVariant} radius={btnRadius} height={btnHeight} disabled>Disabled</Button>
      <Button as="a" href="#" variant={btnVariant} radius={btnRadius} height={btnHeight}>Anchor</Button>
    </div>
  </section>

  <section class="space-y-3 p-4 rounded-lg border border-(--theme-border-default) bg-(--theme-bg-secondary)">
    <div class="flex items-center justify-between flex-wrap gap-3">
      <h2 class="font-semibold text-sm">Select</h2>
      <div class="flex items-center gap-2 text-xs">
        <label class="flex items-center gap-1">
          <span>Radius</span>
          <Select
            options={selectOptions}
            value={selectRadius}
            onCommit={(v: string) => (selectRadius = v as typeof selectRadius)}
            radius="sm"
            class="min-w-[130px]"
          />
        </label>
        <label class="flex items-center gap-1">
          <span>Height</span>
          <Select
            options={["sm", "md", "lg"]}
            value={selectHeight}
            onCommit={(v: string) => (selectHeight = v as typeof selectHeight)}
            radius="sm"
            class="min-w-[110px]"
          />
        </label>
      </div>
    </div>
    <div class="flex items-center gap-3 flex-wrap">
      <label>
        <Select
          options={selectOptions}
          value={selected}
          radius={selectRadius}
          height={selectHeight}
          onCommit={(v: string) => (selected = v)}
        />
      </label>
      <div class="text-sm opacity-80">Selected: {selected || "(none)"}</div>
    </div>
  </section>

  <section class="space-y-3 p-4 rounded-lg border border-(--theme-border-default) bg-(--theme-bg-secondary)">
    <div class="flex items-center justify-between flex-wrap gap-3">
      <h2 class="font-semibold text-sm">Floating Window</h2>
      <Button variant="outline" height="8" radius="md" onClick={() => (windowOpen = true)}>Open Window</Button>
    </div>
    <p class="text-sm opacity-80">Draggable dialog with optional modal overlay; shortcuts can be passed via props.</p>
  </section>

  <section class="space-y-3 p-4 rounded-lg border border-(--theme-border-default) bg-(--theme-bg-secondary)">
    <div class="flex items-center justify-between flex-wrap gap-3">
      <h2 class="font-semibold text-sm">Theme Variables</h2>
      <p class="text-xs opacity-70">Showing CSS variables and values for all loaded themes.</p>
    </div>
    <div class="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
      {#each themeCards as theme (theme.record.id)}
        <div class="rounded-lg border border-(--theme-border-default) bg-(--theme-bg-primary) shadow-sm overflow-hidden">
          <div class="flex items-center justify-between px-3 py-2 border-b border-(--theme-border-subtle)">
            <div class="min-w-0">
              <div class="text-sm font-semibold truncate">{theme.record.name}</div>
              <div class="text-xs opacity-70 truncate">{theme.record.description}</div>
            </div>
            <div class="text-[10px] px-2 py-0.5 rounded-full border border-(--theme-border-default) uppercase">{theme.record.id}</div>
          </div>
          <div class="grid grid-cols-2 gap-2 p-3">
            {#each varMap as entry}
              <div class="flex items-center gap-2 rounded-md border border-(--theme-border-subtle) px-2 py-1 bg-(--theme-bg-secondary)">
                {#if isColor(entry.path(theme.data.ui))}
                  <span class="h-4 w-4 rounded border border-(--theme-border-default)" style={`background:${entry.path(theme.data.ui)}`}></span>
                {:else}
                  <span class="h-4 w-4 rounded border border-(--theme-border-default) bg-(--theme-bg-tertiary)"></span>
                {/if}
                <div class="min-w-0">
                  <div class="text-[11px] font-medium truncate">{entry.key}</div>
                  <div class="text-[11px] opacity-80 truncate">{entry.path(theme.data.ui)}</div>
                </div>
              </div>
            {/each}
          </div>
        </div>
      {/each}
    </div>
  </section>
</div>

<FloatingWindow
  title="Demo Window"
  bind:open={windowOpen}
  modal={true}
  class="max-w-xl"
  openShortcut={undefined}
  closeShortcut={undefined}
  headerActions={undefined}
  onClose={() => (windowOpen = false)}
  onMaximize={() => {}}
>
  <div class="p-4 space-y-3 text-sm">
    <p>This window is draggable by the header bar.</p>
    <p>Pass <code>openShortcut</code> / <code>closeShortcut</code> to toggle via keyboard, and <code>onClose</code> / <code>onMaximize</code> handlers as needed.</p>
  </div>
</FloatingWindow>
