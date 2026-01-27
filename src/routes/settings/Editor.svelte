<script lang="ts">
    import { onMount } from "svelte";
    import { invoke } from "@tauri-apps/api/core";
    import { settingsStore } from "$lib/stores/settings.svelte";
    import SearchInput from "$lib/components/SearchInput.svelte";
    import { cn } from "$lib/utils";
    import IconDeviceFloppy from "@tabler/icons-svelte/icons/device-floppy";
    import IconRotate from "@tabler/icons-svelte/icons/rotate";
    import IconCheck from "@tabler/icons-svelte/icons/check";
    import IconLoader from "@tabler/icons-svelte/icons/loader-2";
    import { emit } from "@tauri-apps/api/event";
    import Checkbox from "$lib/components/ui/checkbox/checkbox.svelte";

    let fonts = $state<string[]>([]);
    let loading = $state(true);
    let searchQuery = $state("");
    let previewFont = $state(settingsStore.editorFontFamily);
    let showAllRunButtons = $state(settingsStore.editorShowAllRunButtons);

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
            // Ensure preview starts with saved value
            previewFont = settingsStore.editorFontFamily;
            showAllRunButtons = settingsStore.editorShowAllRunButtons;
            const systemFonts = await invoke<string[]>("get_system_fonts");
            fonts = systemFonts;
        } catch (e) {
            console.error("Failed to load fonts:", e);
        } finally {
            loading = false;
        }
    });

    function selectFont(font: string) {
        previewFont = font;
    }

    async function saveChanges() {
        settingsStore.editorFontFamily = previewFont;
        settingsStore.editorShowAllRunButtons = showAllRunButtons;
        // Emit event to notify other windows (e.g. main window) to update immediately
        await emit("font-changed", previewFont);
        await emit("settings-changed", [
            "editor_show_all_run_buttons",
            showAllRunButtons.toString(),
        ]);
    }

    function resetChanges() {
        previewFont = settingsStore.editorFontFamily;
        showAllRunButtons = settingsStore.editorShowAllRunButtons;
    }

    const hasChanges = $derived(
        previewFont !== settingsStore.editorFontFamily ||
            showAllRunButtons !== settingsStore.editorShowAllRunButtons,
    );
</script>

<div class="flex h-full w-full overflow-hidden">
    <!-- Left: Font List -->
    <div
        class="w-4/12 flex flex-col border-r border-border h-full bg-background shrink-0"
    >
        <div class="p-4 border-b border-border space-y-4">
            <div class="space-y-2">
                <div
                    class="text-xs font-semibold text-muted-foreground uppercase tracking-wider"
                >
                    Editor Behavior
                </div>
                <div class="flex items-center space-x-2 py-2">
                    <Checkbox
                        id="show-all-run"
                        bind:checked={showAllRunButtons}
                    />
                    <label
                        for="show-all-run"
                        class="text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70 cursor-pointer text-foreground"
                    >
                        Show run buttons for all queries
                    </label>
                </div>
            </div>

            <div class="border-t border-border pt-4 space-y-2">
                <div
                    class="text-xs font-semibold text-muted-foreground uppercase tracking-wider"
                >
                    Font Selection
                </div>
                <SearchInput
                    bind:value={searchQuery}
                    placeholder="Search fonts..."
                    class="w-full"
                />
            </div>
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
                    {@const isSelected = previewFont === font}
                    {@const isSaved = settingsStore.editorFontFamily === font}
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
                        <div class="flex items-center gap-2">
                            {#if isSaved}
                                <span
                                    class="text-[10px] bg-muted text-muted-foreground px-1.5 py-0.5 rounded border border-border"
                                >
                                    Current
                                </span>
                            {/if}
                            {#if isSelected}
                                <IconCheck
                                    class="size-4 shrink-0 text-primary opacity-60"
                                />
                            {/if}
                        </div>
                    </button>
                {/each}
            {/if}
        </div>
    </div>

    <!-- Right: Preview -->
    <div class="flex-1 flex flex-col h-full bg-muted/10 overflow-hidden">
        <div class="flex-1 p-6 overflow-hidden flex flex-col gap-6">
            <!-- Code Editor Preview -->
            <div class="flex-1 min-h-0 flex flex-col gap-2">
                <div
                    class="text-xs font-medium text-muted-foreground uppercase tracking-wider px-1"
                >
                    Editor Preview
                </div>
                <FontPreviewEditor fontFamily={previewFont} />
            </div>

            <!-- Table Preview (Reduced Size) -->
            <div class="flex-none flex flex-col gap-2">
                <div
                    class="text-xs font-medium text-muted-foreground uppercase tracking-wider px-1"
                >
                    Table Preview
                </div>
                <FontPreviewTable fontFamily={previewFont} />
            </div>
        </div>

        <!-- Footer Actions -->
        <div
            class="p-4 border-t border-border bg-background flex items-center justify-between"
        >
            <div class="flex items-center gap-2 text-sm text-muted-foreground">
                <span class="font-medium text-foreground">{previewFont}</span>
                {#if hasChanges}
                    <span class="text-xs text-amber-500 font-medium"
                        >(Unsaved Changes)</span
                    >
                {/if}
            </div>

            <div class="flex items-center gap-3">
                <button
                    class="flex items-center gap-2 px-4 py-2 text-sm font-medium text-muted-foreground hover:text-foreground hover:bg-muted rounded-md transition-colors"
                    onclick={resetChanges}
                    disabled={!hasChanges}
                    class:opacity-50={!hasChanges}
                >
                    <IconRotate class="size-4" />
                    Reset
                </button>

                <button
                    class="flex items-center gap-2 px-4 py-2 text-sm font-medium bg-primary text-primary-foreground hover:bg-primary/90 rounded-md transition-colors shadow-sm"
                    onclick={saveChanges}
                    disabled={!hasChanges}
                    class:opacity-50={!hasChanges}
                >
                    <IconDeviceFloppy class="size-4" />
                    Save & Apply
                </button>
            </div>
        </div>
    </div>
</div>
