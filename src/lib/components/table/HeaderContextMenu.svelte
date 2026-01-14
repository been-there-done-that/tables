<script lang="ts">
    import * as ContextMenuPrimitive from "$lib/components/ui/context-menu";
    import {
        IconCopy,
        IconEyeOff,
        IconEye,
        IconList,
        IconArrowUp,
        IconArrowDown,
        IconFilter,
        IconX,
        IconEdit,
        IconPlus,
        IconColumns,
    } from "@tabler/icons-svelte";
    import type { Column } from "./types";

    interface Props {
        column: Column;
        hiddenCount?: number;
        onCopyName: () => void;
        onCopyColumnData: () => void;
        onHideColumn: () => void;
        onHideOtherColumns: () => void;
        onUnhideAll: () => void;
        onShowColumnList: () => void;
        onSortAsc: () => void;
        onSortDesc: () => void;
        onClearSort: () => void;
        onSetLocalFilter: () => void;
        onOpenInQueryEditor?: () => void;
        onOpenNewQueryTab?: () => void;
    }

    let {
        column,
        hiddenCount = 0,
        onCopyName,
        onCopyColumnData,
        onHideColumn,
        onHideOtherColumns,
        onUnhideAll,
        onShowColumnList,
        onSortAsc,
        onSortDesc,
        onClearSort,
        onSetLocalFilter,
        onOpenInQueryEditor,
        onOpenNewQueryTab,
    }: Props = $props();
</script>

<ContextMenuPrimitive.Content class="w-56 text-xs">
    <ContextMenuPrimitive.Item
        onclick={onShowColumnList}
        class="text-xs py-1.5"
    >
        <IconList class="mr-1.5 size-3.5" />
        <span>Column List</span>
        <ContextMenuPrimitive.Shortcut>⌘F12</ContextMenuPrimitive.Shortcut>
    </ContextMenuPrimitive.Item>

    <ContextMenuPrimitive.Separator />

    <ContextMenuPrimitive.Item onclick={onCopyName} class="text-xs py-1.5">
        <IconCopy class="mr-1.5 size-3.5" />
        Copy Column Name
    </ContextMenuPrimitive.Item>
    <ContextMenuPrimitive.Item
        onclick={onCopyColumnData}
        class="text-xs py-1.5"
    >
        <IconColumns class="mr-1.5 size-3.5" />
        Copy Column Data
    </ContextMenuPrimitive.Item>

    <ContextMenuPrimitive.Separator />

    <ContextMenuPrimitive.Item onclick={onHideColumn} class="text-xs py-1.5">
        <IconEyeOff class="mr-1.5 size-3.5" />
        Hide Column
    </ContextMenuPrimitive.Item>
    <ContextMenuPrimitive.Item
        onclick={onHideOtherColumns}
        class="text-xs py-1.5"
    >
        <IconEyeOff class="mr-1.5 size-3.5" />
        Hide Other Columns
    </ContextMenuPrimitive.Item>
    {#if hiddenCount > 0}
        <ContextMenuPrimitive.Item onclick={onUnhideAll} class="text-xs py-1.5">
            <IconEye class="mr-1.5 size-3.5" />
            Unhide All ({hiddenCount})
        </ContextMenuPrimitive.Item>
    {/if}

    <ContextMenuPrimitive.Separator />

    <ContextMenuPrimitive.Label class="text-[10px] py-0.5"
        >ORDER BY</ContextMenuPrimitive.Label
    >
    <ContextMenuPrimitive.Item onclick={onSortAsc} class="text-xs py-1.5">
        <IconArrowUp class="mr-1.5 size-3.5" />
        Ascending
        <ContextMenuPrimitive.Shortcut>⌥↑</ContextMenuPrimitive.Shortcut>
    </ContextMenuPrimitive.Item>
    <ContextMenuPrimitive.Item onclick={onSortDesc} class="text-xs py-1.5">
        <IconArrowDown class="mr-1.5 size-3.5" />
        Descending
        <ContextMenuPrimitive.Shortcut>⌥↓</ContextMenuPrimitive.Shortcut>
    </ContextMenuPrimitive.Item>

    <ContextMenuPrimitive.Separator />

    <ContextMenuPrimitive.Item
        onclick={onSetLocalFilter}
        class="text-xs py-1.5"
    >
        <IconFilter class="mr-1.5 size-3.5" />
        Set Local Filter
    </ContextMenuPrimitive.Item>
    <ContextMenuPrimitive.Item onclick={onClearSort} class="text-xs py-1.5">
        <IconX class="mr-1.5 size-3.5" />
        Clear Sorting
        <ContextMenuPrimitive.Shortcut>⌥⇧⌘</ContextMenuPrimitive.Shortcut>
    </ContextMenuPrimitive.Item>

    <ContextMenuPrimitive.Separator />
    <ContextMenuPrimitive.Label class="text-[10px] py-0.5"
        >Query</ContextMenuPrimitive.Label
    >
    <ContextMenuPrimitive.Item
        onclick={onOpenInQueryEditor}
        class="text-xs py-1.5"
    >
        <IconEdit class="mr-1.5 size-3.5" />
        Open in Query Editor
    </ContextMenuPrimitive.Item>
    <ContextMenuPrimitive.Item
        onclick={onOpenNewQueryTab}
        class="text-xs py-1.5"
    >
        <IconPlus class="mr-1.5 size-3.5" />
        Open in New Query Tab
    </ContextMenuPrimitive.Item>
</ContextMenuPrimitive.Content>
