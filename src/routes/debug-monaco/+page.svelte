<script lang="ts">
    import MonacoHealthPanel from "$lib/monaco/MonacoHealthPanel.svelte";
    import DebugSlot from "./DebugSlot.svelte";

    // Harbor (Lifecycle) State
    let harborSlots = $state([true, false, false, false, false]);
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

    async function runStressTest() {
        if (stressRunning) return;
        stressRunning = true;
        log("Harbor: Starting lifecycle stress...");
        for (let i = 0; i < 20; i++) {
            const idx = Math.floor(Math.random() * 5);
            harborSlots[idx] = !harborSlots[idx];
            await new Promise((r) => setTimeout(r, 100));
        }
        log("Harbor: Lifecycle stress complete.");
        stressRunning = false;
    }

    function resetHarbor() {
        harborSlots = [true, false, false, false, false];
        log("Harbor: All slots reset to default");
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
    class="p-6 space-y-6 bg-background text-foreground min-h-screen font-mono relative overflow-hidden"
>
    <!-- Header -->
    <div class="flex justify-between items-center border-b border-border pb-4">
        <div>
            <h1 class="text-2xl font-bold text-foreground tracking-tight">
                Monaco Debug Harness
            </h1>
            <p
                class="text-[10px] text-muted-foreground mt-1 uppercase tracking-widest"
            >
                Advanced Pool Visualization & Stress Testing
            </p>
        </div>
        <div class="flex gap-2">
            <span
                class="px-2 py-1 bg-accent/10 border border-accent/20 rounded text-[10px] text-accent font-bold tracking-tighter"
                >POOL_SIZE=3</span
            >
            <span
                class="px-2 py-1 bg-muted rounded text-[10px] text-muted-foreground"
                >DEV MODE</span
            >
        </div>
    </div>

    <!-- Main Laboratory Area -->
    <div class="grid grid-cols-2 gap-6">
        <!-- Section 1: Lifecycle Harbor -->
        <div
            class="bg-accent/5 border border-accent/10 p-5 rounded-2xl flex flex-col space-y-4"
        >
            <div class="flex justify-between items-center">
                <div>
                    <h2
                        class="text-sm font-bold uppercase text-accent tracking-wider flex items-center gap-2"
                    >
                        <span>⚓</span> Lifecycle Harbor
                    </h2>
                    <p class="text-[10px] text-accent/60 mt-1">
                        Monitors editor acquisition, release, and rapid
                        mount/unmount cycles.
                    </p>
                </div>
                <div class="flex gap-2">
                    <button
                        class="px-3 py-1.5 bg-accent hover:bg-accent/80 text-white rounded-md text-[10px] font-bold transition-all shadow-lg shadow-accent/20 disabled:opacity-50"
                        onclick={runStressTest}
                        disabled={stressRunning}
                    >
                        {stressRunning ? "RUNNING..." : "STRESS TEST"}
                    </button>
                    <button
                        class="px-3 py-1.5 border border-border hover:bg-muted rounded-md text-[10px] font-bold transition-all"
                        onclick={resetHarbor}
                    >
                        RESET
                    </button>
                </div>
            </div>

            <!-- Harbor Grid -->
            <div class="grid grid-cols-5 gap-3 h-[240px]">
                {#each harborSlots as active, i}
                    <DebugSlot
                        id={i + 1}
                        name={`SHIP ${String.fromCharCode(65 + i)}`}
                        {active}
                        contextPrefix="harbor"
                        modelUriPrefix="json"
                        onLog={log}
                    />
                {/each}
            </div>

            <div class="p-3 bg-accent/10 border border-accent/20 rounded-lg">
                <h3 class="text-[10px] font-bold text-accent uppercase mb-1">
                    System Notes:
                </h3>
                <p class="text-[9px] text-accent/80 leading-relaxed">
                    Toggling "ships" forces the pool to move the DOM anchor. Use
                    the <strong>STRESS TEST</strong> to verify that rapid re-attachment
                    doesn't cause flickering or zombie editors.
                </p>
            </div>
        </div>

        <!-- Section 2: LRU Laboratory -->
        <div
            class="bg-accent/5 border border-accent/10 p-5 rounded-2xl flex flex-col space-y-4"
        >
            <div class="flex justify-between items-center">
                <div>
                    <h2
                        class="text-sm font-bold uppercase text-accent tracking-wider flex items-center gap-2"
                    >
                        <span>🧪</span> LRU Laboratory
                    </h2>
                    <p class="text-[10px] text-accent/60 mt-1">
                        Visually demonstrates editor theft when pool capacity is
                        exceeded.
                    </p>
                </div>
                <div class="flex gap-2">
                    <button
                        class="px-3 py-1.5 bg-emerald-600 hover:bg-emerald-500 text-white rounded-md text-[10px] font-bold transition-all shadow-lg shadow-emerald-500/20 disabled:opacity-50"
                        onclick={runLruTest}
                        disabled={lruRunning}
                    >
                        {lruRunning ? "RUNNING..." : "START LRU TEST"}
                    </button>
                    <button
                        class="px-3 py-1.5 border border-border hover:bg-muted rounded-md text-[10px] font-bold transition-all"
                        onclick={resetLru}
                    >
                        RESET
                    </button>
                </div>
            </div>

            <!-- LRU Grid -->
            <div class="grid grid-cols-5 gap-3 h-[240px]">
                {#each lruSlots as active, i}
                    <DebugSlot
                        id={i + 1}
                        {active}
                        contextPrefix="lru"
                        modelUriPrefix="json"
                        onLog={log}
                    />
                {/each}
            </div>

            <div class="p-3 bg-accent/10 border border-accent/20 rounded-lg">
                <h3 class="text-[10px] font-bold text-accent uppercase mb-1">
                    Experiment Guide:
                </h3>
                <p class="text-[9px] text-accent/80 leading-relaxed">
                    The pool is capped at <span
                        class="text-foreground font-bold underline"
                        >3 editors</span
                    >. As you activate slots 4 and 5, the system will "steal"
                    the editors from slots 1 and 2 (LRU policy).
                </p>
            </div>
        </div>
    </div>

    <!-- Bottom Diagnostics -->
    <div class="grid grid-cols-12 gap-6">
        <!-- Event Logs -->
        <section
            class="col-span-8 bg-muted/10 p-4 rounded-2xl border border-border h-[260px] flex flex-col"
        >
            <h2
                class="text-[10px] font-bold uppercase text-muted-foreground tracking-widest mb-3 border-b border-border pb-2 flex justify-between"
            >
                <span>Telemetry Logs</span>
                <span class="text-[8px] opacity-50">REAL-TIME FEED</span>
            </h2>
            <div
                class="flex-1 overflow-auto space-y-1 pr-2 scrollbar-thin scrollbar-thumb-border"
            >
                {#each logs as l}
                    <div
                        class="pl-2 py-0.5 text-muted-foreground font-mono text-[10px] hover:text-foreground transition-colors border-l border-transparent hover:border-accent"
                    >
                        {l}
                    </div>
                {:else}
                    <div class="text-muted-foreground/30 italic text-[10px]">
                        Initializing system telemetry...
                    </div>
                {/each}
            </div>
        </section>

        <!-- System Instruction -->
        <div
            class="col-span-4 bg-muted/10 border border-border rounded-2xl p-6 flex flex-col justify-center items-center text-center space-y-4 shadow-inner"
        >
            <div
                class="text-[10px] text-muted-foreground font-bold uppercase tracking-widest px-3 py-1 bg-muted rounded-full"
            >
                System Instruction
            </div>
            <p
                class="text-xs text-muted-foreground max-w-xs leading-relaxed italic"
            >
                "Use Arrow Keys and shortcuts in any active slot to verify the
                keyboard delegation remains intact after editor movement."
            </p>
            <div class="flex gap-1.5">
                <div class="w-1 h-1 rounded-full bg-accent/50"></div>
                <div class="w-1 h-1 rounded-full bg-accent/30"></div>
                <div class="w-1 h-1 rounded-full bg-accent/10"></div>
            </div>
        </div>
    </div>

    <!-- Health Panel Overlay -->
    <MonacoHealthPanel />
</div>

<style>
    /* Scrollbar styling */
    .scrollbar-thin::-webkit-scrollbar {
        width: 3px;
    }
    .scrollbar-thin::-webkit-scrollbar-track {
        background: transparent;
    }
    .scrollbar-thin::-webkit-scrollbar-thumb {
        background: var(--theme-border-default);
        border-radius: 10px;
    }
</style>
