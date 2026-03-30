# AI Provider Settings Page — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a dedicated Settings → AI page where users can configure API keys, base URLs, fetch and browse live model lists per provider, and view harness debug logs — removing the inline API key input from the chat composer.

**Architecture:** New `AI.svelte` component added to the existing `/settings` sidebar pattern. Provider config (API keys, base URLs, pinned models) persisted via the existing `commandClient.updateAppSetting()` SQLite store. Harness emits structured JSON logs to stderr; Rust captures and forwards them as Tauri events; a new Svelte store holds recent log lines displayed in a collapsible panel.

**Tech Stack:** Svelte 5 (runes), Tauri 2 events, `@tauri-apps/api/event`, `@tabler/icons-svelte`, existing `commandClient`, existing `settingsStore`, Bun harness, serde_json (already in Cargo.toml)

---

## File Map

| File | Action |
|------|--------|
| `src/lib/stores/settings.svelte.ts` | Add 4 new settings fields + getters/setters + event cases |
| `packages/harness/src/index.ts` | Add `hLog()` structured stderr logger; replace `console.error` calls |
| `src-tauri/src/harness.rs` | Pipe stderr; parse JSON log lines; emit `harness://log` Tauri events |
| `src/lib/stores/harnessLog.svelte.ts` | NEW — store for last 200 harness log entries |
| `src/routes/settings/ai/ModelGrid.svelte` | NEW — 2-column model card grid with toggles, search, chips |
| `src/routes/settings/ai/HarnessLog.svelte` | NEW — collapsible log panel |
| `src/routes/settings/AI.svelte` | NEW — provider tabs + per-provider config + model fetch |
| `src/routes/settings/+page.svelte` | Add "AI" sidebar item; render `AI.svelte` |
| `src/lib/components/agent/AgentComposer.svelte` | Remove inline API key input + 2 derived values |
| `src/lib/components/agent/AgentPanel.svelte` | Remove `_lastRestartedKey` + restart-on-key `$effect` |

---

## Task 1: Extend settings store with new AI fields

**Files:**
- Modify: `src/lib/stores/settings.svelte.ts`

- [ ] **Step 1: Add new fields to the `Settings` interface**

In `src/lib/stores/settings.svelte.ts`, find the `Settings` interface (around line 4) and add 4 fields after `openrouterApiKey`:

```ts
export interface Settings {
  // ... existing fields ...
  googleApiKey: string;
  openrouterApiKey: string;
  googleBaseUrl: string;
  openrouterBaseUrl: string;
  googlePinnedModels: string[];
  openrouterPinnedModels: string[];
  lastActiveThreadId: string | null;
}
```

- [ ] **Step 2: Add defaults**

In `DEFAULT_SETTINGS` (around line 32), add after `openrouterApiKey: ""`:

```ts
googleBaseUrl: "",
openrouterBaseUrl: "",
googlePinnedModels: [],
openrouterPinnedModels: [],
```

- [ ] **Step 3: Add getters and setters**

In the store object returned by `createSettingsStore()`, add after the `openrouterApiKey` setter (around line 131):

```ts
get googleBaseUrl(): string {
    return settings.googleBaseUrl;
},
set googleBaseUrl(v: string) {
    settings.googleBaseUrl = v;
    commandClient.updateAppSetting("google_base_url", v);
},
get openrouterBaseUrl(): string {
    return settings.openrouterBaseUrl;
},
set openrouterBaseUrl(v: string) {
    settings.openrouterBaseUrl = v;
    commandClient.updateAppSetting("openrouter_base_url", v);
},
get googlePinnedModels(): string[] {
    return settings.googlePinnedModels;
},
set googlePinnedModels(v: string[]) {
    settings.googlePinnedModels = v;
    commandClient.updateAppSetting("google_pinned_models", JSON.stringify(v));
},
get openrouterPinnedModels(): string[] {
    return settings.openrouterPinnedModels;
},
set openrouterPinnedModels(v: string[]) {
    settings.openrouterPinnedModels = v;
    commandClient.updateAppSetting("openrouter_pinned_models", JSON.stringify(v));
},
```

- [ ] **Step 4: Handle the new keys in `settings-changed` event listener**

In the `switch (key)` block inside the `listen("settings-changed", ...)` callback (around line 369), add after the `openrouter_api_key` case:

```ts
case "google_base_url":
    settings.googleBaseUrl = value || "";
    return;
case "openrouter_base_url":
    settings.openrouterBaseUrl = value || "";
    return;
case "google_pinned_models":
    try { settings.googlePinnedModels = JSON.parse(value || "[]"); } catch { settings.googlePinnedModels = []; }
    return;
case "openrouter_pinned_models":
    try { settings.openrouterPinnedModels = JSON.parse(value || "[]"); } catch { settings.openrouterPinnedModels = []; }
    return;
```

- [ ] **Step 5: Handle pinned models on initial load**

In `createSettingsStore()`, in the `for (const [key, value] of Object.entries(globalSettings))` loop (around line 291), add a JSON parse branch before `// Default to string`:

```ts
// Handle JSON arrays (pinned models)
if (key === "googlePinnedModels" || key === "openrouterPinnedModels") {
    try { parsedSettings[key] = JSON.parse(value as string); } catch { parsedSettings[key] = []; }
    continue;
}
```

- [ ] **Step 6: Run type check**

```bash
cd /Users/__deesh_reddy__/projects/personal_git/rust_builds/tables
pnpm check 2>&1 | grep -E "error|Error" | head -20
```

Expected: No errors related to the new fields.

- [ ] **Step 7: Commit**

```bash
git add src/lib/stores/settings.svelte.ts
git commit -m "feat(settings): add base URL and pinned models fields for Google and OpenRouter"
```

---

## Task 2: Add structured logging to harness

**Files:**
- Modify: `packages/harness/src/index.ts`

- [ ] **Step 1: Add `hLog` helper at the top of `index.ts`**

After the imports (after line 6), add:

```ts
/** Emit a structured log line to stderr for Rust to capture and forward as a Tauri event. */
function hLog(level: "info" | "warn" | "error", tag: string, message: string): void {
    process.stderr.write(JSON.stringify({ ts: Date.now(), level, tag, message }) + "\n");
}
```

- [ ] **Step 2: Replace `console.error` calls with `hLog`**

Replace all `console.error(...)` calls inside `handleRequest` and the session handling code with `hLog(...)`. Use these exact replacements:

```ts
// OLD:
console.error(`[harness] session started: ${sessionId}`);
// NEW:
hLog("info", "session", `started — provider=${provider} model=${model ?? "default"} session=${sessionId}`);

// OLD:
console.error(`[harness] session send: ${sessionId} — "${text.slice(0, 60)}"`);
// NEW:
hLog("info", "send", `session=${sessionId} — "${text.slice(0, 60)}"`);

// OLD:
console.error(`[harness] SSE stream cancelled for ${sessionId}`);
// NEW:
hLog("warn", "send", `SSE stream cancelled session=${sessionId}`);
```

Also add `hLog` calls for tool events. In the `/tool-result/:requestId` handler (the POST that resolves tool calls), add:
```ts
hLog("info", "tool", `result received requestId=${requestId}`);
```

And for errors, replace any `console.error` in catch blocks:
```ts
// OLD:
console.error("[harness] ...", e);
// NEW:
hLog("error", "harness", String(e));
```

> Note: Keep the `console.log(\`HARNESS_PORT=\${server.port}\`)` line unchanged — Rust reads stdout for this.

- [ ] **Step 3: Rebuild the harness binary**

```bash
cd /Users/__deesh_reddy__/projects/personal_git/rust_builds/tables/packages/harness
bun run build
```

Expected: Binary rebuilt at `src-tauri/binaries/harness-aarch64-apple-darwin` (or appropriate triple).

- [ ] **Step 4: Commit**

```bash
cd /Users/__deesh_reddy__/projects/personal_git/rust_builds/tables
git add packages/harness/src/index.ts src-tauri/binaries/
git commit -m "feat(harness): structured JSON log output to stderr"
```

---

## Task 3: Capture harness stderr and emit Tauri log events

**Files:**
- Modify: `src-tauri/src/harness.rs`

- [ ] **Step 1: Change stderr from `inherit` to `piped`**

In `harness.rs` in the `Command::new(&binary_path)` block (around line 68), change:

```rust
// OLD:
.stderr(Stdio::inherit())
// NEW:
.stderr(Stdio::piped())
```

- [ ] **Step 2: Spawn a thread to read stderr and emit log events**

After the `let stdout = child.stdout.take().unwrap();` line (around line 80), add before the stdout `for` loop:

```rust
// Capture stderr: parse JSON log lines and emit as Tauri events
let stderr = child.stderr.take().unwrap();
let app_log = app.clone();
std::thread::spawn(move || {
    for line in BufReader::new(stderr).lines().flatten() {
        // Try to parse structured JSON log
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&line) {
            app_log.emit("harness://log", val).ok();
        } else {
            // Fallback: plain text line (shouldn't happen in normal operation)
            eprintln!("[harness stderr] {}", line);
        }
    }
});
```

- [ ] **Step 3: Verify it compiles**

```bash
cd /Users/__deesh_reddy__/projects/personal_git/rust_builds/tables/src-tauri
cargo check 2>&1 | grep -E "^error" | head -20
```

Expected: No errors.

- [ ] **Step 4: Commit**

```bash
cd /Users/__deesh_reddy__/projects/personal_git/rust_builds/tables
git add src-tauri/src/harness.rs
git commit -m "feat(harness): pipe stderr and emit structured log events to frontend"
```

---

## Task 4: Create harnessLog Svelte store

**Files:**
- Create: `src/lib/stores/harnessLog.svelte.ts`

- [ ] **Step 1: Create the store**

```ts
// src/lib/stores/harnessLog.svelte.ts
import { listen } from "@tauri-apps/api/event";

export interface HarnessLogEntry {
    ts: number;
    level: "info" | "warn" | "error";
    tag: string;
    message: string;
}

const MAX_ENTRIES = 200;

function createHarnessLogStore() {
    let entries = $state<HarnessLogEntry[]>([]);

    // Start listening immediately when this module is imported
    listen<HarnessLogEntry>("harness://log", (event) => {
        entries = [...entries.slice(-(MAX_ENTRIES - 1)), event.payload];
    });

    return {
        get entries() { return entries; },
        clear() { entries = []; },
    };
}

export const harnessLogStore = createHarnessLogStore();
```

- [ ] **Step 2: Run type check**

```bash
cd /Users/__deesh_reddy__/projects/personal_git/rust_builds/tables
pnpm check 2>&1 | grep -E "error|Error" | head -20
```

Expected: No errors.

- [ ] **Step 3: Commit**

```bash
git add src/lib/stores/harnessLog.svelte.ts
git commit -m "feat(stores): add harnessLog store for structured harness log events"
```

---

## Task 5: Create ModelGrid component

**Files:**
- Create: `src/routes/settings/ai/ModelGrid.svelte`

- [ ] **Step 1: Create the component**

```svelte
<!-- src/routes/settings/ai/ModelGrid.svelte -->
<script lang="ts">
    export interface ModelEntry {
        id: string;
        contextLength?: number;
        pricingIn?: number;   // cost per 1M input tokens in USD
        pricingOut?: number;  // cost per 1M output tokens in USD
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
```

- [ ] **Step 2: Run type check**

```bash
cd /Users/__deesh_reddy__/projects/personal_git/rust_builds/tables
pnpm check 2>&1 | grep -E "error|Error" | head -20
```

Expected: No errors.

- [ ] **Step 3: Commit**

```bash
git add src/routes/settings/ai/ModelGrid.svelte
git commit -m "feat(settings): add ModelGrid component with search, toggles, and pricing chips"
```

---

## Task 6: Create HarnessLog component

**Files:**
- Create: `src/routes/settings/ai/HarnessLog.svelte`

- [ ] **Step 1: Create the component**

```svelte
<!-- src/routes/settings/ai/HarnessLog.svelte -->
<script lang="ts">
    import { harnessLogStore } from "$lib/stores/harnessLog.svelte";

    let expanded = $state(false);

    function fmtTs(ts: number): string {
        const d = new Date(ts);
        return d.toLocaleTimeString("en-US", { hour12: false, hour: "2-digit", minute: "2-digit", second: "2-digit" });
    }

    const levelColor: Record<string, string> = {
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
                {#each harnessLogStore.entries as entry (entry.ts + entry.message)}
                    <div class="flex gap-2 px-3 py-0.5 border-b border-border/30 text-[10px]">
                        <span class="text-muted-foreground/50 shrink-0">{fmtTs(entry.ts)}</span>
                        <span class="{levelColor[entry.level] ?? 'text-muted-foreground'} shrink-0">[{entry.tag}]</span>
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
                {fmtTs(last.ts)} [{last.tag}] {last.message}
            </div>
        {:else}
            <div class="px-3 py-1 text-[10px] text-muted-foreground/40">No log entries</div>
        {/if}
    {/if}
</div>
```

- [ ] **Step 2: Run type check**

```bash
cd /Users/__deesh_reddy__/projects/personal_git/rust_builds/tables
pnpm check 2>&1 | grep -E "error|Error" | head -20
```

Expected: No errors.

- [ ] **Step 3: Commit**

```bash
git add src/routes/settings/ai/HarnessLog.svelte
git commit -m "feat(settings): add collapsible HarnessLog panel"
```

---

## Task 7: Create AI.svelte — main AI settings page

**Files:**
- Create: `src/routes/settings/AI.svelte`

- [ ] **Step 1: Create the component**

```svelte
<!-- src/routes/settings/AI.svelte -->
<script lang="ts">
    import { settingsStore } from "$lib/stores/settings.svelte";
    import ModelGrid from "./ai/ModelGrid.svelte";
    import HarnessLog from "./ai/HarnessLog.svelte";
    import type { ModelEntry } from "./ai/ModelGrid.svelte";

    type ProviderTab = "claude" | "google" | "openrouter" | "codex" | "opencode" | "cursor";
    let activeTab = $state<ProviderTab>("claude");

    // ── Google ──────────────────────────────────────────────────────────
    let googleModels = $state<ModelEntry[]>([]);
    let googleFetchStatus = $state<"idle" | "loading" | "ok" | "error">("idle");
    let googleFetchError = $state("");

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
    let orModels = $state<ModelEntry[]>([]);
    let orFetchStatus = $state<"idle" | "loading" | "ok" | "error">("idle");
    let orFetchError = $state("");

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
                contextLength: m.context_length,
                pricingIn: m.pricing?.prompt ? parseFloat(m.pricing.prompt) : undefined,
                pricingOut: m.pricing?.completion ? parseFloat(m.pricing.completion) : undefined,
            }));
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

    // ── Helpers ──────────────────────────────────────────────────────────
    function truncateKey(key: string): string {
        if (!key) return "";
        if (key.length <= 16) return key;
        return key.slice(0, 12) + "…" + key.slice(-3);
    }

    const tabs: { id: ProviderTab; label: string; hasKey: boolean | null }[] = [
        { id: "claude",     label: "Claude",     hasKey: null },
        { id: "google",     label: "Google",     hasKey: !!settingsStore.googleApiKey },
        { id: "openrouter", label: "OpenRouter", hasKey: !!settingsStore.openrouterApiKey },
        { id: "codex",      label: "Codex",      hasKey: null },
        { id: "opencode",   label: "OpenCode",   hasKey: null },
        { id: "cursor",     label: "Cursor",     hasKey: null },
    ];
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
                <!-- status dot -->
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

            <!-- Model grid -->
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

            <!-- Model grid -->
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

    <!-- Harness log panel — always visible at bottom of AI settings -->
    <div class="px-5 pb-4 shrink-0">
        <HarnessLog />
    </div>
</div>
```

- [ ] **Step 2: Run type check**

```bash
cd /Users/__deesh_reddy__/projects/personal_git/rust_builds/tables
pnpm check 2>&1 | grep -E "error|Error" | head -20
```

Expected: No errors.

- [ ] **Step 3: Commit**

```bash
git add src/routes/settings/AI.svelte
git commit -m "feat(settings): add AI provider settings page with tabs, model fetch, and log panel"
```

---

## Task 8: Add AI to the settings sidebar

**Files:**
- Modify: `src/routes/settings/+page.svelte`

- [ ] **Step 1: Import AI icon and AI component**

Add these imports to the `<script>` block in `+page.svelte` (after the existing imports):

```ts
import AiIcon from "@tabler/icons-svelte/icons/ai";
import AIComponent from "./AI.svelte";
```

- [ ] **Step 2: Add AI to the sections array**

In the `sections` array (around line 37), add the AI entry between editor and shortcuts:

```ts
let sections = [
    { name: "theme",     icon: BrushIcon },
    { name: "shortcuts", icon: KeyboardIcon },
    { name: "editor",    icon: TypographyIcon },
    { name: "ai",        icon: AiIcon },         // ← add this
    { name: "dangerous", icon: AlertTriangleIcon },
    { name: "updates",   icon: RefreshIcon },
];
```

- [ ] **Step 3: Render AI component in main content area**

In the `{#if selectedSection === ...}` chain in the template (around line 88), add before the `{/if}`:

```svelte
{:else if selectedSection === "ai"}
    <AIComponent />
```

- [ ] **Step 4: Run type check**

```bash
cd /Users/__deesh_reddy__/projects/personal_git/rust_builds/tables
pnpm check 2>&1 | grep -E "error|Error" | head -20
```

Expected: No errors.

- [ ] **Step 5: Smoke-test in dev**

```bash
pnpm tauri dev
```

Open Settings (usually via top-right gear icon), click "AI" in the sidebar. Verify:
- Provider tabs visible (Claude, Google, OpenRouter, Codex, OpenCode, Cursor)
- Claude tab shows the info message
- OpenRouter tab shows API key + base URL fields + Fetch button
- Google tab shows API key field + Fetch button
- Harness log panel visible at the bottom

- [ ] **Step 6: Commit**

```bash
git add src/routes/settings/+page.svelte
git commit -m "feat(settings): add AI tab to settings sidebar"
```

---

## Task 9: Remove inline API key input from AgentComposer

**Files:**
- Modify: `src/lib/components/agent/AgentComposer.svelte`

- [ ] **Step 1: Remove the two derived values**

Find and delete these two lines (around line 58–62):

```ts
// DELETE these two:
const apiKeySettingsKey = $derived(providerConfig.apiKeySettingsKey);
const hasApiKey = $derived(
    !providerConfig.requiresApiKey ||
    (apiKeySettingsKey ? !!settingsStore[apiKeySettingsKey] : true)
);
```

- [ ] **Step 2: Remove the API key input block from the template**

Find and delete the entire block (around line 462–476):

```svelte
<!-- DELETE this entire block: -->
{#if providerConfig.requiresApiKey && !hasApiKey && apiKeySettingsKey}
    <div class="flex items-center gap-1">
        <input
            type="password"
            placeholder={providerConfig.apiKeyLabel ?? "API Key"}
            class="h-6 rounded border border-border bg-background px-2 text-[11px] font-mono text-foreground placeholder:text-muted-foreground/40 focus:outline-none focus:ring-1 focus:ring-accent w-40"
            oninput={(e) => {
                if (apiKeySettingsKey) {
                    settingsStore[apiKeySettingsKey] = (e.target as HTMLInputElement).value;
                }
            }}
        />
    </div>
{/if}
```

- [ ] **Step 3: Run type check**

```bash
cd /Users/__deesh_reddy__/projects/personal_git/rust_builds/tables
pnpm check 2>&1 | grep -E "error|Error" | head -20
```

Expected: No errors.

- [ ] **Step 4: Commit**

```bash
git add src/lib/components/agent/AgentComposer.svelte
git commit -m "feat(composer): remove inline API key input — configure providers via Settings → AI"
```

---

## Task 10: Remove restart-on-key effect from AgentPanel

**Files:**
- Modify: `src/lib/components/agent/AgentPanel.svelte`

- [ ] **Step 1: Remove the `_lastRestartedKey` guard and `$effect` block**

Find and delete these lines (around line 463–478):

```ts
// DELETE from here:
    // When the user enters an API key for Google/OpenRouter, restart the harness session
    // so the new key is picked up. Plain (non-reactive) variable guards against re-entry —
    // startThread writes reactive state which would otherwise re-trigger this effect.
    let _lastRestartedKey = "";
    $effect(() => {
        const key = currentProvider === "google"     ? settingsStore.googleApiKey
                  : currentProvider === "openrouter" ? settingsStore.openrouterApiKey
                  : null;
        if (!key || key === _lastRestartedKey || agentStore.messages.length > 0) return;
        const thread = threadsStore.activeThread;
        if (thread && thread.provider === currentProvider) {
            _lastRestartedKey = key; // plain write — not tracked by Svelte, breaks the loop
            startThread(thread).catch(console.error);
        }
    });
// DELETE to here
```

- [ ] **Step 2: Run type check**

```bash
cd /Users/__deesh_reddy__/projects/personal_git/rust_builds/tables
pnpm check 2>&1 | grep -E "error|Error" | head -20
```

Expected: No errors.

- [ ] **Step 3: Full smoke test**

```bash
pnpm tauri dev
```

Verify:
1. Settings → AI → OpenRouter: paste API key, click Fetch — model list loads, cards appear in 2 columns with chips
2. Toggle a model — it stays toggled after switching away and back
3. Settings → AI → Google: same flow works
4. Agent panel: no API key input visible in composer toolbar
5. Harness log panel in settings shows live entries when agent is used

- [ ] **Step 4: Commit**

```bash
git add src/lib/components/agent/AgentPanel.svelte
git commit -m "feat(agent): remove restart-on-key effect — API keys managed via Settings → AI"
```

---

## Self-Review

**Spec coverage check:**

| Spec requirement | Covered by |
|---|---|
| API key per provider | Tasks 1, 7 |
| Base URL override | Tasks 1, 7 (OpenRouter tab) |
| Fetch live model list from provider API | Task 7 (fetchGoogleModels, fetchOpenRouterModels) |
| Browse/search models | Task 5 (ModelGrid with search) |
| Pin models to main picker | Tasks 1, 5, 7 (toggle + persist) |
| Harness debug log panel | Tasks 2, 3, 4, 6 |
| Remove inline API key from composer | Task 9 |
| Remove restart-on-key effect | Task 10 |
| Provider tabs UI | Task 7 |
| New "AI" sidebar item | Task 8 |

**No placeholder scan:** All tasks contain complete code. No TBD/TODO items.

**Type consistency:** `ModelEntry` type defined in `ModelGrid.svelte` and imported in `AI.svelte`. `HarnessLogEntry` defined in `harnessLog.svelte.ts` and used in `HarnessLog.svelte`. `settingsStore.googlePinnedModels` / `openrouterPinnedModels` match the `string[]` type added in Task 1.
