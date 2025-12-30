<script lang="ts">
    import type { Column } from "./types";
    import { onMount } from "svelte";
    import BooleanPopoverEditor from "./BooleanPopoverEditor.svelte";
    import EnumPopoverEditor from "./EnumPopoverEditor.svelte";
    import JsonPopoverEditor from "./JsonPopoverEditor.svelte";
    import DateTimePopoverEditor from "./DateTimePopoverEditor.svelte";
    import NumberPopoverEditor from "./NumberPopoverEditor.svelte";
    import TextPopoverEditor from "./TextPopoverEditor.svelte";

    interface Props {
        value: any;
        column: Column;
        onCommit: (newValue: any) => void;
        onCancel: () => void;
        anchorEl?: HTMLElement | null;
    }

    let { value, column, onCommit, onCancel, anchorEl }: Props = $props();

    // Local editable value (synchronised via effect)
    let inputValue = $state<any>();

    // Keep in sync if parent updates value while editor is open
    $effect(() => {
        inputValue = value;
    });
    let inputRef = $state<
        HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement | undefined
    >();

    onMount(() => {
        if (
            inputRef instanceof HTMLInputElement ||
            inputRef instanceof HTMLTextAreaElement ||
            inputRef instanceof HTMLSelectElement
        ) {
            inputRef.focus();
        }

        console.info("[CellEditor] mounted", {
            column: column.id,
            type: column.type,
            value,
            anchorPresent: Boolean(anchorEl),
        });
    });

    function handleKeydown(e: KeyboardEvent) {
        if (e.key === "Enter" && column.type !== "json" && column.type !== "JSON") {
            onCommit(processValue(inputValue));
        } else if (e.key === "Escape") {
            onCancel();
        }
    }

    function handleBlur() {
        // Don't commit on blur for modal types
        if (column.type !== "json" && column.type !== "JSON") {
            onCommit(processValue(inputValue));
        }
    }

    function processValue(val: any): any {
        if (column.type === "int") return parseInt(val);
        if (column.type === "float") return parseFloat(val);
        if (column.type === "boolean") return val === "true" || val === true;
        return val;
    }

    function isNumberType(t: Column["type"]): t is "int" | "float" {
        return t === "int" || t === "float";
    }

    const numberKind = $derived(isNumberType(column.type) ? column.type : "int");
</script>

<div
    class="w-full h-full"
    onmousedown={(e) => e.stopPropagation()}
    onclick={(e) => e.stopPropagation()}
    role="presentation"
>
    {#if column.type === "json" || column.type === "JSON"}
        <JsonPopoverEditor
            {value}
            {anchorEl}
            {onCommit}
            {onCancel}
        />
    {:else if column.type === "boolean"}
        <BooleanPopoverEditor
            {value}
            {anchorEl}
            {onCommit}
            {onCancel}
        />
    {:else if column.type === "enum" && column.enumValues}
        <EnumPopoverEditor
            value={inputValue}
            options={column.enumValues}
            {anchorEl}
            onCommit={(v) => onCommit(v)}
            {onCancel}
        />
    {:else if column.type === "text"}
        <TextPopoverEditor
            value={inputValue}
            {anchorEl}
            onCommit={(v: any) => onCommit(processValue(v))}
            {onCancel}
        />
    {:else if isNumberType(column.type)}
        <NumberPopoverEditor
            value={inputValue}
            kind={numberKind}
            {anchorEl}
            onCommit={(v: any) => onCommit(processValue(v))}
            {onCancel}
        />
    {:else if column.type === "date" || column.type === "datetime"}
        <DateTimePopoverEditor
            value={inputValue}
            mode={column.type === "datetime" ? "datetime" : "date"}
            {anchorEl}
            {onCommit}
            {onCancel}
        />
    {:else}
        <!-- Text / int / float / fallback -->
        <input
            bind:this={inputRef}
            bind:value={inputValue}
            type="text"
            class="h-full w-full rounded-none border-0 px-2 py-1 focus-visible:ring-0 focus-visible:outline-none bg-background text-foreground"
            onkeydown={handleKeydown}
            onblur={handleBlur}
        />
    {/if}
</div>

