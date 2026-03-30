<!-- src/routes/settings/ai/ModelGrid.svelte -->
<script lang="ts">
    export interface ModelEntry {
        id: string;
        contextLength?: number;
        pricingIn?: number;   // cost per token in USD (multiply by 1M to get per-1M price)
        pricingOut?: number;  // cost per token in USD
    }

    let {
        models,
        pinned,
        onToggle,
    }: {
        models: ModelEntry[];
        pinned: string[];
        onToggle: (id: string) => void;
    } = $props();

    let search = $state("");

    const filtered = $derived(
        search.trim()
            ? models.filter((m) => m.id.toLowerCase().includes(search.trim().toLowerCase()))
            : models
    );

    function fmtCtx(n: number | undefined): string {
        if (!n) return "";
        if (n >= 1_000_000) return `${n / 1_000_000}M`;
        if (n >= 1_000) return `${Math.round(n / 1_000)}k`;
        return String(n);
    }

    function fmtPrice(n: number | undefined): string {
        if (n === undefined || n === null) return "";
        // n is per-token cost; convert to per-1M
        const per1M = n * 1_000_000;
        if (per1M < 1) return `$${per1M.toFixed(2)}`;
        return `$${per1M % 1 === 0 ? per1M : per1M.toFixed(2)}`;
    }
</script>

<div class="flex flex-col gap-2 min-h-0 flex-1">
    <!-- Search + count row -->
    <div class="flex items-center gap-2">
        <div class="relative flex-1">
            <svg class="absolute left-2 top-1/2 -translate-y-1/2 opacity-40 pointer-events-none" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <circle cx="11" cy="11" r="8"/><path d="m21 21-4.35-4.35"/>
            </svg>
            <input
                bind:value={search}
                placeholder="Search models…"
                class="w-full bg-muted border border-border rounded pl-7 pr-3 py-1 text-[11px] font-mono text-foreground placeholder:text-muted-foreground/40 focus:outline-none focus:ring-1 focus:ring-accent"
            />
        </div>
        <span class="text-[10px] text-muted-foreground whitespace-nowrap">
            {filtered.length} of {models.length}
        </span>
    </div>

    <!-- 2-column grid -->
    <div class="grid grid-cols-2 gap-1 overflow-y-auto flex-1 pr-1 min-h-0" style="scrollbar-width:thin">
        {#each filtered as model (model.id)}
            {@const isPinned = pinned.includes(model.id)}
            <div
                class="flex items-center gap-2 rounded px-2.5 py-1.5 border transition-colors cursor-default
                    {isPinned
                        ? 'border-accent/40 bg-accent/5'
                        : 'border-border bg-muted/60'}"
            >
                <!-- Info -->
                <div class="flex-1 min-w-0 flex flex-col gap-0.5">
                    <span
                        class="text-[10px] font-mono truncate
                            {isPinned ? 'text-accent' : 'text-foreground'}"
                        title={model.id}
                    >{model.id}</span>
                    <div class="flex gap-1">
                        {#if model.contextLength}
                            <span class="text-[9px] px-1 rounded border bg-blue-950/30 border-blue-800/30 text-blue-400">{fmtCtx(model.contextLength)}</span>
                        {/if}
                        {#if model.pricingIn !== undefined}
                            <span class="text-[9px] px-1 rounded border bg-green-950/30 border-green-800/30 text-green-400">↓{fmtPrice(model.pricingIn)}</span>
                        {/if}
                        {#if model.pricingOut !== undefined}
                            <span class="text-[9px] px-1 rounded border bg-amber-950/30 border-amber-800/30 text-amber-400">↑{fmtPrice(model.pricingOut)}</span>
                        {/if}
                    </div>
                </div>

                <!-- Toggle -->
                <button
                    onclick={() => onToggle(model.id)}
                    class="relative shrink-0 w-7 h-4 rounded-full transition-colors focus:outline-none
                        {isPinned ? 'bg-accent' : 'bg-border'}"
                    title={isPinned ? "Remove from picker" : "Add to picker"}
                >
                    <span
                        class="absolute top-0.5 w-3 h-3 rounded-full bg-white shadow transition-all
                            {isPinned ? 'left-[14px]' : 'left-0.5'}"
                    ></span>
                </button>
            </div>
        {/each}

        {#if filtered.length === 0}
            <div class="col-span-2 text-center text-[11px] text-muted-foreground py-6">
                {search ? "No models match your search" : "No models — click Fetch to load"}
            </div>
        {/if}
    </div>
</div>
