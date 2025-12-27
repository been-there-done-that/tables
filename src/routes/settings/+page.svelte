<script lang="ts">
  import BrushIcon from "@tabler/icons-svelte/icons/brush";
  import { getThemeContext } from "$lib/theme/context";
  import type { ThemeRecord } from "$lib/theme/types";
  import ThemeComponent from "./Theme.svelte";

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
  ];
</script>

<div
  class="flex flex-col w-full h-full bg-[--theme-bg-primary] overflow-hidden"
>
  <div class="flex grow w-full overflow-hidden">
    <!-- Sidebar -->
    <div
      class="max-w-60 w-full shrink-0 h-full border-r border-(--theme-border-default)"
    >
      <div
        class="flex flex-col h-full bg-(--theme-bg-) border-r border-(--theme-border-default)"
      >
        <div class="p-3 flex w-full flex-col mt-24">
          <div class="space-y-0.5">
            {#each sections as second}
              {@const IconComponent = second.icon}
              <button
                class="w-full text-left px-3 py-1.5 flex items-center space-x-2 text-sm rounded-md
                    {selectedSection === second.name
                  ? 'bg-(--theme-accent-primary) text-white [text-shadow:0_1px_2px_rgba(0,0,0,0.45)] hover:bg-[color-mix(in_srgb,var(--theme-accent-primary)_78%,black_22%)] focus-visible:ring-offset-2 focus-visible:ring-offset-(--theme-bg-primary)'
                  : 'text-(--theme-fg-secondary) hover:bg-(--theme-bg-hover)'}"
                onclick={() => (selectedSection = second.name)}
              >
                <IconComponent />

                <span class="truncate">{second.name}</span>
              </button>
            {/each}
          </div>
        </div>
      </div>
    </div>
    <!-- Main Content -->
    <div class="">
      <ThemeComponent />
    </div>
  </div>
</div>
