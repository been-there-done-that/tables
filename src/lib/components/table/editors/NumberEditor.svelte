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
    let localValue = $state<any>(value);

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
            // Commit on Enter or Cmd+Enter
            e.preventDefault();
            e.stopPropagation();
            // Convert to number before commit if possible
            const num = parseFloat(localValue);
            onCommit(isNaN(num) ? localValue : num);
        } else if (e.key === "Escape") {
            e.preventDefault();
            e.stopPropagation();
            onCancel();
        } else if (e.key === "Tab") {
            const num = parseFloat(localValue);
            onCommit(isNaN(num) ? localValue : num);
        }
    }

    function handleBlur() {
        const num = parseFloat(localValue);
        onCommit(isNaN(num) ? localValue : num);
    }
</script>

<input
    bind:this={inputEl}
    bind:value={localValue}
    type="number"
    class="w-full h-full bg-surface border-none outline-none px-2 py-1 text-sm m-0 leading-none focus:ring-0 box-border"
    style="width: 100%; min-width: 100%; max-width: 100%;"
    onkeydown={handleKeydown}
    onblur={handleBlur}
    onclick={(e) => e.stopPropagation()}
    {...rest}
/>

<style>
    input {
        font-family: inherit;
        color: inherit;
        background-color: var(--theme-bg-primary);
        appearance: textfield; /* Remove spinner buttons for cleaner look if desired */
    }
    input::-webkit-outer-spin-button,
    input::-webkit-inner-spin-button {
        -webkit-appearance: none;
        margin: 0;
    }
</style>
