<script lang="ts">
    import { onMount, tick } from "svelte";

    interface Props {
        value: any;
        onCommit: (val: any) => void;
        onCancel: () => void;
        class?: string;
        [key: string]: any;
    }

    let {
        value,
        onCommit,
        onCancel,
        class: className,
        ...rest
    }: Props = $props();

    let inputEl: HTMLInputElement;
    let localValue = $state<any>(value); // Initialize with passed value

    // Sync on mount/prop change
    $effect(() => {
        localValue = value;
    });

    onMount(async () => {
        await tick();
        if (inputEl) {
            inputEl.focus();
            inputEl.select();
        }
    });

    function handleKeydown(e: KeyboardEvent) {
        if (e.key === "Enter") {
            if (e.metaKey || e.ctrlKey || !e.shiftKey) {
                // Allow Shift+Enter for newline if we switch to textarea later
                e.preventDefault();
                e.stopPropagation();
                onCommit(localValue);
            }
        } else if (e.key === "Escape") {
            e.preventDefault();
            e.stopPropagation();
            onCancel();
        } else if (e.key === "Tab") {
            // Let table handle tab navigation, but commit first?
            // Usually default is to just let blur handle commit,
            // but explicit commit is safer if parent captures tab.
            // For now, let's treat Tab as commit + move.
            onCommit(localValue);
            // Do NOT stop propagation of Tab so the table can move focus
        }
    }

    function handleBlur() {
        onCommit(localValue);
    }
</script>

<input
    bind:this={inputEl}
    bind:value={localValue}
    type="text"
    class="w-full h-full bg-surface border-none outline-none px-2 py-1 text-sm m-0 leading-none focus:ring-0 box-border"
    style="width: 100%; min-width: 100%; max-width: 100%;"
    autocomplete="new-password"
    data-lpignore="true"
    data-form-type="other"
    data-1p-ignore
    onkeydown={handleKeydown}
    onblur={handleBlur}
    onclick={(e) => e.stopPropagation()}
    {...rest}
/>

<style>
    /* Ensure no browser defaults mess up the inline look */
    input {
        font-family: inherit;
        color: inherit;
        background-color: var(--theme-bg-primary);
    }
</style>
