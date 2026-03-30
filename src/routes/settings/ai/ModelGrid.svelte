<!-- src/routes/settings/ai/ModelGrid.svelte -->
<script lang="ts">
    import type { CachedModel } from "$lib/stores/settings.svelte";

    let {
        models,
        pinned,
        onToggle,
    }: {
        models: CachedModel[];
        pinned: string[];
        onToggle: (id: string) => void;
    } = $props();

    let search = $state("");

    const searchLower = $derived(search.trim().toLowerCase());

    /**
     * Pinned models in saved order — shows stubs for IDs not yet fetched.
     * This means pinned models always appear even before clicking Fetch.
     */
    const pinnedModels = $derived(
        (searchLower
            ? pinned.filter(id => id.toLowerCase().includes(searchLower))
            : pinned
        ).map(id => models.find(m => m.id === id) ?? { id })
    );

    const pinnedSet = $derived(new Set(pinned));

    /** Fetched models that are not pinned, subject to search filter */
    const unpinnedModels = $derived(
        models.filter(m =>
            !pinnedSet.has(m.id) &&
            (!searchLower || m.id.toLowerCase().includes(searchLower))
        )
    );

    const totalVisible = $derived(pinnedModels.length + unpinnedModels.length);
    const totalAll = $derived(models.length + pinned.filter(id => !models.find(m => m.id === id)).length);

    function fmtCtx(n: number | undefined): string {
        if (!n) return "";
        if (n >= 1_000_000) return `${n / 1_000_000}M`;
        if (n >= 1_000) return `${Math.round(n / 1_000)}k`;
        return String(n);
    }

    function fmtPrice(n: number | undefined): string {
        if (n === undefined || n === null) return "";
        const per1M = n * 1_000_000;
        if (per1M < 1) return `$${per1M.toFixed(2)}`;
        return `$${per1M % 1 === 0 ? per1M : per1M.toFixed(2)}`;
    }
</script>

{#snippet modelMeta(model: CachedModel)}
    {@const parts: string[] = []}
    {@const ctx = model.contextLength ? fmtCtx(model.contextLength) : null}
    {@const prIn = model.pricingIn !== undefined ? `↓${fmtPrice(model.pricingIn)}` : null}
    {@const prOut = model.pricingOut !== undefined ? `↑${fmtPrice(model.pricingOut)}` : null}
    {#if ctx || prIn || prOut}
        <span class="text-[9px] font-mono text-muted-foreground/50 tracking-tight leading-none">
            {[ctx, prIn, prOut].filter(Boolean).join(" · ")}
        </span>
    {/if}
{/snippet}

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
            {totalVisible} of {totalAll}
        </span>
    </div>

    <div class="overflow-y-auto flex-1 flex flex-col gap-3 pr-1 min-h-0" style="scrollbar-width:thin">

        <!-- ── Pinned section ──────────────────────────────────── -->
        {#if pinnedModels.length > 0}
            <div class="flex flex-col gap-1">
                <div class="flex items-center gap-2">
                    <span class="text-[9px] font-medium uppercase tracking-wider text-accent/70">Pinned</span>
                    <span class="flex-1 h-px bg-accent/20"></span>
                    <span class="text-[9px] text-muted-foreground/40">{pinnedModels.length}</span>
                </div>
                <div class="rounded-md border border-accent/25 bg-accent/[0.04] p-1.5 grid grid-cols-2 gap-1">
                    {#each pinnedModels as model (model.id)}
                        <div class="flex items-center gap-2 rounded px-2 py-1.5 bg-accent/5 border border-accent/30">
                            <div class="flex-1 min-w-0 flex flex-col gap-1">
                                <span class="text-[10px] font-mono truncate text-accent leading-tight" title={model.id}>
                                    {model.id}
                                </span>
                                {@render modelMeta(model)}
                            </div>
                            <button
                                onclick={() => onToggle(model.id)}
                                class="relative shrink-0 w-7 h-4 rounded-full bg-accent transition-colors focus:outline-none"
                                title="Remove from picker"
                            >
                                <span class="absolute top-0.5 left-[14px] w-3 h-3 rounded-full bg-white shadow transition-all"></span>
                            </button>
                        </div>
                    {/each}
                </div>
            </div>
        {/if}

        <!-- ── All other models ────────────────────────────────── -->
        {#if unpinnedModels.length > 0}
            <div class="flex flex-col gap-1">
                {#if pinnedModels.length > 0}
                    <div class="flex items-center gap-2">
                        <span class="text-[9px] font-medium uppercase tracking-wider text-muted-foreground/50">All models</span>
                        <span class="flex-1 h-px bg-border/60"></span>
                        <span class="text-[9px] text-muted-foreground/40">{unpinnedModels.length}</span>
                    </div>
                {/if}
                <div class="grid grid-cols-2 gap-1">
                    {#each unpinnedModels as model (model.id)}
                        <div class="flex items-center gap-2 rounded px-2.5 py-1.5 border border-border bg-muted/60 transition-colors">
                            <div class="flex-1 min-w-0 flex flex-col gap-1">
                                <span class="text-[10px] font-mono truncate text-foreground leading-tight" title={model.id}>
                                    {model.id}
                                </span>
                                {@render modelMeta(model)}
                            </div>
                            <button
                                onclick={() => onToggle(model.id)}
                                class="relative shrink-0 w-7 h-4 rounded-full bg-border transition-colors focus:outline-none"
                                title="Add to picker"
                            >
                                <span class="absolute top-0.5 left-0.5 w-3 h-3 rounded-full bg-white shadow transition-all"></span>
                            </button>
                        </div>
                    {/each}
                </div>
            </div>
        {/if}

        {#if totalVisible === 0}
            <div class="text-center text-[11px] text-muted-foreground py-6">
                {search ? "No models match your search" : "No models — click Fetch to load"}
            </div>
        {/if}
    </div>
</div>
