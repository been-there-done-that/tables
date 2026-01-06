<script lang="ts">
  import BrushIcon from "@tabler/icons-svelte/icons/brush";
  import { getThemeContext } from "$lib/theme/context";
  import type { ThemeRecord } from "$lib/theme/types";
  import ThemeComponent from "./Theme.svelte";
  import ShortcutsComponent from "./Shortcuts.svelte";
  import KeyboardIcon from "@tabler/icons-svelte/icons/keyboard";

  import TypographyIcon from "@tabler/icons-svelte/icons/typography";
  import FontsComponent from "./Fonts.svelte";

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

  let selectedSection = $state<string>("theme");

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
      name: "fonts",
      icon: TypographyIcon,
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
      {:else if selectedSection === "fonts"}
        <FontsComponent />
      {/if}
    </div>
  </div>
</div>
