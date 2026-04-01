<script lang="ts">
    import { Handle, Position } from '@xyflow/svelte';
    import type { ErdNodeData } from './erd-layout';
    import { TABLE_NODE_WIDTH, COLUMN_ROW_HEIGHT, TABLE_HEADER_HEIGHT } from './erd-layout';

    interface Props {
        data: ErdNodeData;
        selected?: boolean;
    }
    let { data, selected = false }: Props = $props();

    const table = $derived(data.table);
</script>

<div
    class="rounded-md border bg-card text-card-foreground shadow-sm overflow-hidden"
    class:ring-2={selected}
    class:ring-primary={selected}
    style="width: {TABLE_NODE_WIDTH}px;"
>
    <!-- Header -->
    <div
        class="flex items-center gap-1.5 border-b border-border bg-muted/50 px-2 font-semibold text-xs text-foreground"
        style="height: {TABLE_HEADER_HEIGHT}px;"
    >
        <span class="truncate">{table.table_name}</span>
        {#if table.schema !== 'public'}
            <span class="ml-auto text-muted-foreground shrink-0">{table.schema}</span>
        {/if}
    </div>

    <!-- Column rows -->
    {#each table.columns as col (col.column_name)}
        <div
            class="relative flex items-center gap-1 px-2 text-xs border-b border-border/50 last:border-0 hover:bg-muted/30"
            style="height: {COLUMN_ROW_HEIGHT}px;"
        >
            <!-- Source handle (right side) -->
            <Handle
                type="source"
                position={Position.Right}
                id="{col.column_name}-source"
                class="!w-2 !h-2 !bg-border !border-border"
            />
            <!-- Target handle (left side) -->
            <Handle
                type="target"
                position={Position.Left}
                id="{col.column_name}-target"
                class="!w-2 !h-2 !bg-border !border-border"
            />

            <!-- PK badge -->
            {#if col.is_primary_key}
                <span class="text-amber-500 shrink-0 font-bold text-[10px]">PK</span>
            {:else}
                <span class="w-[18px] shrink-0"></span>
            {/if}

            <!-- Column name -->
            <span class="truncate flex-1" class:text-muted-foreground={!col.is_primary_key}>
                {col.column_name}
            </span>

            <!-- Nullable dot -->
            {#if col.nullable}
                <span class="text-muted-foreground/50 shrink-0" title="nullable">○</span>
            {/if}

            <!-- Type -->
            <span class="text-muted-foreground/70 text-[10px] shrink-0 font-mono ml-1">
                {col.raw_type.replace(/\(.*\)/, '')}
            </span>
        </div>
    {/each}
</div>
