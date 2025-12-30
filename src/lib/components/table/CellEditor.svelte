<script lang="ts">
    import type { Column } from "./types";
    import { onMount } from "svelte";
    import { resolveEditor } from "./resolver";
    import { getEditorRenderer, registerEditorRenderer } from "./registry";

    // Import editors to register them (side-effect or explicit)
    import IndependentInlineTextEditor from "./editors/InlineTextEditor.svelte";
    import BooleanPopoverEditor from "./BooleanPopoverEditor.svelte";
    import EnumPopoverEditor from "./EnumPopoverEditor.svelte";
    import JsonPopoverEditor from "./JsonPopoverEditor.svelte";
    import DateTimePopoverEditor from "./DateTimePopoverEditor.svelte";
    import NumberPopoverEditor from "./NumberPopoverEditor.svelte";
    // We map "TextPopoverEditor" to a registry key if needed, or use InlineTextEditor for "text"
    import TextPopoverEditor from "./TextPopoverEditor.svelte";

    // Register default editors (idempotent)
    registerEditorRenderer("text", IndependentInlineTextEditor);
    registerEditorRenderer("boolean", BooleanPopoverEditor);
    registerEditorRenderer("enum", EnumPopoverEditor);
    registerEditorRenderer("json", JsonPopoverEditor);
    registerEditorRenderer("datetime", DateTimePopoverEditor);
    registerEditorRenderer("number", NumberPopoverEditor);
    // You might want a "text-popover" for specific cases
    registerEditorRenderer("text-popover", TextPopoverEditor);

    interface Props {
        value: any;
        column: Column;
        onCommit: (newValue: any) => void;
        onCancel: () => void;
        anchorEl?: HTMLElement | null;
        trigger?: string; // Optional trigger context
    }

    let { value, column, onCommit, onCancel, anchorEl, trigger }: Props =
        $props();

    // 1. Resolve configuration
    let config = $derived(resolveEditor(column, value, trigger));

    // 2. Get renderer component
    let RendererComponent = $derived(getEditorRenderer(config.renderer));

    // 3. Normalize props
    let rendererProps = $derived({
        value,
        column, // Some editors might still need column access
        onCommit,
        onCancel,
        anchorEl,
        ...config.props,
    });

    // Debug
    $effect(() => {
        if (!RendererComponent) {
            console.error(
                `[CellEditor] No renderer found for key: "${config.renderer}" (col: ${column.id})`,
            );
        }
    });
</script>

<div
    class="w-full h-full"
    onmousedown={(e) => e.stopPropagation()}
    onclick={(e) => e.stopPropagation()}
    role="presentation"
>
    {#if RendererComponent}
        <RendererComponent {...rendererProps} />
    {:else}
        <!-- Fallback safe mode -->
        <IndependentInlineTextEditor {...rendererProps} />
    {/if}
</div>
