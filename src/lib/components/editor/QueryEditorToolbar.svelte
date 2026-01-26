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
    } from "@tabler/icons-svelte";
    import { schemaStore } from "$lib/stores/schema.svelte";

    interface Props {
        isRunning?: boolean;
        executionTime?: number;
        activeSchema?: string;
        onExecute: () => void;
        onStop: () => void;
        onFormat: () => void;
        onClear: () => void;
        onExplain: (raw: boolean) => void;
        onSchemaChange: (schema: string) => void;
    }

    let {
        isRunning = false,
        executionTime = 0,
        activeSchema = "public",
        onExecute,
        onStop,
        onFormat,
        onClear,
        onExplain,
        onSchemaChange,
    }: Props = $props();

    const currentSchemas = $derived.by(() => {
        const dbName = schemaStore.selectedDatabase;
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
                class="h-7 px-2 flex items-center gap-1.5 text-red-500 hover:bg-red-500/10 transition-colors"
                onclick={onStop}
                title="Stop Execution (Esc)"
            >
                <IconPlayerStopFilled class="size-4" />
                <span class="text-xs font-semibold">Stop</span>
            </Button>
        {:else}
            <Button
                variant="ghost"
                size="sm"
                class="h-7 px-3 flex items-center gap-1.5 hover:bg-green-500/10 hover:text-green-500 transition-colors"
                onclick={onExecute}
                title="Run (Cmd+Enter)"
            >
                <IconPlayerPlayFilled class="size-4 text-green-500" />
                <span class="text-xs font-semibold">Run</span>
            </Button>
        {/if}

        <div class="w-px h-4 bg-border/40 mx-1"></div>

        <!-- Format & Clear -->
        <Button
            variant="ghost"
            size="sm"
            class="h-7 px-2 flex items-center gap-1.5 opacity-70 hover:opacity-100"
            onclick={onFormat}
            title="Format SQL"
        >
            <IconCode class="size-4" />
            <span class="text-xs font-medium">Format</span>
        </Button>

        <Button
            variant="ghost"
            size="sm"
            class="h-7 px-2 flex items-center gap-1.5 opacity-70 hover:opacity-100 text-muted-foreground hover:text-foreground"
            onclick={onClear}
            title="Clear Editor"
        >
            <IconClearAll class="size-4" />
            <span class="text-xs font-medium">Clear</span>
        </Button>

        <div class="w-px h-4 bg-border/40 mx-1"></div>

        <!-- Explain Actions -->
        <Button
            variant="ghost"
            size="sm"
            class="h-7 px-2 flex items-center gap-1.5 opacity-70 hover:opacity-100"
            onclick={() => onExplain(false)}
            title="Explain Plan"
        >
            <IconSearch class="size-4" />
            <span class="text-xs font-medium">Explain</span>
        </Button>

        <Button
            variant="ghost"
            size="sm"
            class="h-7 px-2 flex items-center gap-1.5 opacity-70 hover:opacity-100"
            onclick={() => onExplain(true)}
            title="Explain Plan (Raw)"
        >
            <span class="text-xs font-bold opacity-70">RAW</span>
            <span class="text-xs font-medium">Explain</span>
        </Button>
    </div>

    <div class="flex items-center gap-3">
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
