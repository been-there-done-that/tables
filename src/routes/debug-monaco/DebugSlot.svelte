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
    }

    let {
        id,
        name,
        active,
        contextPrefix,
        modelUriPrefix,
        onLog,
        initialValue,
    }: Props = $props();

    let container: HTMLDivElement | null = null;
    let hasEditor = $state(false);
    let handle: EditorHandle | null = null;
    let theftCheck: any;

    async function ensureEditor() {
        if (!container) return;
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

<div
    class="relative flex flex-col h-full bg-background border {active
        ? 'border-accent/40 shadow-[0_0_15px_rgba(var(--theme-accent-primary-rgb),0.1)]'
        : 'border-border'} rounded-lg overflow-hidden transition-all duration-300"
>
    <div
        class="flex justify-between items-center px-2 py-1 bg-muted/30 border-b border-border"
    >
        <span
            class="text-[10px] font-bold uppercase tracking-tight {active
                ? 'text-accent'
                : 'text-muted-foreground'}">{name || `SLOT ${id}`}</span
        >
        <div
            class="w-1.5 h-1.5 rounded-full {hasEditor
                ? 'bg-emerald-500 shadow-[0_0_8px_rgba(16,185,129,0.5)]'
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
        {:else if !hasEditor}
            <div
                class="absolute inset-0 flex items-center justify-center text-[10px] text-accent/50 animate-pulse"
            >
                Acquiring...
            </div>
        {/if}
    </div>
</div>
