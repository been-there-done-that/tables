<script lang="ts">
    import { getThemeContext } from "$lib/theme/context";
    import type { ThemeRecord } from "$lib/theme/types";

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
                class="group flex w-72 flex-col gap-3 rounded-xl border px-4 py-3 transition duration-150"
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
                        <div
                            class="font-semibold"
                            style="color: var(--theme-fg-primary);"
                        >
                            {theme.name}
                        </div>
                        <div
                            class="text-xs"
                            style="color: var(--theme-fg-secondary);"
                        >
                            {theme.author}
                        </div>
                    </div>
                    {#if theme.id === activeId}
                        <div
                            class="inline-flex items-center gap-2 rounded-lg border px-3 py-1 text-xs"
                            style="border-color: var(--theme-border-default); background: color-mix(in srgb, var(--theme-bg-tertiary) 75%, transparent); color: var(--theme-fg-secondary);"
                        >
                            Active
                        </div>
                    {/if}
                </div>
                {#if theme.description}
                    <div
                        class="text-sm"
                        style="color: var(--theme-fg-secondary); margin-top: 6px;"
                    >
                        {theme.description}
                    </div>
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
                        <span
                            class="h-4 w-4 rounded-md border"
                            style={`background:${color}; border-color: var(--theme-border-subtle);`}
                        ></span>
                    {/each}
                </div>
            </div>
        {/each}
    </div>
