<script lang="ts">
    import { getThemeContext } from "$lib/theme/context";
    import type { ThemeRecord } from "$lib/theme/types";
    import ThemePreview from "$lib/components/ThemePreview.svelte";

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
</script>

<!-- Main Content -->
<div class="flex flex-wrap overflow-scroll h-full gap-3 justify-center py-12">
    {#each themes as theme}
        <div
            role="button"
            tabindex="0"
            class="group flex w-72 flex-col gap-3 rounded-xl p-2 transition duration-150"
            style={`box-shadow: ${
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
            <div class="mt-1">
                <ThemePreview {theme} />
            </div>
            <div class="text-center text-sm">{theme.name}</div>
        </div>
    {/each}
</div>
