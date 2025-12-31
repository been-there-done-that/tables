<script lang="ts">
    import { onDestroy } from "svelte";
    import { preloadMonaco } from "$lib/monaco/monaco-runtime";
    import { getWindowEditorPool } from "$lib/monaco/editor-pool";
    import type { EditorHandle } from "$lib/monaco/editor-types";

    interface Props {
        id: number | string;
        name?: string;
        active: boolean;
        contextPrefix: string;
        modelUriPrefix: string;
        onLog: (msg: string) => void;
        initialValue?: string;
        onStateChange?: (id: number | string, hasEditor: boolean) => void;
    }

    let {
        id,
        name,
        active,
        contextPrefix,
        modelUriPrefix,
        onLog,
        initialValue,
        onStateChange,
    }: Props = $props();

    let container: HTMLDivElement | null = null;
    let hasEditor = $state(false);
    let isStolen = $state(false);
    let handle: EditorHandle | null = null;
    let theftCheck: any;

    async function ensureEditor() {
        if (!container) return;
        isStolen = false;
        const monaco = await preloadMonaco();
        const pool = getWindowEditorPool(monaco);
        handle = pool.acquire({
            contextId: `${contextPrefix}-${id}`,
            windowId: "main",
            kind: "json",
            modelUri: `${modelUriPrefix}://${contextPrefix}-${id}`,
            container: () => container,
        });
        hasEditor = true;
        isStolen = false;
        onStateChange?.(id, true);
        onLog(`${name || id}: Acquired editor`);
        if (initialValue) {
            handle.editor.setValue(initialValue);
        } else {
            handle.editor.setValue(
                `// ${name || id}\n{\n  "status": "Occupying Editor",\n  "timestamp": ${Date.now()}\n}`,
            );
        }
    }

    function cleanupEditor() {
        if (handle) {
            handle.release();
            handle = null;
            hasEditor = false;
            isStolen = false;
            onStateChange?.(id, false);
            onLog(`${name || id}: Released editor`);
        }
    }

    // React to activation toggle
    $effect(() => {
        if (active) {
            ensureEditor();
        } else {
            cleanupEditor();
        }
    });

    // Detect if editor was stolen by the pool for another slot
    $effect(() => {
        if (active && hasEditor) {
            theftCheck = setInterval(() => {
                if (container && container.children.length === 0) {
                    hasEditor = false;
                    isStolen = true;
                    onStateChange?.(id, false);
                    onLog(`${name || id}: Editor was stolen!`);
                }
            }, 100);
            return () => clearInterval(theftCheck);
        }
    });

    onDestroy(() => {
        clearInterval(theftCheck);
        cleanupEditor();
    });
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
    class="relative flex flex-col h-full bg-background border {active
        ? isStolen
            ? 'border-yellow-500/40 cursor-pointer hover:bg-yellow-500/5'
            : 'border-accent/40 shadow-lg shadow-accent/20'
        : 'border-border'} rounded-lg overflow-hidden transition-all duration-300"
    onclick={() => {
        if (isStolen) ensureEditor();
    }}
>
    <div
        class="flex justify-between items-center px-2 py-1 bg-muted/30 border-b border-border"
    >
        <span
            class="text-[10px] font-bold uppercase tracking-tight {active
                ? isStolen
                    ? 'text-yellow-500'
                    : 'text-accent'
                : 'text-muted-foreground'}">{name || `SLOT ${id}`}</span
        >
        <div
            class="w-1.5 h-1.5 rounded-full {hasEditor
                ? 'bg-emerald-500 shadow-[0_0_8px_rgba(16,185,129,0.5)]'
                : isStolen
                  ? 'bg-yellow-500 animate-pulse'
                  : 'bg-border'}"
        ></div>
    </div>

    <div class="flex-1 relative">
        <div bind:this={container} class="absolute inset-0 w-full h-full"></div>

        {#if !active}
            <div
                class="absolute inset-0 flex items-center justify-center bg-background/80 backdrop-blur-[1px] text-[10px] text-muted-foreground italic"
            >
                Standby
            </div>
        {:else if isStolen}
            <div
                class="absolute inset-0 flex flex-col items-center justify-center bg-yellow-500/5 text-[10px] text-yellow-500 font-bold"
            >
                <span>EVICTED (LRU)</span>
                <span class="text-[8px] opacity-70 font-normal italic"
                    >Pool Capacity Exceeded</span
                >
            </div>
        {:else if !hasEditor}
            <div
                class="absolute inset-0 flex items-center justify-center text-[10px] text-accent/50 animate-pulse"
            >
                Acquiring...
            </div>
        {/if}
    </div>
</div>
