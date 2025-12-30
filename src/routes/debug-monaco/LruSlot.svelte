<script lang="ts">
    import { onDestroy } from "svelte";
    import { preloadMonaco } from "$lib/monaco/monaco-runtime";
    import { getWindowEditorPool } from "$lib/monaco/editor-pool";
    import type { EditorHandle } from "$lib/monaco/editor-types";

    let { id, active, onLog } = $props<{
        id: number;
        active: boolean;
        onLog: (msg: string) => void;
    }>();

    let container: HTMLDivElement | null = null;
    let hasEditor = $state(false);
    let handle: EditorHandle | null = null;
    let theftCheck: any;

    async function ensureEditor() {
        if (!container) return;
        const monaco = await preloadMonaco();
        const pool = getWindowEditorPool(monaco);
        handle = pool.acquire({
            contextId: `lru-slot-${id}`,
            windowId: "main",
            kind: "json",
            modelUri: `json://lru-slot-${id}`,
            container: () => container,
        });
        hasEditor = true;
        onLog(`Slot ${id}: Acquired editor`);
        handle.editor.setValue(
            `// LRU Slot ${id}\n{\n  "status": "Occupying Editor",\n  "slot": ${id}\n}`,
        );
    }

    function cleanupEditor() {
        if (handle) {
            handle.release();
            handle = null;
            hasEditor = false;
            onLog(`Slot ${id}: Released editor`);
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
                    onLog(`Slot ${id}: Editor was stolen!`);
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
    class="relative flex flex-col h-full bg-slate-950 border {active
        ? 'border-indigo-500/50'
        : 'border-slate-800'} rounded-lg overflow-hidden transition-all duration-300"
>
    <div
        class="flex justify-between items-center px-2 py-1 bg-slate-900/50 border-b border-slate-800"
    >
        <span
            class="text-[10px] font-bold {active
                ? 'text-indigo-400'
                : 'text-slate-600'}">SLOT {id}</span
        >
        <div
            class="w-1.5 h-1.5 rounded-full {hasEditor
                ? 'bg-emerald-500 shadow-[0_0_8px_rgba(16,185,129,0.5)]'
                : 'bg-slate-800'}"
        ></div>
    </div>

    <div class="flex-1 relative">
        <div bind:this={container} class="absolute inset-0 w-full h-full"></div>

        {#if !active}
            <div
                class="absolute inset-0 flex items-center justify-center bg-slate-950/80 backdrop-blur-[1px] text-[10px] text-slate-700 italic"
            >
                Standby
            </div>
        {:else if !hasEditor}
            <div
                class="absolute inset-0 flex items-center justify-center text-[10px] text-indigo-400/50 animate-pulse"
            >
                Acquiring...
            </div>
        {/if}
    </div>
</div>
