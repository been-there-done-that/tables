<script lang="ts">
    import { onMount } from "svelte";
    import type { EditorHandle } from "$lib/monaco/editor-types";
    import MonacoHealthPanel from "$lib/monaco/MonacoHealthPanel.svelte";
    import TestEditorInstance from "./TestEditorInstance.svelte";
    import LruSlot from "./LruSlot.svelte";

    // Lifecycle State
    let isMounted = $state(true);
    let stressRunning = $state(false);

    // LRU State
    let lruSlots = $state([false, false, false, false, false]);
    let lruRunning = $state(false);

    let logs = $state<string[]>([]);

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

    let lifeHandle: EditorHandle | null = null;

    function handleLifeAcquired(h: EditorHandle) {
        lifeHandle = h;
        if (h.editor.getValue() === "") {
            h.editor.setValue(initialJson);
            log("Harbor: Initial value set");
        }
    }

    function toggleMount() {
        isMounted = !isMounted;
        log(`Harbor: Mounted = ${isMounted}`);
        if (!isMounted) lifeHandle = null;
    }

    async function runStressTest() {
        if (stressRunning) return;
        stressRunning = true;
        log("Harbor: Starting lifecycle stress...");
        for (let i = 0; i < 15; i++) {
            isMounted = false;
            await new Promise((r) => setTimeout(r, 50));
            isMounted = true;
            await new Promise((r) => setTimeout(r, 150));
        }
        log("Harbor: Lifecycle stress complete.");
        stressRunning = false;
    }

    async function runLruTest() {
        if (lruRunning) return;
        lruRunning = true;
        log("LRU Lab: Starting sequential activation...");

        // Reset
        lruSlots = [false, false, false, false, false];
        await new Promise((r) => setTimeout(r, 500));

        for (let i = 0; i < 5; i++) {
            lruSlots[i] = true;
            log(`LRU Lab: Activated Slot ${i + 1}`);
            await new Promise((r) => setTimeout(r, 800)); // Wait to see the editor move
        }

        log(
            "LRU Lab: Test complete. Notice Slots 1 & 2 lost their editors (stolen)!",
        );
        lruRunning = false;
    }

    function resetLru() {
        lruSlots = [false, false, false, false, false];
        log("LRU Lab: All slots released");
    }
</script>

<div
    class="p-6 space-y-6 bg-slate-900 text-slate-200 min-h-screen font-mono relative overflow-hidden"
>
    <div
        class="flex justify-between items-center border-b border-slate-800 pb-4"
    >
        <div>
            <h1 class="text-2xl font-bold text-white tracking-tight">
                Monaco Debug Harness
            </h1>
            <p
                class="text-[10px] text-slate-500 mt-1 uppercase tracking-widest"
            >
                Advanced Pool Visualization & Stress Testing
            </p>
        </div>
        <div class="flex gap-2">
            <span
                class="px-2 py-1 bg-indigo-500/10 border border-indigo-500/20 rounded text-[10px] text-indigo-400 font-bold tracking-tighter"
                >POOL_SIZE=3</span
            >
            <span
                class="px-2 py-1 bg-slate-800 rounded text-[10px] text-slate-400"
                >DEV MODE</span
            >
        </div>
    </div>

    <div class="grid grid-cols-12 gap-6">
        <!-- Column 1: Lifecycle Harbor -->
        <div class="col-span-5 flex flex-col space-y-4">
            <div
                class="bg-slate-800/30 border border-slate-800 p-4 rounded-xl space-y-4"
            >
                <div class="flex justify-between items-center">
                    <h2
                        class="text-xs font-bold uppercase text-slate-400 tracking-wider"
                    >
                        ⚓ Lifecycle Harbor
                    </h2>
                    <div class="flex gap-2">
                        <button
                            class="px-3 py-1.5 bg-indigo-600 hover:bg-indigo-500 rounded text-[10px] font-bold transition-all shadow-lg shadow-indigo-500/20"
                            onclick={toggleMount}
                        >
                            {isMounted ? "UNMOUNT" : "MOUNT"}
                        </button>
                        <button
                            class="px-3 py-1.5 border border-rose-500/30 bg-rose-500/10 hover:bg-rose-500/20 text-rose-400 rounded text-[10px] font-bold transition-all disabled:opacity-50"
                            onclick={runStressTest}
                            disabled={stressRunning}
                        >
                            {stressRunning ? "RUNNING..." : "STRESS TEST"}
                        </button>
                    </div>
                </div>

                <div
                    class="relative aspect-video bg-slate-950 rounded-lg border border-slate-800 overflow-hidden shadow-2xl"
                >
                    {#if isMounted}
                        <TestEditorInstance
                            onAcquired={handleLifeAcquired}
                            onLog={log}
                        />
                    {:else}
                        <div
                            class="absolute inset-0 flex items-center justify-center text-slate-700 italic text-xs bg-[radial-gradient(circle_at_center,_#0f172a_0%,_#020617_100%)]"
                        >
                            SLOT_VACANT
                        </div>
                    {/if}
                    <div
                        class="absolute bottom-2 right-2 px-1.5 py-0.5 bg-black/50 text-[8px] text-slate-500 rounded border border-white/5 uppercase"
                    >
                        Harbor_Viewport
                    </div>
                </div>
            </div>

            <!-- Event Logs -->
            <section
                class="bg-slate-950 p-3 rounded-xl border border-slate-800 h-[300px] flex flex-col shadow-inner"
            >
                <h2
                    class="text-[9px] font-bold uppercase text-slate-600 tracking-widest mb-2 border-b border-slate-900 pb-1"
                >
                    Telemetry Logs
                </h2>
                <div
                    class="flex-1 overflow-auto space-y-1 pr-2 scrollbar-thin scrollbar-thumb-slate-800"
                >
                    {#each logs as l}
                        <div
                            class="pl-2 py-0.5 text-slate-500 font-mono text-[9px] hover:text-slate-300 transition-colors"
                        >
                            {l}
                        </div>
                    {:else}
                        <div class="text-slate-800 italic text-[9px]">
                            Initializing system telemetry...
                        </div>
                    {/each}
                </div>
            </section>
        </div>

        <!-- Column 2: LRU Lab -->
        <div class="col-span-7 flex flex-col space-y-4">
            <div
                class="bg-indigo-500/5 border border-indigo-500/10 p-4 rounded-xl space-y-4"
            >
                <div class="flex justify-between items-center">
                    <div>
                        <h2
                            class="text-xs font-bold uppercase text-indigo-400 tracking-wider"
                        >
                            🧪 LRU Laboratory
                        </h2>
                        <p class="text-[9px] text-indigo-500/60 mt-0.5">
                            Visually demonstrates editor theft when pool
                            capacity is exceeded.
                        </p>
                    </div>
                    <div class="flex gap-2">
                        <button
                            class="px-3 py-1.5 bg-emerald-600 hover:bg-emerald-500 rounded text-[10px] font-bold transition-all shadow-lg shadow-emerald-500/20"
                            onclick={runLruTest}
                            disabled={lruRunning}
                        >
                            {lruRunning ? "RUNNING..." : "START LRU TEST"}
                        </button>
                        <button
                            class="px-3 py-1.5 border border-slate-700 hover:bg-slate-800 rounded text-[10px] font-bold transition-all"
                            onclick={resetLru}
                        >
                            RESET
                        </button>
                    </div>
                </div>

                <!-- LRU Grid -->
                <div class="grid grid-cols-5 gap-3 h-[280px]">
                    {#each lruSlots as active, i}
                        <LruSlot id={i + 1} {active} onLog={log} />
                    {/each}
                </div>

                <div
                    class="p-3 bg-indigo-500/10 border border-indigo-500/20 rounded-lg"
                >
                    <h3
                        class="text-[10px] font-bold text-indigo-300 uppercase mb-1"
                    >
                        Experiment Guide:
                    </h3>
                    <p class="text-[9px] text-indigo-400/80 leading-relaxed">
                        The pool is capped at <span
                            class="text-white font-bold underline"
                            >3 editors</span
                        >. As you activate slots 4 and 5, the system will
                        "steal" the editors from slots 1 and 2 (LRU policy).
                        Watch them turn back to
                        <span class="italic text-slate-500">Standby</span> mode as
                        their DOM nodes are yanked away!
                    </p>
                </div>
            </div>

            <div
                class="flex-1 bg-slate-800/10 border border-slate-800 rounded-xl p-4 flex flex-col justify-center items-center text-center space-y-2"
            >
                <div
                    class="text-[10px] text-slate-500 font-bold uppercase tracking-widest"
                >
                    System Instruction
                </div>
                <p
                    class="text-xs text-slate-400 max-w-sm leading-relaxed italic"
                >
                    "Use Arrow Keys and shortcuts in any active slot to verify
                    the keyboard delegation remains intact after editor
                    movement."
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
        overflow-x: hidden;
    }

    /* Scrollbar styling */
    .scrollbar-thin::-webkit-scrollbar {
        width: 3px;
    }
    .scrollbar-thin::-webkit-scrollbar-track {
        background: transparent;
    }
    .scrollbar-thin::-webkit-scrollbar-thumb {
        background: #1e293b;
        border-radius: 10px;
    }
</style>
