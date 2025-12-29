<script lang="ts">
  import Button from "$lib/components/Button.svelte";
  import Select, { type Option } from "$lib/components/Select.svelte";
  import DraggableWindow from "$lib/components/DraggableWindow.svelte";
  import SearchInput from "$lib/components/SearchInput.svelte";
  import CommandPalette, {
    type CommandItem,
  } from "$lib/components/CommandPalette.svelte";
  import Tree from "$lib/components/Tree.svelte";
  import type { TreeNode } from "$lib/components/Tree.svelte";
  import ThemePreview from "$lib/components/ThemePreview.svelte";
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
  let btnRadius = $state<"none" | "sm" | "md" | "lg" | "full">("sm");
  let btnHeight = $state<"8" | "10" | "12">("8");
  let selectRadius = $state<"sm" | "md" | "lg">("sm");
  let selectHeight = $state<"sm" | "md" | "lg">("sm");
  let selected = $state("alpha");
  let windowOpen = $state(false);
  let themeSearch = $state("");
  let activeId = $state("");
  let paletteOpen = $state(false);
  let themeSearchInput = $state<HTMLInputElement | null>(null);
  const treeItems: TreeNode[] = [
    {
      id: "src",
      label: "src",
      type: "folder",
      children: [
        {
          id: "routes",
          label: "routes",
          type: "folder",
          children: [{ id: "demo", label: "demo/+page.svelte", type: "file" }],
        },
        {
          id: "lib",
          label: "lib",
          type: "folder",
          children: [{ id: "components", label: "components", type: "folder" }],
        },
      ],
    },
    {
      id: "db",
      label: "database",
      type: "database",
      children: [
        { id: "themes", label: "themes.db", type: "file" },
        {
          id: "keys",
          label: "keys",
          type: "folder",
          children: [{ id: "secret", label: "api.key", type: "key" }],
        },
      ],
    },
  ];

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
      .filter(Boolean)
      .filter((t) =>
        `${t?.record.name ?? ""} ${t?.record.id ?? ""}`
          .toLowerCase()
          .includes(themeSearch.toLowerCase()),
      ) as { record: ThemeRecord; data: ThemeData }[],
  );

  $effect(() => {
    const unsub = themeCtx.subscribe((s) => {
      themes = s.themes;
      activeId = s.activeId;
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

  const paletteItems = $derived<CommandItem[]>(
    (() => {
      const base = themes ?? [];
      const currentIdx = base.findIndex((t) => t.id === activeId);
      const nextTheme = base.length
        ? base[(currentIdx + 1 + base.length) % base.length]
        : null;
      const prevTheme = base.length
        ? base[(currentIdx - 1 + base.length) % base.length]
        : null;
      return [
        {
          id: "open-window",
          label: "Open Floating Window",
          section: "Actions",
          shortcut: "⌘ O",
          hint: "Show the floating dialog",
          action: () => (windowOpen = true),
        },
        nextTheme && {
          id: "next-theme",
          label: `Next Theme (${nextTheme.name})`,
          section: "Themes",
          shortcut: "⌘ →",
          hint: "Activate next theme",
          action: () => themeCtx.setActive(nextTheme.id),
        },
        prevTheme && {
          id: "prev-theme",
          label: `Previous Theme (${prevTheme.name})`,
          section: "Themes",
          shortcut: "⌘ ←",
          hint: "Activate previous theme",
          action: () => themeCtx.setActive(prevTheme.id),
        },
        {
          id: "focus-search",
          label: "Focus Theme Search",
          section: "Themes",
          shortcut: "⌘ F",
          hint: "Jump to theme filter",
          action: () => themeSearchInput?.focus(),
        },
      ].filter(Boolean) as CommandItem[];
    })(),
  );

  $effect(() => {
    const handler = (e: KeyboardEvent) => {
      const isCmdOrCtrl = e.metaKey || e.ctrlKey;
      const key = e.key.toLowerCase();
      if (isCmdOrCtrl && key === "k") {
        e.preventDefault();
        paletteOpen = true;
      } else if (isCmdOrCtrl && key === "f") {
        // If palette is open, don't hijack normal browser find.
        if (document.activeElement === document.body) {
          e.preventDefault();
          themeSearchInput?.focus();
        }
      } else if (isCmdOrCtrl && key === "o") {
        e.preventDefault();
        windowOpen = true;
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  });
</script>

<svelte:head>
  <title>Components Demo</title>
</svelte:head>

<div
  class="h-full flex flex-col overflow-hidden text-(--theme-fg-primary) bg-(--theme-bg-primary)"
>
  <div
    class="flex-1 grid grid-cols-[0.4fr_0.6fr] gap-4 p-4 overflow-hidden min-h-0"
  >
    <!-- Left: themes + search + cards -->
    <aside
      class="flex flex-col gap-3 rounded-xl border border-(--theme-border-default) bg-(--theme-bg-secondary) p-3 overflow-hidden"
    >
      <div class="flex items-center justify-between">
        <h2 class="font-semibold text-sm">Themes</h2>
        <span
          class="text-[11px] px-2 py-0.5 rounded-full border border-(--theme-border-subtle)"
          >{themes.length} total</span
        >
      </div>
      <SearchInput
        placeholder="Search themes..."
        class="mx-1"
        bind:value={themeSearch}
        bind:inputRef={themeSearchInput}
      />
      <div class="overflow-auto flex-1 space-y-4 pr-1">
        {#each themeCards as theme (theme.record.id)}
          <div
            class="rounded-lg border border-(--theme-border-default) bg-(--theme-bg-primary) shadow-sm overflow-hidden flex flex-col transition-all duration-200"
            class:ring-2={theme.record.id === activeId}
            class:ring-(--theme-accent-primary)={theme.record.id === activeId}
          >
            <div class="p-2 border-b border-(--theme-border-subtle)">
              <ThemePreview theme={theme.record} />
            </div>

            <div class="flex items-center justify-between px-3 py-2">
              <div class="min-w-0">
                <div class="text-sm font-semibold truncate leading-none">
                  {theme.record.name}
                </div>
                <div class="text-[10px] opacity-70 truncate mt-1">
                  {theme.record.description}
                </div>
              </div>
              <div class="flex items-center gap-1.5 shrink-0">
                {#if theme.record.id === activeId}
                  <span
                    class="text-[10px] uppercase font-bold text-(--theme-accent-primary) px-1.5"
                    >Active</span
                  >
                {:else}
                  <button
                    type="button"
                    class="text-[11px] font-medium px-3 py-1 rounded-md border border-(--theme-border-default) bg-(--theme-bg-secondary) hover:bg-(--theme-bg-hover) active:bg-(--theme-bg-active) transition-colors"
                    onclick={() => themeCtx.setActive(theme.record.id)}
                  >
                    Apply
                  </button>
                {/if}
              </div>
            </div>

            <!-- Quick Swatches -->
            <div class="flex items-center gap-1.5 px-3 pb-3">
              {#each [theme.data.ui.background.primary, theme.data.ui.background.secondary, theme.data.ui.accent.primary, theme.data.ui.border.default] as color}
                <div
                  class="size-3 rounded-full border border-(--theme-border-subtle)"
                  style="background: {color};"
                ></div>
              {/each}
            </div>
          </div>
        {/each}
      </div>
    </aside>

    <!-- Right: controls + components -->
    <div class="flex flex-col gap-4 overflow-hidden min-h-0">
      <section
        class="space-y-3 p-4 rounded-lg border border-(--theme-border-default) bg-(--theme-bg-secondary)"
      >
        <div class="flex items-center justify-between flex-wrap gap-3">
          <h2 class="font-semibold text-sm">Controls</h2>
          <div class="flex items-center gap-2 text-xs flex-wrap">
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
            <Button
              variant="outline"
              height={btnHeight}
              radius={btnRadius}
              onClick={() => (paletteOpen = true)}
            >
              Open Palette (⌘K)
            </Button>
          </div>
        </div>
      </section>

      <div class="flex-1 overflow-auto space-y-4 pr-1">
        <section
          class="space-y-3 p-4 rounded-lg border border-(--theme-border-default) bg-(--theme-bg-secondary)"
        >
          <h2 class="font-semibold text-sm">Button</h2>
          <div class="flex items-center gap-3 flex-wrap">
            <Button variant={btnVariant} radius={btnRadius} height={btnHeight}
              >Primary</Button
            >
            <Button
              variant={btnVariant}
              radius={btnRadius}
              height={btnHeight}
              disabled>Disabled</Button
            >
            <Button
              as="a"
              href="#"
              variant={btnVariant}
              radius={btnRadius}
              height={btnHeight}>Anchor</Button
            >
          </div>
        </section>

        <section
          class="space-y-3 p-4 rounded-lg border border-(--theme-border-default) bg-(--theme-bg-secondary)"
        >
          <div class="flex items-center justify-between flex-wrap gap-3">
            <h2 class="font-semibold text-sm">Select</h2>
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
            <div class="text-sm opacity-80">
              Selected: {selected || "(none)"}
            </div>
          </div>
        </section>

        <section
          class="space-y-3 p-4 rounded-lg border border-(--theme-border-default) bg-(--theme-bg-secondary)"
        >
          <div class="flex items-center justify-between flex-wrap gap-3">
            <h2 class="font-semibold text-sm">Floating Window</h2>
            <Button
              variant="outline"
              height={btnHeight}
              radius={btnRadius}
              onClick={() => (windowOpen = true)}>Open Window</Button
            >
          </div>
          <p class="text-sm opacity-80">
            Draggable dialog with optional modal overlay; shortcuts can be
            passed via props.
          </p>
        </section>

        <section
          class="space-y-3 p-4 rounded-lg border border-(--theme-border-default) bg-(--theme-bg-secondary)"
        >
          <div class="flex items-center justify-between flex-wrap gap-3">
            <h2 class="font-semibold text-sm">Command Palette</h2>
            <div class="text-xs opacity-80">
              Try ⌘/Ctrl + K, ⌘/Ctrl + F, ⌘/Ctrl + O
            </div>
          </div>
          <p class="text-sm opacity-80">
            Search and trigger actions. Uses keyboard navigation
            (↑/↓/Enter/Esc).
          </p>
          <Button
            variant="outline"
            height={btnHeight}
            radius={btnRadius}
            onClick={() => (paletteOpen = true)}
          >
            Open Palette
          </Button>
        </section>

        <section
          class="space-y-3 p-4 rounded-lg border border-(--theme-border-default) bg-(--theme-bg-secondary)"
        >
          <div class="flex items-center justify-between flex-wrap gap-3">
            <h2 class="font-semibold text-sm">Tree</h2>
            <div class="text-xs opacity-80">
              Hover to show arrows; click row or arrow to toggle.
            </div>
          </div>
          <div
            class="rounded-md border border-(--theme-border-subtle) bg-(--theme-bg-primary) p-2"
          >
            <Tree items={treeItems} />
          </div>
        </section>
      </div>
    </div>
  </div>
</div>

<DraggableWindow
  title="Demo Window"
  bind:open={windowOpen}
  modal={true}
  class="max-w-xl"
  openShortcut={undefined}
  closeShortcut={undefined}
  headerActions={undefined}
  onClose={() => (windowOpen = false)}
>
  <div class="p-4 space-y-3 text-sm">
    <p>This window is draggable by the header bar.</p>
    <p>
      Pass <code>openShortcut</code> / <code>closeShortcut</code> to toggle via
      keyboard, and <code>onClose</code> / <code>onMaximize</code> handlers as needed.
    </p>
  </div>
</DraggableWindow>

<CommandPalette
  bind:open={paletteOpen}
  items={paletteItems}
  placeholder="Type a command or search..."
/>
