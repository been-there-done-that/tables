<script lang="ts">
    import * as ContextMenuPrimitive from "$lib/components/ui/context-menu";
    import {
        Copy,
        EyeOff,
        List,
        ArrowUp,
        ArrowDown,
        Filter,
        X,
        Check,
        FilePenLine,
        FilePlus2,
    } from "lucide-svelte";
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
        <Copy class="mr-2 size-4" />
        Copy Column Name
    </ContextMenuPrimitive.Item>
    <ContextMenuPrimitive.Item onclick={onSelectColumn}>
        <Check class="mr-2 size-4" />
        Select Column
    </ContextMenuPrimitive.Item>

    <ContextMenuPrimitive.Separator />

    <ContextMenuPrimitive.Item onclick={onHideColumn}>
        <EyeOff class="mr-2 size-4" />
        Hide Column
    </ContextMenuPrimitive.Item>
    <ContextMenuPrimitive.Item onclick={onHideOtherColumns}>
        <EyeOff class="mr-2 size-4" />
        Hide Other Columns
    </ContextMenuPrimitive.Item>
    <ContextMenuPrimitive.Item onclick={onShowColumnList}>
        <List class="mr-2 size-4" />
        Show Column List
        <ContextMenuPrimitive.Shortcut>⌘F12</ContextMenuPrimitive.Shortcut>
    </ContextMenuPrimitive.Item>

    <ContextMenuPrimitive.Separator />

    <ContextMenuPrimitive.Label>ORDER BY</ContextMenuPrimitive.Label>
    <ContextMenuPrimitive.Item onclick={onSortAsc}>
        <ArrowUp class="mr-2 size-4" />
        Ascending
        <ContextMenuPrimitive.Shortcut>⌥↑</ContextMenuPrimitive.Shortcut>
    </ContextMenuPrimitive.Item>
    <ContextMenuPrimitive.Item onclick={onSortDesc}>
        <ArrowDown class="mr-2 size-4" />
        Descending
        <ContextMenuPrimitive.Shortcut>⌥↓</ContextMenuPrimitive.Shortcut>
    </ContextMenuPrimitive.Item>

    <ContextMenuPrimitive.Separator />

    <ContextMenuPrimitive.Item onclick={onSetLocalFilter}>
        <Filter class="mr-2 size-4" />
        Set Local Filter
    </ContextMenuPrimitive.Item>
    <ContextMenuPrimitive.Item onclick={onClearSort}>
        <X class="mr-2 size-4" />
        Clear Sorting
        <ContextMenuPrimitive.Shortcut>⌥⇧⌘</ContextMenuPrimitive.Shortcut>
    </ContextMenuPrimitive.Item>

    <ContextMenuPrimitive.Separator />
    <ContextMenuPrimitive.Label>Query</ContextMenuPrimitive.Label>
    <ContextMenuPrimitive.Item onclick={onOpenInQueryEditor}>
        <FilePenLine class="mr-2 size-4" />
        Open in Query Editor
    </ContextMenuPrimitive.Item>
    <ContextMenuPrimitive.Item onclick={onOpenNewQueryTab}>
        <FilePlus2 class="mr-2 size-4" />
        Open in New Query Tab
    </ContextMenuPrimitive.Item>

    <!-- TODO: Add "Set Highlighting Language" if needed -->
</ContextMenuPrimitive.Content>
