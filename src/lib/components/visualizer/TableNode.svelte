<script lang="ts">
    import { Handle, Position, type NodeProps } from "@xyflow/svelte";
    import { KeyRound, Table, Columns } from "lucide-svelte";

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
    class="shadow-md rounded-md bg-white border-2 border-slate-200 min-w-[200px] overflow-hidden"
>
    <div
        class="custom-drag-handle bg-slate-100 border-b border-slate-200 px-3 py-2 flex items-center gap-2 font-bold text-slate-700"
    >
        <!-- Icon for Table -->
        <Table size={16} class="text-slate-500" />
        <div class="flex-1 truncate">{data.label}</div>
    </div>

    <div class="flex flex-col text-sm bg-white">
        {#each data.columns as col}
            <div
                class="flex items-center justify-between px-3 py-1.5 border-b border-slate-100 last:border-0 hover:bg-slate-50 relative group transition-colors duration-150"
            >
                <div class="flex items-center gap-2 overflow-hidden flex-1">
                    {#if col.is_primary_key}
                        <KeyRound size={12} class="text-amber-500 shrink-0" />
                    {:else if col.foreignKey}
                        <Columns size={12} class="text-blue-500 shrink-0" />
                        <!-- Placeholder for FK icon -->
                    {:else}
                        <div class="w-3 h-3 shrink-0"></div>
                        <!-- Spacer -->
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

                <!-- Handles for edges. 
             We arguably want handles on every column for precise FK connections, 
             or just left/right of the node. 
             For simplicity in v1, let's keep one generic handle or handles per column if needed.
             Let's try handles on the node level first for generic connections, 
             or specific handles if we can map them.
             
             Actually, for Schema visualization, typically you want edges to point to specific columns.
             But xyflow handles are by default on the node borders.
             To map to a specific row (column), we'd need handles positioned dynamically.
             For now, let's use standard node-level handles (Source Right, Target Left) 
             to avoid complex absolute positioning of handles inside the list.
         -->
            </div>
        {/each}
    </div>

    <Handle
        type="target"
        position={Position.Left}
        class="w-2 h-2 !bg-slate-400"
    />
    <Handle
        type="source"
        position={Position.Right}
        class="w-2 h-2 !bg-slate-400"
    />
</div>
