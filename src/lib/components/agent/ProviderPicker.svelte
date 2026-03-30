<!-- src/lib/components/agent/ProviderPicker.svelte -->
<script lang="ts">
    import { PROVIDER_CONFIGS } from "$lib/agent/providers";
    import IconCheck from "@tabler/icons-svelte/icons/check";
    import IconChevronDown from "@tabler/icons-svelte/icons/chevron-down";
    import IconSettings from "@tabler/icons-svelte/icons/settings";
    import * as Menu from "$lib/components/ui/dropdown-menu";
    import { goto } from "$app/navigation";

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

    // USER-FACING providers only — filter out internal harness aliases like "gemini"
    const USER_PROVIDERS = new Set(Object.keys(PROVIDER_CONFIGS));

    const displayProviders = $derived(
        (providers.length > 0 ? providers : Object.entries(PROVIDER_CONFIGS).map(([id, cfg]) => ({
            id,
            label: cfg.label,
            available: false,
        }))).filter(p => USER_PROVIDERS.has(p.id))
    );

    /** Model list for the selected provider's picker */
    const modelList = $derived.by(() => {
        const cfg = PROVIDER_CONFIGS[selected];
        if (!cfg?.supportsModel) return [];

        const pinnedIds = selected === "google" ? pinnedGoogleModels
                        : selected === "openrouter" ? pinnedOpenrouterModels
                        : [];

        // When user has pinned models, show ONLY pinned — they curated this list
        if (pinnedIds.length > 0) {
            return pinnedIds.map(id => {
                const known = cfg.models.find(m => m.id === id);
                return known ?? { id, label: id.split("/").pop() ?? id };
            });
        }

        return cfg.models;
    });

    const currentModelLabel = $derived.by(() => {
        const found = modelList.find(m => m.id === currentModel);
        if (found) return found.label;
        // Fallback for IDs not in static config (e.g. fetched from API)
        return currentModel?.split("/").pop() ?? currentModel ?? "Model";
    });

    function handleClick(p: AvailableProvider) {
        if (!p.available) return;
        onProviderChange(p.id);
    }
</script>

<div class="flex flex-col items-center gap-4 px-4 py-5">
    <div class="flex flex-col items-center gap-1">
        <p class="text-[11.5px] font-medium text-foreground/80">Choose a provider</p>
        <p class="text-[10px] text-muted-foreground/50 text-center">Locked once you send your first message</p>
    </div>

    <!-- Provider grid -->
    <div class="grid grid-cols-2 gap-1.5 w-full max-w-[260px]">
        {#each displayProviders as p}
            {@const isSelected = p.id === selected}
            <button
                onclick={() => handleClick(p)}
                disabled={!p.available}
                class="flex flex-col items-start rounded-md border px-3 py-2 text-left transition-all
                    {isSelected
                        ? 'border-accent/50 bg-accent/8 cursor-pointer'
                        : p.available
                            ? 'border-border/40 bg-muted/30 cursor-pointer hover:border-border hover:bg-muted/60'
                            : 'border-border/20 bg-transparent cursor-not-allowed opacity-30'}"
                title={p.available ? `Use ${p.label}` : `${p.label} not available`}
            >
                <div class="flex items-center gap-1.5 w-full">
                    <span class="w-1.5 h-1.5 rounded-full shrink-0
                        {isSelected ? 'bg-accent' : p.available ? 'bg-green-500/70' : 'bg-muted-foreground/20'}">
                    </span>
                    <span class="text-[10.5px] font-medium leading-none
                        {isSelected ? 'text-accent' : p.available ? 'text-foreground/80' : 'text-foreground/40'}">
                        {p.label}
                    </span>
                </div>
                {#if !p.available}
                    <span class="text-[9px] text-muted-foreground/35 mt-1 pl-3">
                        {p.id === "google" || p.id === "openrouter" ? "No API key" : "Not installed"}
                    </span>
                {/if}
            </button>
        {/each}
    </div>

    <!-- Quick link to AI settings -->
    <button
        onclick={() => goto("/settings?section=ai")}
        class="flex items-center gap-1 text-[10px] text-muted-foreground/40 hover:text-muted-foreground transition-colors"
    >
        <IconSettings size={10} />
        Configure providers
    </button>

    <!-- Model picker — custom dropdown, only for providers that support it -->
    {#if modelList.length > 0}
        <div class="w-full max-w-[260px] flex flex-col gap-1">
            <span class="text-[9.5px] text-muted-foreground/40 uppercase tracking-wider font-medium">Model</span>
            <Menu.Root>
                <Menu.Trigger>
                    <button class="w-full flex items-center justify-between gap-2 rounded-md border border-border/40 bg-muted/30 px-2.5 py-1.5 text-left hover:border-border hover:bg-muted/60 transition-all">
                        <span class="text-[10.5px] font-mono text-foreground/75 truncate">{currentModelLabel}</span>
                        <IconChevronDown size={10} class="shrink-0 text-muted-foreground/40" />
                    </button>
                </Menu.Trigger>
                <Menu.Content class="w-56 border border-border bg-background shadow-md p-1" align="start" side="top">
                    {#each modelList as m}
                        <Menu.Item
                            class="flex items-center justify-between gap-2 px-2 py-1.5 text-[10.5px] font-mono rounded cursor-pointer"
                            onclick={() => onModelChange(m.id)}
                        >
                            <span class="truncate">{m.label}</span>
                            {#if currentModel === m.id}
                                <IconCheck size={10} class="shrink-0 text-accent" />
                            {/if}
                        </Menu.Item>
                    {/each}
                </Menu.Content>
            </Menu.Root>
        </div>
    {/if}
</div>
