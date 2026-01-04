<script lang="ts">
    import { Handle, Position, type NodeProps } from "@xyflow/svelte";
    import IconKey from "@tabler/icons-svelte/icons/key";
    import IconTable from "@tabler/icons-svelte/icons/table";
    import IconLink from "@tabler/icons-svelte/icons/link";

    type TableNodeData = {
        label: string;
        columns: Array<{
            column_name: string;
            raw_type: string;
            is_primary_key: boolean;
            foreignKey?: boolean;
        }>;
    };

    export let data: TableNodeData;
</script>

<div
    class="flex flex-col min-w-[200px] bg-white border border-slate-200 rounded-md shadow-sm overflow-hidden text-slate-800 font-sans relative"
>
    <!-- Handles: Positioned slightly outside or on the edge -->
    <Handle
        type="target"
        position={Position.Left}
        class="!w-3 !h-3 !bg-slate-400 !-ml-1.5 border-2 border-white"
    />
    <Handle
        type="source"
        position={Position.Right}
        class="!w-3 !h-3 !bg-slate-400 !-mr-1.5 border-2 border-white"
    />

    <div
        class="flex items-center gap-2 px-3 py-2 bg-slate-50 border-b border-slate-100 font-semibold text-sm"
    >
        <IconTable class="size-4 text-slate-500" />
        <div class="flex-1 truncate">{data.label}</div>
    </div>

    <div class="flex flex-col text-sm bg-white">
        {#each data.columns as col}
            <div
                class="flex items-center justify-between px-3 py-1.5 border-b border-slate-100 last:border-0 hover:bg-slate-50 relative group transition-colors duration-150"
            >
                <div class="flex items-center gap-2 overflow-hidden flex-1">
                    {#if col.is_primary_key}
                        <IconKey class="size-3 text-amber-500 shrink-0" />
                    {:else if col.foreignKey}
                        <IconLink class="size-3 text-blue-500 shrink-0" />
                    {:else}
                        <div class="w-3 h-3 shrink-0"></div>
                    {/if}

                    <span
                        class="truncate font-medium text-slate-700"
                        title={col.column_name}
                    >
                        {col.column_name}
                    </span>
                </div>
                <span
                    class="text-[10px] uppercase tracking-wider text-slate-400 ml-3 shrink-0 font-medium"
                >
                    {col.raw_type}
                </span>
            </div>
        {/each}
    </div>
</div>
