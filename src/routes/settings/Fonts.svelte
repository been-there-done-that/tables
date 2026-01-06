<script lang="ts">
    import { onMount } from "svelte";
    import { invoke } from "@tauri-apps/api/core";
    import { settingsStore } from "$lib/stores/settings.svelte";
    import SearchInput from "$lib/components/SearchInput.svelte";
    import { cn } from "$lib/utils";
    import IconCheck from "@tabler/icons-svelte/icons/check";
    import IconLoader from "@tabler/icons-svelte/icons/loader-2";

    let fonts = $state<string[]>([]);
    let loading = $state(true);
    let searchQuery = $state("");
    import FontPreviewEditor from "./FontPreviewEditor.svelte";
    import FontPreviewTable from "./FontPreviewTable.svelte";

    const filteredFonts = $derived(
        fonts.filter((f) =>
            f.toLowerCase().includes(searchQuery.toLowerCase()),
        ),
    );

    onMount(async () => {
        try {
            loading = true;
            const systemFonts = await invoke<string[]>("get_system_fonts");
            fonts = systemFonts;
        } catch (e) {
            console.error("Failed to load fonts:", e);
        } finally {
            loading = false;
        }
    });

    function selectFont(font: string) {
        settingsStore.editorFontFamily = font;
    }
</script>

<div class="flex h-full w-full overflow-hidden">
    <!-- Left: Font List -->
    <div
        class="w-4/12 flex flex-col border-r border-border h-full bg-background shrink-0"
    >
        <div class="p-4 border-b border-border">
            <SearchInput
                bind:value={searchQuery}
                placeholder="Search fonts..."
                class="w-full"
            />
        </div>

        <div class="flex-1 overflow-y-auto p-2 space-y-1">
            {#if loading}
                <div
                    class="flex items-center justify-center h-20 text-muted-foreground"
                >
                    <IconLoader class="size-5 animate-spin mr-2" />
                    Loading fonts...
                </div>
            {:else if filteredFonts.length === 0}
                <div class="text-center p-4 text-muted-foreground text-sm">
                    No fonts found.
                </div>
            {:else}
                {#each filteredFonts as font}
                    {@const isSelected =
                        settingsStore.editorFontFamily === font}
                    <button
                        class={cn(
                            "w-full text-left px-3 py-2 text-sm rounded-md flex items-center justify-between group transition-colors",
                            isSelected
                                ? "bg-primary/10 text-foreground"
                                : "hover:bg-muted/50 text-foreground/80 hover:text-foreground",
                        )}
                        onclick={() => selectFont(font)}
                    >
                        <span class="truncate pr-2" style:font-family={font}
                            >{font}</span
                        >
                        {#if isSelected}
                            <IconCheck
                                class="size-4 shrink-0 text-primary opacity-60"
                            />
                        {/if}
                    </button>
                {/each}
            {/if}
        </div>
    </div>

    <!-- Right: Preview -->
    <div class="flex-1 flex flex-col h-full bg-muted/10 overflow-hidden">
        <div
            class="p-6 flex-col border-b border-border bg-background flex items-center justify-between gap-4 shrink-0"
        >
            <div class="flex justify-start items-center gap-2 w-full">
                <h2 class="text-lg font-medium">Font Preview:</h2>
                <p class="text-sm text-muted-foreground">
                    {settingsStore.editorFontFamily}
                </p>
            </div>

            <div class="flex items-center gap-2 w-full">
                <label
                    for="font-size"
                    class="text-sm font-medium whitespace-nowrap">Size:</label
                >
                <input
                    id="font-size"
                    type="range"
                    min="10"
                    max="24"
                    step="1"
                    bind:value={settingsStore.editorFontSize}
                    class="w-full"
                />
                <span class="text-sm font-mono w-8 text-right"
                    >{settingsStore.editorFontSize}</span
                >
            </div>
        </div>

        <div class="flex-1 p-6 overflow-hidden flex flex-col gap-6">
            <!-- Code Editor Preview -->
            <div class="flex-1 min-h-[300px] flex flex-col gap-2">
                <div
                    class="text-xs font-medium text-muted-foreground uppercase tracking-wider px-1"
                >
                    Editor Preview
                </div>
                <FontPreviewEditor />
            </div>

            <!-- Table Preview -->
            <div class="flex-1 min-h-[300px] flex flex-col gap-2">
                <div
                    class="text-xs font-medium text-muted-foreground uppercase tracking-wider px-1"
                >
                    Table Preview
                </div>
                <FontPreviewTable />
            </div>
        </div>
    </div>
</div>
