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
        onProviderChange: (id: string) => void;
    }

    let { providers, selected, onProviderChange }: Props = $props();

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
                            ? 'border-accent/60 bg-accent/5 cursor-pointer'
                            : 'border-border/50 bg-background hover:border-border cursor-pointer hover:bg-foreground/[0.02]'
                        : 'border-border/20 bg-background/50 cursor-not-allowed opacity-35'}"
                style={p.available
                    ? p.id === selected
                        ? 'box-shadow: 0 0 0 1.5px hsl(var(--accent) / 0.5)'
                        : 'box-shadow: 0 0 0 1.5px hsl(142 76% 36% / 0.35)'
                    : ''}
                title={p.available ? `Use ${p.label}` : `${p.label} not installed`}
            >
                <span class="text-[10.5px] font-medium {p.id === selected ? 'text-accent' : 'text-foreground/80'}">
                    {p.label}
                </span>
                {#if !p.available}
                    <span class="text-[9px] text-muted-foreground/40">Not installed</span>
                {/if}
            </button>
        {/each}
    </div>
</div>
