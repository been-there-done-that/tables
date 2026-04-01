<script lang="ts">
    import { Handle, Position, NodeResizer } from '@xyflow/svelte';
    import type { ErdNodeData } from './erd-layout';
    import { COLUMN_ROW_HEIGHT, TABLE_HEADER_HEIGHT } from './erd-layout';
    import IconKey from '@tabler/icons-svelte/icons/key';
    import IconArrowUpRight from '@tabler/icons-svelte/icons/arrow-up-right';

    interface Props {
        data: ErdNodeData;
        selected?: boolean;
    }
    let { data, selected = false }: Props = $props();

    const table = $derived(data.table);
    const fkColumns = $derived(new Set(table.foreign_keys.map(fk => fk.column_name)));
</script>

<NodeResizer minWidth={200} maxWidth={500} isVisible={selected} />

<div
    class="rounded-md border border-[--theme-border-default] bg-[--theme-bg-secondary] text-[--theme-fg-primary] shadow-sm w-full"
    class:ring-2={selected}
    class:ring-primary={selected}
>
    <!-- Header: schema.table_name -->
    <div
        class="flex items-center border-b border-[--theme-border-default] bg-[--theme-bg-tertiary] px-2 font-semibold text-xs"
        style="height: {TABLE_HEADER_HEIGHT}px;"
    >
        <span class="truncate">
            <span class="text-[--theme-fg-secondary]">{table.schema}.</span>{table.table_name}
        </span>
    </div>

    <!-- Column rows -->
    <div class="overflow-hidden">
    {#each table.columns as col (col.column_name)}
        {@const isFk = fkColumns.has(col.column_name)}
        {@const isConnectable = col.is_primary_key || isFk}
        <div
            class="relative flex items-center gap-1 px-2 text-xs border-b border-[--theme-border-default]/50 last:border-0 hover:bg-[--theme-bg-hover]"
            style="height: {COLUMN_ROW_HEIGHT}px;"
        >
            {#if isConnectable}
                <!-- Source handle (right side) -->
                <Handle
                    type="source"
                    position={Position.Right}
                    id="{col.column_name}-source"
                    class="!w-2 !h-2 !bg-[--theme-border-default] !border-[--theme-border-default]"
                />
                <!-- Target handle (left side) — used by normal FK edges -->
                <Handle
                    type="target"
                    position={Position.Left}
                    id="{col.column_name}-target"
                    class="!w-2 !h-2 !bg-[--theme-border-default] !border-[--theme-border-default]"
                />
                <!-- Self-loop target handle (right side) — used only by self-referencing FK edges -->
                <Handle
                    type="target"
                    position={Position.Right}
                    id="{col.column_name}-self-target"
                    style="opacity: 0; pointer-events: none;"
                />
            {/if}

            <!-- PK badge -->
            {#if col.is_primary_key}
                <IconKey class="h-3 w-3 text-amber-500 shrink-0" />
                <span class="text-amber-500 shrink-0 font-bold text-[10px]">PK</span>
            {/if}

            <!-- FK badge (only when not also a PK) -->
            {#if isFk && !col.is_primary_key}
                <IconArrowUpRight class="h-3 w-3 text-blue-400 shrink-0" />
                <span class="text-blue-400 shrink-0 font-bold text-[10px]">FK</span>
            {/if}

            <!-- Spacer when no badges -->
            {#if !col.is_primary_key && !isFk}
                <span class="w-5 shrink-0"></span>
            {/if}

            <!-- Column name -->
            <span
                class="truncate flex-1"
                class:text-[--theme-fg-secondary]={!col.is_primary_key}
            >
                {col.column_name}
            </span>

            <!-- Nullable indicator -->
            {#if col.nullable}
                <span class="text-[--theme-fg-tertiary] shrink-0 text-[10px]" title="nullable">○</span>
            {/if}

            <!-- Type -->
            <span class="text-[--theme-fg-tertiary] text-[10px] shrink-0 font-mono ml-1">
                {col.raw_type.replace(/\(.*\)/, '')}
            </span>
        </div>
    {/each}
    </div>
</div>
