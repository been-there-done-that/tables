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
        IconPin,
        IconRefresh,
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
        isPinned?: boolean;
        onPinColumn: () => void;
        onUnpinColumn: () => void;
        onResetWidth: () => void;
        onResetOrder: () => void;
        onResetAll: () => void;
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
        isPinned,
        onPinColumn,
        onUnpinColumn,
        onResetWidth,
        onResetOrder,
        onResetAll,
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

    <ContextMenuPrimitive.Separator />
    <ContextMenuPrimitive.Label>View & Layout</ContextMenuPrimitive.Label>
    {#if isPinned}
        <ContextMenuPrimitive.Item onclick={onUnpinColumn}>
            <IconPin class="mr-2 size-4 rotate-45" />
            Unpin Column
        </ContextMenuPrimitive.Item>
    {:else}
        <ContextMenuPrimitive.Item onclick={onPinColumn}>
            <IconPin class="mr-2 size-4" />
            Pin Column (Stay Left)
        </ContextMenuPrimitive.Item>
    {/if}
    <ContextMenuPrimitive.Item onclick={onResetWidth}>
        <IconRefresh class="mr-2 size-4" />
        Reset Column Width
    </ContextMenuPrimitive.Item>
    <ContextMenuPrimitive.Item onclick={onResetOrder}>
        <IconRefresh class="mr-2 size-4" />
        Reset Column Order
    </ContextMenuPrimitive.Item>
    <ContextMenuPrimitive.Item onclick={onResetAll} class="text-destructive">
        <IconX class="mr-2 size-4" />
        Reset All Customizations
    </ContextMenuPrimitive.Item>

    <!-- TODO: Add "Set Highlighting Language" if needed -->
</ContextMenuPrimitive.Content>
