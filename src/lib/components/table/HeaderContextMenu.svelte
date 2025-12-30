<script lang="ts">
    import * as ContextMenuPrimitive from "$lib/components/ui/context-menu";
    import {
        IconCopy,
        IconEyeOff,
        IconList,
        IconArrowUp,
        IconArrowDown,
        IconFilter,
        IconX,
        IconCheck,
        IconEdit,
        IconPlus,
    } from "@tabler/icons-svelte";
    import type { Column } from "./types";

    interface Props {
        column: Column;
        onCopyName: () => void;
        onSelectColumn: () => void;
        onHideColumn: () => void;
        onHideOtherColumns: () => void;
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
        onCopyName,
        onSelectColumn,
        onHideColumn,
        onHideOtherColumns,
        onShowColumnList,
        onSortAsc,
        onSortDesc,
        onClearSort,
        onSetLocalFilter,
        onOpenInQueryEditor,
        onOpenNewQueryTab,
    }: Props = $props();
</script>

<ContextMenuPrimitive.Content class="w-64">
    <ContextMenuPrimitive.Item onclick={onCopyName}>
        <IconCopy class="mr-2 size-4" />
        Copy Column Name
    </ContextMenuPrimitive.Item>
    <ContextMenuPrimitive.Item onclick={onSelectColumn}>
        <IconCheck class="mr-2 size-4" />
        Select Column
    </ContextMenuPrimitive.Item>

    <ContextMenuPrimitive.Separator />

    <ContextMenuPrimitive.Item onclick={onHideColumn}>
        <IconEyeOff class="mr-2 size-4" />
        Hide Column
    </ContextMenuPrimitive.Item>
    <ContextMenuPrimitive.Item onclick={onHideOtherColumns}>
        <IconEyeOff class="mr-2 size-4" />
        Hide Other Columns
    </ContextMenuPrimitive.Item>
    <ContextMenuPrimitive.Item onclick={onShowColumnList}>
        <IconList class="mr-2 size-4" />
        <span>Show Column List</span>
        <ContextMenuPrimitive.Shortcut>⌘F12</ContextMenuPrimitive.Shortcut>
    </ContextMenuPrimitive.Item>

    <ContextMenuPrimitive.Separator />

    <ContextMenuPrimitive.Label>ORDER BY</ContextMenuPrimitive.Label>
    <ContextMenuPrimitive.Item onclick={onSortAsc}>
        <IconArrowUp class="mr-2 size-4" />
        Ascending
        <ContextMenuPrimitive.Shortcut>⌥↑</ContextMenuPrimitive.Shortcut>
    </ContextMenuPrimitive.Item>
    <ContextMenuPrimitive.Item onclick={onSortDesc}>
        <IconArrowDown class="mr-2 size-4" />
        Descending
        <ContextMenuPrimitive.Shortcut>⌥↓</ContextMenuPrimitive.Shortcut>
    </ContextMenuPrimitive.Item>

    <ContextMenuPrimitive.Separator />

    <ContextMenuPrimitive.Item onclick={onSetLocalFilter}>
        <IconFilter class="mr-2 size-4" />
        Set Local Filter
    </ContextMenuPrimitive.Item>
    <ContextMenuPrimitive.Item onclick={onClearSort}>
        <IconX class="mr-2 size-4" />
        Clear Sorting
        <ContextMenuPrimitive.Shortcut>⌥⇧⌘</ContextMenuPrimitive.Shortcut>
    </ContextMenuPrimitive.Item>

    <ContextMenuPrimitive.Separator />
    <ContextMenuPrimitive.Label>Query</ContextMenuPrimitive.Label>
    <ContextMenuPrimitive.Item onclick={onOpenInQueryEditor}>
        <IconEdit class="mr-2 size-4" />
        Open in Query Editor
    </ContextMenuPrimitive.Item>
    <ContextMenuPrimitive.Item onclick={onOpenNewQueryTab}>
        <IconPlus class="mr-2 size-4" />
        Open in New Query Tab
    </ContextMenuPrimitive.Item>

    <!-- TODO: Add "Set Highlighting Language" if needed -->
</ContextMenuPrimitive.Content>
