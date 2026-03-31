<script lang="ts">
    import { Button } from "$lib/components/ui/button";
    import * as DropdownMenu from "$lib/components/ui/dropdown-menu";
    import { cn } from "$lib/utils";
    import {
        IconPlayerPlayFilled,
        IconPlayerStopFilled,
        IconRefresh,
        IconClearAll,
        IconCode,
        IconSearch,
        IconChevronDown,
        IconSchema,
        IconStopwatch,
        IconLoader2,
        IconArrowBackUp,
        IconArrowForwardUp,
        IconLock,
    } from "@tabler/icons-svelte";
    import IconBolt from "@tabler/icons-svelte/icons/bolt";
    import { schemaStore } from "$lib/stores/schema.svelte";

    interface Props {
        isRunning?: boolean;
        executionTime?: number;
        activeSchema?: string;
        viewData?: any; // View data containing databaseContext
        showAll?: boolean;
        isReadOnly?: boolean;
        onToggleShowAll?: () => void;
        onExecute: () => void;
        onStop: () => void;
        onFormat: () => void;
        onClear: () => void;
        onUndo: () => void;
        onRedo: () => void;
        onExplain: (raw: boolean) => void;
        onSchemaChange: (schema: string) => void;
    }

    let {
        isRunning = false,
        executionTime = 0,
        activeSchema = "public",
        viewData,
        showAll,
        isReadOnly = false,
        onToggleShowAll,
        onExecute,
        onStop,
        onFormat,
        onClear,
        onUndo,
        onRedo,
        onExplain,
        onSchemaChange,
    }: Props = $props();

    // Use view-specific database context if available, otherwise fall back to global
    const effectiveDatabase = $derived.by(() => {
        return viewData?.databaseContext || schemaStore.selectedDatabase;
    });

    const currentSchemas = $derived.by(() => {
        const dbName = effectiveDatabase;
        if (!dbName) return [];
        const db = schemaStore.databases.find((d) => d.name === dbName);
        return db?.schemas || [];
    });
</script>

<div
    class="flex h-8 items-center justify-between border-b border-border bg-muted/20 px-2 gap-2"
>
    <div class="flex items-center gap-1">
        <!-- Main Actions -->
        {#if isRunning}
            <Button
                variant="ghost"
                size="sm"
                class="h-6 px-2.5 flex items-center gap-1 rounded-full text-red-500 hover:bg-red-500/10 shadow-sm transition-colors"
                onclick={onStop}
                title="Stop Execution (Esc)"
            >
                <IconPlayerStopFilled class="size-3.5" />
                <span class="text-[11px] font-semibold">Stop</span>
            </Button>
        {:else}
            <Button
                variant="ghost"
                size="sm"
                class="h-6 px-2.5 flex items-center gap-1 rounded-full bg-green-500/10 text-green-600 hover:bg-green-500/20 hover:text-green-500 shadow-sm transition-colors"
                onclick={onExecute}
                title="Run (Cmd+↵)"
            >
                <IconPlayerPlayFilled class="size-3.5 text-green-500" />
                <span class="text-[11px] font-semibold">Run</span>
            </Button>
        {/if}

        <Button
            variant="ghost"
            size="sm"
            class="h-6 px-2 flex items-center gap-1 rounded-md text-orange-400/80 hover:text-orange-400 hover:bg-orange-500/10 shadow-sm transition-colors"
            onclick={() => onExplain(false)}
            title="Explain Query (Cmd+Shift+E)"
        >
            <IconBolt class="size-3.5" />
            <span class="text-[11px] font-medium">Explain</span>
        </Button>

        <div class="w-px h-4 bg-border/40 mx-1"></div>

        <!-- Format & Clear -->
        <Button
            variant="ghost"
            size="sm"
            class="h-6 px-2 flex items-center gap-1 rounded-md opacity-70 hover:opacity-100 shadow-sm"
            onclick={onFormat}
            title="Format SQL (Shift+Alt+F)"
        >
            <IconCode class="size-3.5" />
            <span class="text-[11px] font-medium">Format</span>
        </Button>

        <Button
            variant="ghost"
            size="sm"
            class="h-6 px-2 flex items-center gap-1 rounded-md opacity-70 hover:opacity-100 text-muted-foreground hover:text-foreground shadow-sm"
            onclick={onClear}
            title="Clear Editor"
        >
            <IconClearAll class="size-3.5" />
            <span class="text-[11px] font-medium">Clear</span>
        </Button>

        <div class="w-px h-4 bg-border/40 mx-1"></div>

        <!-- Undo/Redo -->
        <Button
            variant="ghost"
            size="sm"
            class="h-6 w-6 p-0 flex items-center justify-center rounded-md opacity-70 hover:opacity-100"
            onclick={onUndo}
            title="Undo (Cmd+Z)"
        >
            <IconArrowBackUp class="size-3.5" />
        </Button>

        <Button
            variant="ghost"
            size="sm"
            class="h-6 w-6 p-0 flex items-center justify-center rounded-md opacity-70 hover:opacity-100"
            onclick={onRedo}
            title="Redo (Cmd+Shift+Z)"
        >
            <IconArrowForwardUp class="size-3.5" />
        </Button>
    </div>

    <div class="flex items-center gap-2">
        <!-- Execution Time -->
        {#if executionTime > 0 || isRunning}
            <div
                class="flex items-center gap-1.5 text-[10px] text-muted-foreground font-mono tabular-nums"
            >
                {#if isRunning}
                    <IconLoader2 class="size-3 animate-spin" />
                    <span>Running...</span>
                {:else}
                    <IconStopwatch class="size-3 opacity-70" />
                    <span>
                        {executionTime < 1000
                            ? `${executionTime.toFixed(0)}ms`
                            : `${(executionTime / 1000).toFixed(2)}s`}
                    </span>
                {/if}
            </div>
            <div class="w-px h-4 bg-border/40"></div>
        {/if}

        <!-- Read-only lock indicator -->
        {#if isReadOnly}
            <div
                class="flex items-center gap-1 text-[10px] text-muted-foreground/70"
                title="Read-only — DDL view"
            >
                <IconLock class="size-3" />
            </div>
        {/if}

        <!-- Schema Picker -->
        <DropdownMenu.Root>
            <DropdownMenu.Trigger
                class="flex items-center gap-1.5 rounded border border-border bg-background px-2.5 py-1 text-[11px] font-medium hover:bg-accent hover:text-accent-foreground transition-colors"
                title="Select Schema"
            >
                <IconSchema class="size-3.5 text-muted-foreground" />
                <span class="truncate max-w-[120px]">{activeSchema}</span>
                <IconChevronDown class="size-3 text-muted-foreground/50" />
            </DropdownMenu.Trigger>
            <DropdownMenu.Content
                align="end"
                class="min-w-[120px] w-max max-w-[300px] max-h-[300px] overflow-auto"
            >
                <DropdownMenu.Label class="text-[10px] uppercase opacity-50"
                    >Schemas</DropdownMenu.Label
                >
                <DropdownMenu.Separator />
                <DropdownMenu.RadioGroup
                    value={activeSchema}
                    onValueChange={onSchemaChange}
                >
                    {#each currentSchemas as schema (schema.name)}
                        <DropdownMenu.RadioItem
                            value={schema.name}
                            class="text-xs"
                        >
                            {schema.name}
                        </DropdownMenu.RadioItem>
                    {/each}
                </DropdownMenu.RadioGroup>
            </DropdownMenu.Content>
        </DropdownMenu.Root>
    </div>
</div>
