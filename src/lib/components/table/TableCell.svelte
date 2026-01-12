<script lang="ts">
    import type { Column } from "./types";
    import { cn } from "$lib/utils";
    import { Badge } from "$lib/components/ui/badge";
    import CellEditor from "./CellEditor.svelte";

    interface Props {
        row: any;
        column: Column;
        rowIndex: number;
        columnIndex: number;
        isSelected?: boolean;
        isEditing?: boolean;
        isFocused?: boolean;
        isPendingEdit?: boolean;
        pendingValue?: any;
        disabled?: boolean;
        onClick?: (
            rowIndex: number,
            columnIndex: number,
            event: MouseEvent,
        ) => void;
        onMouseDown?: (
            rowIndex: number,
            columnIndex: number,
            event: MouseEvent,
        ) => void;
        onMouseEnter?: (rowIndex: number, columnIndex: number) => void;
        onDoubleClick?: (rowIndex: number, columnIndex: number) => void;
        onEditComplete?: (
            rowIndex: number,
            columnIndex: number,
            newValue: any,
        ) => void;
        onEditCancel?: () => void;
        onContextMenu?: (
            rowIndex: number,
            columnIndex: number,
            event: MouseEvent,
        ) => void;
    }

    let {
        row,
        column,
        rowIndex,
        columnIndex,
        isSelected,
        isEditing,
        isFocused,
        isPendingEdit,
        pendingValue,
        disabled = false,
        onClick,
        onMouseDown,
        onMouseEnter,
        onDoubleClick,
        onEditComplete,
        onEditCancel,
        onContextMenu,
    }: Props = $props();

    function unwrapValue(v: any) {
        let current = v;
        let depth = 0;
        while (typeof current === "function" && depth < 3) {
            try {
                current = current();
            } catch {
                break;
            }
            depth += 1;
        }
        return current;
    }

    // Use pendingValue if it exists (even if it's falsy like 0 or ""), otherwise use original row value
    let value = $derived(
        unwrapValue(pendingValue !== undefined ? pendingValue : row[column.id]),
    );

    import {
        DEFAULT_TOKEN,
        NULL_TOKEN,
        displayBooleanValue,
    } from "./valueUtils";

    const LONG_TEXT_THRESHOLD = 120;

    const isBoolean = () => column.type === "boolean";
    const isJson = () =>
        column.type === "json" ||
        column.type === "jsonb" ||
        column.type === "JSON";
    const isBinary = () =>
        column.type === "blob" ||
        column.type === "bytea" ||
        column.type === "binary";
    const isDateLike = () =>
        column.type === "date" ||
        column.type === "time" ||
        column.type === "datetime";

    function formatDateValue(val: any) {
        if (val === null || val === undefined) return "";
        const d = val instanceof Date ? val : new Date(val);
        if (Number.isNaN(d.getTime())) return String(val);
        const opts: Intl.DateTimeFormatOptions =
            column.type === "date"
                ? { year: "numeric", month: "2-digit", day: "2-digit" }
                : column.type === "time"
                  ? {
                        hour: "2-digit",
                        minute: "2-digit",
                        second: "2-digit",
                    }
                  : {
                        year: "numeric",
                        month: "2-digit",
                        day: "2-digit",
                        hour: "2-digit",
                        minute: "2-digit",
                        second: "2-digit",
                        timeZoneName: "short",
                    };
        return new Intl.DateTimeFormat(undefined, opts).format(d);
    }

    function jsonPreview(val: any) {
        try {
            const str =
                typeof val === "string" ? val : JSON.stringify(val, null, 0);
            return str.length > 80 ? str.slice(0, 80) + "…" : str;
        } catch {
            return String(val);
        }
    }

    function jsonPretty(val: any) {
        try {
            return JSON.stringify(val, null, 2);
        } catch {
            return String(val);
        }
    }

    function binaryInfo(val: any) {
        if (val === null || val === undefined)
            return { length: 0, preview: "" };
        if (typeof val === "string") {
            return {
                length: val.length,
                preview: val.slice(0, 64),
            };
        }
        if (ArrayBuffer.isView(val)) {
            const uint = new Uint8Array(
                val.buffer,
                val.byteOffset,
                val.byteLength,
            );
            const hex = Array.from(uint.slice(0, 32))
                .map((b) => b.toString(16).padStart(2, "0"))
                .join("");
            return { length: uint.length, preview: hex };
        }
        return { length: 0, preview: String(val).slice(0, 64) };
    }

    let displayValue = $derived.by(() => {
        if (typeof value === "function") {
            return "[fn]";
        }
        if (isBoolean()) {
            return displayBooleanValue(value);
        }
        if (isJson() || Array.isArray(value)) return jsonPreview(value);
        if (isBinary()) {
            const info = binaryInfo(value);
            return `len ${info.length}${info.preview ? ` · ${info.preview}` : ""}`;
        }
        if (isDateLike()) return formatDateValue(value);
        return value;
    });

    let isLongText = $derived.by(
        () =>
            typeof displayValue === "string" &&
            displayValue.length > LONG_TEXT_THRESHOLD,
    );

    function handleClick(e: MouseEvent) {
        if (disabled) return;
        e.stopPropagation();
        onClick?.(rowIndex, columnIndex, e);
    }

    function handleMouseDown(e: MouseEvent) {
        if (disabled) return;
        onMouseDown?.(rowIndex, columnIndex, e);
    }

    function handleMouseEnter() {
        if (disabled) return;
        onMouseEnter?.(rowIndex, columnIndex);
    }

    function handleDoubleClick() {
        if (disabled) return;
        onDoubleClick?.(rowIndex, columnIndex);
    }

    function handleContextMenu(event: MouseEvent) {
        if (disabled) return;
        event.preventDefault();
        event.stopPropagation();
        onContextMenu?.(rowIndex, columnIndex, event);
    }

    function handleEditComplete(newValue: any) {
        onEditComplete?.(rowIndex, columnIndex, newValue);
    }

    function handleEditCancel() {
        onEditCancel?.();
    }

    let cellEl = $state<HTMLDivElement | null>(null);
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
    bind:this={cellEl}
    class={cn(
        "relative flex items-center border-r truncate text-sm select-none text-foreground border-border box-border outline-none focus:outline-none",
        // Selection state
        isSelected && "bg-accent/10 border-accent/20",
        // Focus state (keyboard nav)
        isFocused && "z-10 ring-1 ring-inset ring-accent",
        // Pending/Dirty state (edited but not saved)
        isPendingEdit &&
            "bg-amber-500/10 italic border-l-2 border-l-amber-500 pl-1.5",
        // Editing state (active input)
        isEditing && "z-20 bg-surface ring-2 ring-inset ring-accent shadow-lg",
        // Disabled
        disabled && "opacity-50 cursor-not-allowed",
    )}
    style="width: {column.width || 150}px; min-width: {column.minWidth ||
        50}px; max-width: {column.maxWidth}px; flex-shrink: 0; overflow: hidden;"
    onclick={handleClick}
    onmousedown={handleMouseDown}
    onmouseenter={handleMouseEnter}
    ondblclick={handleDoubleClick}
    oncontextmenu={handleContextMenu}
    aria-disabled={disabled}
    data-row-index={rowIndex}
    data-col-index={columnIndex}
    data-is-focused={isFocused}
    tabindex="-1"
>
    {#if isEditing}
        <CellEditor
            {value}
            {column}
            anchorEl={cellEl}
            onCommit={handleEditComplete}
            onCancel={handleEditCancel}
        />
    {/if}

    <div class={cn("w-full h-full truncate", isEditing && "invisible")}>
        {#if isBoolean()}
            <span
                class="truncate select-none px-2 py-1 w-full h-full block text-xs"
            >
                {displayValue}
            </span>
        {:else if isJson() || Array.isArray(value)}
            <span
                class="truncate select-none px-2 py-1 w-full h-full block font-mono text-xs text-muted-foreground"
            >
                {jsonPreview(value)}
            </span>
        {:else if isBinary()}
            {@const info = binaryInfo(value)}
            <span
                class="truncate select-none px-2 py-1 w-full h-full block font-mono text-xs text-muted-foreground"
            >
                {`len ${info.length}${info.preview ? ` · ${info.preview}` : ""}`}
            </span>
        {:else if isDateLike()}
            <span class="truncate select-none px-2 py-1 w-full h-full block">
                {formatDateValue(value)}
            </span>
        {:else if isLongText}
            <span class="truncate select-none px-2 py-1 w-full h-full block">
                {typeof displayValue === "string"
                    ? displayValue.slice(0, LONG_TEXT_THRESHOLD) + "…"
                    : displayValue}
            </span>
        {:else}
            <span class="truncate select-none px-2 py-1 w-full h-full block">
                {displayValue}
            </span>
        {/if}
    </div>
</div>
