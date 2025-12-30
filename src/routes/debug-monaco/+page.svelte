<script lang="ts">
    import { onMount } from "svelte";
    import { getWindowEditorPool } from "$lib/monaco/editor-pool";
    import type { EditorHandle } from "$lib/monaco/editor-types";
    import MonacoHealthPanel from "$lib/monaco/MonacoHealthPanel.svelte";
    import TestEditorInstance from "./TestEditorInstance.svelte";

    let isMounted = $state(true);
    let logs = $state<string[]>([]);
    let stressRunning = $state(false);

    function log(msg: string) {
        logs = [
            `${new Date().toISOString().split("T")[1].substring(0, 12)} - ${msg}`,
            ...logs.slice(0, 99),
        ];
        console.log(`[DEBUG-PAGE] ${msg}`);
    }

    const initialJson = JSON.stringify(
        { hello: "world", debug: true, timestamp: Date.now() },
        null,
        2,
    );

    let handle: EditorHandle | null = null;

    function handleAcquired(h: EditorHandle) {
        handle = h;
        if (h.editor.getValue() === "") {
            h.editor.setValue(initialJson);
            log("Initial value set");
        }
    }

    function toggleMount() {
        isMounted = !isMounted;
        log(`Mounted: ${isMounted}`);
        if (!isMounted) handle = null;
    }

    async function runStressTest() {
        if (stressRunning) return;
        stressRunning = true;
        log("Starting rapid mount/unmount stress test (20 cycles)...");

        for (let i = 0; i < 20; i++) {
            isMounted = false;
            await new Promise((r) => setTimeout(r, 50));
            isMounted = true;
            await new Promise((r) => setTimeout(r, 150));
            if (i % 5 === 0) log(`Cycle ${i} complete...`);
        }

        log("Stress test complete.");
        stressRunning = false;
    }

    async function stressLRU() {
        log("Testing LRU: acquiring 5 different models (MAX is 3)...");
        const pool = getWindowEditorPool();
        const handles: any[] = [];

        for (let i = 0; i < 5; i++) {
            const h = pool.acquire({
                contextId: `lru-test-${i}`,
                windowId: "main",
                kind: "json",
                modelUri: `json://lru-${i}`,
                container: document.createElement("div"), // Virtual container for testing
            });
            handles.push(h);
            log(`Acquired lru-${i}`);
            await new Promise((r) => setTimeout(r, 100));
        }

        log("Releasing all LRU test handles...");
        handles.forEach((h) => h.release());
        log(
            "LRU test complete. Check Health Panel for editor count (should be 3).",
        );
    }
</script>

<div
    class="p-8 space-y-4 bg-slate-900 text-slate-200 min-h-screen font-mono relative overflow-hidden"
>
    <div
        class="flex justify-between items-center border-b border-slate-800 pb-4"
    >
        <h1 class="text-2xl font-bold text-white tracking-tight">
            Monaco Debug Harness
        </h1>
        <div class="flex gap-2">
            <span
                class="px-2 py-1 bg-slate-800 rounded text-[10px] text-slate-400"
                >DEV MODE</span
            >
        </div>
    </div>

    <div class="grid grid-cols-12 gap-6">
        <!-- Controls & Logs -->
        <div class="col-span-4 space-y-6">
            <section
                class="bg-slate-800/50 p-4 rounded-lg border border-slate-700 space-y-3"
            >
                <h2
                    class="text-xs font-bold uppercase text-slate-500 tracking-wider"
                >
                    Lifecycle Controls
                </h2>
                <div class="grid grid-cols-2 gap-2">
                    <button
                        class="px-3 py-2 bg-indigo-600 hover:bg-indigo-500 rounded text-xs font-bold transition-colors"
                        onclick={toggleMount}
                    >
                        {isMounted ? "Unmount" : "Mount"}
                    </button>
                    <button
                        class="px-3 py-2 bg-emerald-600 hover:bg-emerald-500 rounded text-xs font-bold transition-colors"
                        onclick={() => handle?.editor.layout()}
                    >
                        Layout
                    </button>
                </div>
                <div class="space-y-2 pt-2">
                    <button
                        class="w-full px-3 py-2 border border-rose-500/30 bg-rose-500/10 hover:bg-rose-500/20 text-rose-400 rounded text-xs font-bold transition-all disabled:opacity-50"
                        onclick={runStressTest}
                        disabled={stressRunning}
                    >
                        {stressRunning
                            ? "Running Stress Test..."
                            : "Start Lifecycle Stress"}
                    </button>
                    <button
                        class="w-full px-3 py-2 border border-amber-500/30 bg-amber-500/10 hover:bg-amber-500/20 text-amber-400 rounded text-xs font-bold transition-all"
                        onclick={stressLRU}
                    >
                        Test LRU Eviction
                    </button>
                </div>
            </section>

            <section
                class="bg-slate-950 p-3 rounded-lg border border-slate-800 h-[400px] flex flex-col"
            >
                <h2
                    class="text-xs font-bold uppercase text-slate-600 tracking-wider mb-2"
                >
                    Event Logs
                </h2>
                <div
                    class="flex-1 overflow-auto space-y-1 font-mono text-[10px] pr-2 scrollbar-thin scrollbar-thumb-slate-700"
                >
                    {#each logs as l}
                        <div
                            class="border-l-2 border-slate-800 pl-2 py-0.5 text-slate-400 font-mono"
                        >
                            {l}
                        </div>
                    {:else}
                        <div class="text-slate-700 italic">No logs yet...</div>
                    {/each}
                </div>
            </section>
        </div>

        <!-- Editor Viewport -->
        <div class="col-span-8 flex flex-col space-y-4">
            <div
                class="relative bg-slate-950 rounded-xl border border-slate-800 aspect-video shadow-2xl overflow-hidden group"
            >
                <div class="absolute top-3 left-4 z-10 flex items-center gap-2">
                    <div
                        class="w-2 h-2 rounded-full {isMounted
                            ? 'bg-emerald-500'
                            : 'bg-slate-700'}"
                    ></div>
                    <span
                        class="text-[10px] font-bold text-slate-500 uppercase tracking-widest"
                        >Viewport</span
                    >
                </div>

                {#if isMounted}
                    <TestEditorInstance
                        onAcquired={handleAcquired}
                        onLog={log}
                    />
                {:else}
                    <div
                        class="absolute inset-0 flex items-center justify-center text-slate-600 italic text-sm"
                    >
                        Container Detached
                    </div>
                {/if}
            </div>

            <div
                class="bg-indigo-900/10 border border-indigo-500/20 p-4 rounded-lg"
            >
                <p class="text-[11px] text-indigo-300 flex gap-2">
                    <span class="font-bold">TIP:</span> Use Arrow Keys and Cmd+A
                    to verify the keyboard service is active.
                </p>
            </div>
        </div>
    </div>

    <!-- Health Panel Overlay -->
    <MonacoHealthPanel />
</div>

<style>
    :global(body) {
        background: #0f172a;
    }

    /* Simple scrollbar styles */
    .scrollbar-thin::-webkit-scrollbar {
        width: 4px;
    }
    .scrollbar-thin::-webkit-scrollbar-track {
        background: transparent;
    }
    .scrollbar-thin::-webkit-scrollbar-thumb {
        background: #334155;
        border-radius: 10px;
    }
</style>
