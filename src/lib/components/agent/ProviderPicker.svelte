<!-- src/lib/components/agent/ProviderPicker.svelte -->
<script lang="ts">
    import { PROVIDER_CONFIGS } from "$lib/agent/providers";

    export interface AvailableProvider {
        id: string;
        label: string;
        available: boolean;
    }

    interface Props {
        providers: AvailableProvider[];
        selected: string;
        currentModel: string;
        pinnedGoogleModels: string[];
        pinnedOpenrouterModels: string[];
        onProviderChange: (id: string) => void;
        onModelChange: (model: string) => void;
    }

    let { providers, selected, currentModel, pinnedGoogleModels, pinnedOpenrouterModels, onProviderChange, onModelChange }: Props = $props();

    // Fallback: if no providers passed yet, show all from PROVIDER_CONFIGS as unavailable
    const displayProviders = $derived(
        providers.length > 0
            ? providers
            : Object.entries(PROVIDER_CONFIGS).map(([id, cfg]) => ({
                  id,
                  label: cfg.label,
                  available: false,
              }))
    );

    /** Models available for the currently selected provider */
    const availableModels = $derived.by(() => {
        const cfg = PROVIDER_CONFIGS[selected];
        if (!cfg || !cfg.supportsModel) return [];

        if (selected === "google") {
            if (pinnedGoogleModels.length > 0) {
                // Show pinned models with labels where known
                return pinnedGoogleModels.map(id => {
                    const known = cfg.models.find(m => m.id === id);
                    return known ?? { id, label: id };
                });
            }
            return cfg.models;
        }

        if (selected === "openrouter") {
            if (pinnedOpenrouterModels.length > 0) {
                return pinnedOpenrouterModels.map(id => {
                    const known = cfg.models.find(m => m.id === id);
                    return known ?? { id, label: id.split("/").pop() ?? id };
                });
            }
            return cfg.models;
        }

        return cfg.models;
    });

    function handleClick(p: AvailableProvider) {
        if (!p.available) return;
        onProviderChange(p.id);
    }
</script>

<div class="flex flex-col items-center gap-3 px-4 py-5">
    <p class="text-[11px] font-medium text-foreground/70">Choose a provider</p>
    <p class="text-[10.5px] text-muted-foreground/60 text-center leading-relaxed max-w-[200px]">
        Locked for the session once you send your first message.
    </p>
    <div class="grid grid-cols-2 gap-2 w-full max-w-[240px]">
        {#each displayProviders as p}
            <button
                onclick={() => handleClick(p)}
                disabled={!p.available}
                class="flex flex-col items-start gap-1.5 rounded-lg border px-3 py-2.5 text-left transition-all
                    {p.available
                        ? p.id === selected
                            ? 'border-accent/60 bg-accent/5 cursor-pointer ring-1 ring-accent/50'
                            : 'border-border/50 bg-background hover:border-border cursor-pointer hover:bg-foreground/[0.02] ring-1 ring-green-600/35'
                        : 'border-border/20 bg-background/50 cursor-not-allowed opacity-35'}"
                title={p.available ? `Use ${p.label}` : `${p.label} not available`}
            >
                <span class="text-[10.5px] font-medium {p.id === selected ? 'text-accent' : 'text-foreground/80'}">
                    {p.label}
                </span>
                {#if !p.available}
                    <span class="text-[9px] text-muted-foreground/40">
                        {p.id === "google" || p.id === "openrouter" ? "No API key" : "Not installed"}
                    </span>
                {/if}
            </button>
        {/each}
    </div>

    <!-- Model selector — shown for providers that support model selection -->
    {#if availableModels.length > 0}
        <div class="w-full max-w-[240px] flex flex-col gap-1">
            <label for="provider-model-select" class="text-[10px] text-muted-foreground/50 uppercase tracking-wider">Model</label>
            <select id="provider-model-select"
                value={currentModel}
                onchange={(e) => onModelChange((e.target as HTMLSelectElement).value)}
                class="w-full rounded-md border border-border/50 bg-background px-2.5 py-1.5 text-[11px] text-foreground focus:border-accent/60 focus:outline-none focus:ring-1 focus:ring-accent/40"
            >
                {#each availableModels as m}
                    <option value={m.id}>{m.label}</option>
                {/each}
            </select>
        </div>
    {/if}
</div>
