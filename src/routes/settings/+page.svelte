<script lang="ts">
  import BrushIcon from "@tabler/icons-svelte/icons/brush";
  import { getThemeContext } from "$lib/theme/context";
  import type { ThemeRecord } from "$lib/theme/types";
  import ThemeComponent from "./Theme.svelte";
  import ShortcutsComponent from "./Shortcuts.svelte";
  import KeyboardIcon from "@tabler/icons-svelte/icons/keyboard";

  import TypographyIcon from "@tabler/icons-svelte/icons/typography";
  import EditorComponent from "./Editor.svelte";

  import AlertTriangleIcon from "@tabler/icons-svelte/icons/alert-triangle";
  import DangerousComponent from "./Dangerous.svelte";

  import RefreshIcon from "@tabler/icons-svelte/icons/refresh";
  import UpdatesComponent from "./Updates.svelte";
  import AiIcon from "@tabler/icons-svelte/icons/ai";
  import AIComponent from "./AI.svelte";

  let themes = $state<ThemeRecord[]>([]);
  let activeId = $state<string>("");

  const { subscribe, setActive } = getThemeContext();

  $effect(() => {
    const unsubscribe = subscribe(
      (s: { themes: ThemeRecord[]; activeId: string }) => {
        themes = s.themes;
        activeId = s.activeId;
      },
    );
    return () => unsubscribe();
  });

  const handleSetActive = (id: string) => setActive(id);

  import { page } from "$app/stores";
  let selectedSection = $state<string>($page.url.searchParams.get("section") ?? "theme");

  let sections = [
    {
      name: "theme",
      icon: BrushIcon,
    },
    {
      name: "shortcuts",
      icon: KeyboardIcon,
    },
    {
      name: "editor",
      icon: TypographyIcon,
    },
    {
      name: "ai",
      icon: AiIcon,
    },
    {
      name: "dangerous",
      icon: AlertTriangleIcon,
    },
    {
      name: "updates",
      icon: RefreshIcon,
    },
  ];
</script>

<div class="flex flex-col w-full h-full bg-background overflow-hidden">
  <div class="flex grow w-full overflow-hidden">
    <!-- Sidebar -->
    <div class="max-w-60 w-full shrink-0 h-full border-r border-border">
      <div class="flex flex-col h-full bg-background border-r border-border">
        <div class="p-3 flex w-full flex-col mt-24">
          <div class="space-y-0.5">
            {#each sections as second}
              {@const IconComponent = second.icon}
              <button
                class="w-full text-left px-3 py-1.5 flex items-center space-x-2 text-sm rounded-md
                    {selectedSection === second.name
                  ? 'bg-accent text-accent-foreground shadow-sm hover:bg-accent/90 focus-visible:ring-offset-2 focus-visible:ring-offset-background'
                  : 'text-muted-foreground hover:bg-muted'}"
                onclick={() => (selectedSection = second.name)}
              >
                <IconComponent />

                <span class="truncate capitalize">{second.name}</span>
              </button>
            {/each}
          </div>
        </div>
      </div>
    </div>
    <!-- Main Content -->
    <div class="flex-1 overflow-auto">
      {#if selectedSection === "theme"}
        <ThemeComponent />
      {:else if selectedSection === "shortcuts"}
        <ShortcutsComponent />
      {:else if selectedSection === "editor"}
        <EditorComponent />
      {:else if selectedSection === "ai"}
        <AIComponent />
      {:else if selectedSection === "dangerous"}
        <DangerousComponent />
      {:else if selectedSection === "updates"}
        <UpdatesComponent />
      {/if}
    </div>
  </div>
</div>
