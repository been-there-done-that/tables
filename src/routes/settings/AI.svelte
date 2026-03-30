<!-- src/routes/settings/AI.svelte -->
<script lang="ts">
    import { settingsStore, type CachedModel } from "$lib/stores/settings.svelte";
    import ModelGrid from "./ai/ModelGrid.svelte";
    import HarnessLog from "./ai/HarnessLog.svelte";

    type ProviderTab = "claude" | "google" | "openrouter" | "codex" | "opencode" | "cursor";
    let activeTab = $state<ProviderTab>("claude");

    // ── Google ──────────────────────────────────────────────────────────
    let googleModels = $state<CachedModel[]>([]);
    let googleFetchStatus = $state<"idle" | "loading" | "ok" | "error">("idle");
    let googleFetchError = $state("");

    // Reactively seed from cache once settings finish loading from SQLite
    $effect(() => {
        const cached = settingsStore.googleCachedModels;
        if (cached.length > 0 && googleFetchStatus === "idle") {
            googleModels = cached;
            googleFetchStatus = "ok";
        }
    });

    async function fetchGoogleModels() {
        const key = settingsStore.googleApiKey;
        if (!key) { googleFetchError = "Enter an API key first"; googleFetchStatus = "error"; return; }
        googleFetchStatus = "loading";
        try {
            const res = await fetch(
                `https://generativelanguage.googleapis.com/v1beta/models?key=${encodeURIComponent(key)}`
            );
            if (!res.ok) throw new Error(`HTTP ${res.status}`);
            const data = await res.json();
            googleModels = (data.models ?? [])
                .filter((m: any) => m.name && m.supportedGenerationMethods?.includes("generateContent"))
                .map((m: any) => ({
                    id: m.name.replace("models/", ""),
                    contextLength: m.inputTokenLimit,
                }));
            settingsStore.googleCachedModels = googleModels;
            googleFetchStatus = "ok";
            googleFetchError = "";
        } catch (e) {
            googleFetchError = String(e);
            googleFetchStatus = "error";
        }
    }

    function toggleGooglePin(id: string) {
        const current = settingsStore.googlePinnedModels;
        settingsStore.googlePinnedModels = current.includes(id)
            ? current.filter((m) => m !== id)
            : [...current, id];
    }

    // ── OpenRouter ──────────────────────────────────────────────────────
    let orModels = $state<CachedModel[]>([]);
    let orFetchStatus = $state<"idle" | "loading" | "ok" | "error">("idle");
    let orFetchError = $state("");

    // Reactively seed from cache once settings finish loading from SQLite
    $effect(() => {
        const cached = settingsStore.openrouterCachedModels;
        if (cached.length > 0 && orFetchStatus === "idle") {
            orModels = cached;
            orFetchStatus = "ok";
        }
    });

    async function fetchOpenRouterModels() {
        const key = settingsStore.openrouterApiKey;
        const baseUrl = settingsStore.openrouterBaseUrl || "https://openrouter.ai/api/v1";
        if (!key) { orFetchError = "Enter an API key first"; orFetchStatus = "error"; return; }
        orFetchStatus = "loading";
        try {
            const res = await fetch(`${baseUrl}/models`, {
                headers: { Authorization: `Bearer ${key}` },
            });
            if (!res.ok) throw new Error(`HTTP ${res.status}`);
            const data = await res.json();
            orModels = (data.data ?? []).map((m: any) => ({
                id: m.id,
                contextLength: m.context_length ?? undefined,
                pricingIn: m.pricing?.prompt ? parseFloat(m.pricing.prompt) : undefined,
                pricingOut: m.pricing?.completion ? parseFloat(m.pricing.completion) : undefined,
            }));
            settingsStore.openrouterCachedModels = orModels;
            orFetchStatus = "ok";
            orFetchError = "";
        } catch (e) {
            orFetchError = String(e);
            orFetchStatus = "error";
        }
    }

    function toggleOrPin(id: string) {
        const current = settingsStore.openrouterPinnedModels;
        settingsStore.openrouterPinnedModels = current.includes(id)
            ? current.filter((m) => m !== id)
            : [...current, id];
    }

    const tabs: { id: ProviderTab; label: string; hasKey: boolean | null }[] = $derived([
        { id: "claude",     label: "Claude",     hasKey: null },
        { id: "google",     label: "Google",     hasKey: !!settingsStore.googleApiKey },
        { id: "openrouter", label: "OpenRouter", hasKey: !!settingsStore.openrouterApiKey },
        { id: "codex",      label: "Codex",      hasKey: null },
        { id: "opencode",   label: "OpenCode",   hasKey: null },
        { id: "cursor",     label: "Cursor",     hasKey: null },
    ]);
</script>

<div class="flex flex-col h-full overflow-hidden">

    <!-- Provider tab strip -->
    <div class="flex border-b border-border px-4 shrink-0">
        {#each tabs as tab}
            <button
                onclick={() => (activeTab = tab.id)}
                class="flex items-center gap-1.5 px-4 py-2.5 text-[11.5px] border-b-2 transition-colors
                    {activeTab === tab.id
                        ? 'border-accent text-accent font-medium'
                        : 'border-transparent text-muted-foreground hover:text-foreground'}"
            >
                <span class="w-1.5 h-1.5 rounded-full
                    {tab.hasKey === null ? 'bg-green-400'
                    : tab.hasKey         ? 'bg-green-400'
                    :                      'bg-amber-400'}">
                </span>
                {tab.label}
            </button>
        {/each}
    </div>

    <!-- Tab content -->
    <div class="flex-1 overflow-hidden flex flex-col gap-3 p-5 min-h-0">

        {#if activeTab === "claude"}
            <div class="text-sm text-muted-foreground">
                Claude is configured via the <code class="text-accent text-[11px]">claude</code> CLI — no API key needed here.
                Model and effort settings are available in the agent panel.
            </div>

        {:else if activeTab === "google"}
            <!-- API key + Fetch -->
            <div class="flex gap-2 items-end">
                <div class="flex flex-col gap-1 flex-1">
                    <span class="text-[10px] text-muted-foreground uppercase tracking-wide font-medium">API Key</span>
                    <input
                        type="password"
                        placeholder="AIza…"
                        value={settingsStore.googleApiKey}
                        oninput={(e) => { settingsStore.googleApiKey = (e.target as HTMLInputElement).value; }}
                        class="bg-muted border border-border rounded px-2.5 py-1.5 text-[11px] font-mono text-foreground placeholder:text-muted-foreground/40 focus:outline-none focus:ring-1 focus:ring-accent"
                    />
                </div>
                <button
                    onclick={fetchGoogleModels}
                    disabled={googleFetchStatus === "loading"}
                    class="flex items-center gap-1.5 px-3 py-1.5 rounded border border-accent/30 bg-accent/10 text-accent text-[11px] font-medium hover:bg-accent/20 transition-colors disabled:opacity-50"
                >
                    {#if googleFetchStatus === "loading"}
                        <span class="animate-spin inline-block w-3 h-3 border-2 border-accent border-t-transparent rounded-full"></span>
                    {:else}
                        ↻
                    {/if}
                    Fetch models
                </button>
            </div>

            <!-- Status -->
            {#if googleFetchStatus === "ok"}
                <div class="flex items-center gap-2 text-[10.5px] text-green-400 bg-green-950/20 border border-green-800/30 rounded px-3 py-1.5">
                    <span class="w-1.5 h-1.5 rounded-full bg-green-400"></span>
                    {googleModels.length} models available · {settingsStore.googlePinnedModels.length} pinned
                </div>
            {:else if googleFetchStatus === "error"}
                <div class="text-[10.5px] text-red-400 bg-red-950/20 border border-red-800/30 rounded px-3 py-1.5">{googleFetchError}</div>
            {/if}

            <ModelGrid
                models={googleModels}
                pinned={settingsStore.googlePinnedModels}
                onToggle={toggleGooglePin}
            />

        {:else if activeTab === "openrouter"}
            <!-- API key + Base URL + Fetch -->
            <div class="flex gap-2 items-end">
                <div class="flex flex-col gap-1" style="flex:1.2">
                    <span class="text-[10px] text-muted-foreground uppercase tracking-wide font-medium">API Key</span>
                    <input
                        type="password"
                        placeholder="sk-or-v1-…"
                        value={settingsStore.openrouterApiKey}
                        oninput={(e) => { settingsStore.openrouterApiKey = (e.target as HTMLInputElement).value; }}
                        class="bg-muted border border-border rounded px-2.5 py-1.5 text-[11px] font-mono text-foreground placeholder:text-muted-foreground/40 focus:outline-none focus:ring-1 focus:ring-accent"
                    />
                </div>
                <div class="flex flex-col gap-1" style="flex:2">
                    <span class="text-[10px] text-muted-foreground uppercase tracking-wide font-medium">
                        Base URL <span class="normal-case text-muted-foreground/50 tracking-normal font-normal">(override)</span>
                    </span>
                    <input
                        type="text"
                        placeholder="https://openrouter.ai/api/v1"
                        value={settingsStore.openrouterBaseUrl}
                        oninput={(e) => { settingsStore.openrouterBaseUrl = (e.target as HTMLInputElement).value; }}
                        class="bg-muted border border-border rounded px-2.5 py-1.5 text-[11px] font-mono text-foreground placeholder:text-muted-foreground/30 focus:outline-none focus:ring-1 focus:ring-accent"
                    />
                </div>
                <button
                    onclick={fetchOpenRouterModels}
                    disabled={orFetchStatus === "loading"}
                    class="flex items-center gap-1.5 px-3 py-1.5 rounded border border-accent/30 bg-accent/10 text-accent text-[11px] font-medium hover:bg-accent/20 transition-colors disabled:opacity-50"
                >
                    {#if orFetchStatus === "loading"}
                        <span class="animate-spin inline-block w-3 h-3 border-2 border-accent border-t-transparent rounded-full"></span>
                    {:else}
                        ↻
                    {/if}
                    Fetch models
                </button>
            </div>

            <!-- Status -->
            {#if orFetchStatus === "ok"}
                <div class="flex items-center gap-2 text-[10.5px] text-green-400 bg-green-950/20 border border-green-800/30 rounded px-3 py-1.5">
                    <span class="w-1.5 h-1.5 rounded-full bg-green-400"></span>
                    {orModels.length} models available · {settingsStore.openrouterPinnedModels.length} pinned
                </div>
            {:else if orFetchStatus === "error"}
                <div class="text-[10.5px] text-red-400 bg-red-950/20 border border-red-800/30 rounded px-3 py-1.5">{orFetchError}</div>
            {/if}

            <ModelGrid
                models={orModels}
                pinned={settingsStore.openrouterPinnedModels}
                onToggle={toggleOrPin}
            />

        {:else}
            <div class="text-sm text-muted-foreground">
                Configuration for <strong>{activeTab}</strong> is coming soon.
            </div>
        {/if}

    </div>

    <!-- Harness log panel — always visible at bottom -->
    <div class="px-5 pb-4 shrink-0">
        <HarnessLog />
    </div>
</div>
