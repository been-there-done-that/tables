<!-- src/routes/settings/ai/HarnessLog.svelte -->
<script lang="ts">
    import { harnessLogStore } from "$lib/stores/harnessLog.svelte";

    let expanded = $state(false);
    let copied = $state(false);

    function copyLogs() {
        const text = harnessLogStore.entries
            .map(e => `${new Date(e.ts).toISOString()} [${e.level}] [${e.tag}] ${e.message}`)
            .join("\n");
        navigator.clipboard.writeText(text);
        copied = true;
        setTimeout(() => { copied = false; }, 1500);
    }

    function fmtTs(ts: number): string {
        const d = new Date(ts);
        return d.toLocaleTimeString("en-US", { hour12: false, hour: "2-digit", minute: "2-digit", second: "2-digit" });
    }

    const levelColor: Record<string, string> = {
        debug: "text-blue-400/70",
        info:  "text-green-400",
        warn:  "text-amber-400",
        error: "text-red-400",
    };
</script>

<div class="border border-border rounded bg-muted/60 overflow-hidden shrink-0">
    <!-- Header -->
    <div class="flex items-center justify-between px-3 py-1.5 border-b border-border/50">
        <div class="flex items-center gap-2">
            <span class="text-[10px] text-muted-foreground uppercase tracking-wider font-medium">Harness Log</span>
            <span class="w-1.5 h-1.5 rounded-full bg-green-400 animate-pulse"></span>
        </div>
        <div class="flex items-center gap-3">
            <button
                onclick={copyLogs}
                disabled={harnessLogStore.entries.length === 0}
                class="text-[10px] text-muted-foreground hover:text-foreground transition-colors disabled:opacity-30"
            >{copied ? "✓ copied" : "Copy"}</button>
            <button
                onclick={() => harnessLogStore.clear()}
                class="text-[10px] text-muted-foreground hover:text-foreground transition-colors"
            >Clear</button>
            <button
                onclick={() => (expanded = !expanded)}
                class="text-[10px] text-muted-foreground hover:text-foreground transition-colors"
            >{expanded ? "▼ collapse" : "▶ expand"}</button>
        </div>
    </div>

    {#if expanded}
        <div class="overflow-y-auto max-h-36 font-mono" style="scrollbar-width:thin">
            {#if harnessLogStore.entries.length === 0}
                <div class="px-3 py-3 text-[10px] text-muted-foreground/50">No log entries yet</div>
            {:else}
                {#each harnessLogStore.entries as entry, i (i)}
                    <div class="flex gap-2 px-3 py-0.5 border-b border-border/30 text-[10px]">
                        <span class="text-muted-foreground/50 shrink-0">{fmtTs(entry.ts)}</span>
                        <span class="{levelColor[entry.level] ?? 'text-muted-foreground'} shrink-0">[{entry.level}]</span>
                        <span class="text-muted-foreground/60 shrink-0">[{entry.tag}]</span>
                        <span class="text-muted-foreground truncate">{entry.message}</span>
                    </div>
                {/each}
            {/if}
        </div>
    {:else}
        <!-- Collapsed: show last log line as preview -->
        {#if harnessLogStore.entries.length > 0}
            {@const last = harnessLogStore.entries.at(-1)!}
            <div class="px-3 py-1 text-[10px] font-mono text-muted-foreground/50 truncate">
                {fmtTs(last.ts)} <span class="{levelColor[last.level] ?? ''}">[{last.level}]</span> [{last.tag}] {last.message}
            </div>
        {:else}
            <div class="px-3 py-1 text-[10px] text-muted-foreground/40">No log entries</div>
        {/if}
    {/if}
</div>
