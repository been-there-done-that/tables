# Agent Panel — Provider Support & UI Polish Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix arrow-key scroll in the `@` chip dropdown, polish the composer bottom bar (bigger icons, "Agent" label), and wire up multi-provider support with a provider picker on the empty state and a locked badge on active sessions.

**Architecture:** Frontend-only for Tasks 1–4 and 8–9. Harness changes (Tasks 7) add `isAvailable()` to each provider class and a `GET /providers` endpoint. DB migration (Task 5) and threads store update (Task 6) add a `provider` column to persist per-thread provider selection. `PROVIDER_CONFIGS` (Task 2) is the single source of truth for per-provider capabilities.

**Tech Stack:** SvelteKit + Svelte 5 runes, TypeScript, Bun (harness), Rust/SQLite (migration), `@tabler/icons-svelte`, Tailwind CSS theme tokens

---

## File Map

| File | Change |
|---|---|
| `src/lib/components/agent/ComposerDropdown.svelte` | Add `scrollIntoView` `$effect` on `activeIndex` |
| `src/lib/agent/providers.ts` | **New** — `PROVIDER_CONFIGS` map |
| `src/lib/stores/settings.svelte.ts` | Add `aiProvider: string` field |
| `src/lib/components/agent/AgentComposer.svelte` | Icons 13px, "Agent" label, conditional model/effort pickers |
| `src-tauri/migrations/008_agent_thread_provider.sql` | **New** — `ALTER TABLE agent_threads ADD COLUMN provider` |
| `src-tauri/src/commands/agent_commands.rs` | Add `provider` to `AgentThread` struct, `create_agent_thread`, `list_agent_threads` |
| `src/lib/stores/threads.svelte.ts` | Add `provider: string` to `AgentThread`, pass through `createThread` |
| `packages/harness/src/providers/claude.ts` | Add `isAvailable()` |
| `packages/harness/src/providers/gemini.ts` | Add `isAvailable()` |
| `packages/harness/src/providers/codex.ts` | Add `isAvailable()` |
| `packages/harness/src/providers/opencode.ts` | Add `isAvailable()` |
| `packages/harness/src/providers/cursor.ts` | Add `isAvailable()` |
| `packages/harness/src/registry.ts` | Export `PROVIDER_LABELS`, add `checkAvailability()` |
| `packages/harness/src/index.ts` | Add `GET /providers` route |
| `src/lib/components/agent/ProviderPicker.svelte` | **New** — tile grid with green ring / dimmed tiles |
| `src/lib/components/agent/AgentPanel.svelte` | Fetch `/providers`, show `ProviderPicker` on empty state, locked badge in header, pass `provider` to session |

---

## Task 1: Arrow Key Scroll Fix in ComposerDropdown

**Files:**
- Modify: `src/lib/components/agent/ComposerDropdown.svelte`

The dropdown has `max-h-[220px] overflow-y-auto` but `activeIndex` changes never scroll the container. Fix: bind the scroll container, track item elements, and scroll on index change.

- [ ] **Step 1: Read the current file**

```bash
cat src/lib/components/agent/ComposerDropdown.svelte
```

- [ ] **Step 2: Add scroll container bind and item ref tracking**

In the `<script>` block, add after the existing `$effect` that resets `activeIndex`:

```ts
let listEl = $state<HTMLElement | null>(null);

$effect(() => {
    // Scroll active item into view when activeIndex changes
    const el = listEl?.children[activeIndex] as HTMLElement | undefined;
    el?.scrollIntoView({ block: "nearest" });
});
```

In the template, add `bind:this={listEl}` to the outer `<div>`:

```svelte
<div
    bind:this={listEl}
    class="z-50 min-w-[220px] max-w-[300px] rounded-md border border-border bg-popover shadow-xl overflow-hidden max-h-[220px] overflow-y-auto"
    role="listbox"
>
```

- [ ] **Step 3: Verify TypeScript**

```bash
cd /Users/__deesh_reddy__/projects/personal_git/rust_builds/tables
pnpm check
```

Expected: 0 errors, 0 warnings.

- [ ] **Step 4: Commit**

```bash
git add src/lib/components/agent/ComposerDropdown.svelte
git commit -m "fix(agent): scroll active item into view on arrow key navigation in @ dropdown"
```

---

## Task 2: Provider Configs — PROVIDER_CONFIGS

**Files:**
- Create: `src/lib/agent/providers.ts`

Single source of truth for per-provider capabilities. Used by AgentComposer (conditional pickers) and ProviderPicker (tile grid).

- [ ] **Step 1: Create `src/lib/agent/providers.ts`**

```ts
export interface ProviderModel {
    id: string;
    label: string;
}

export interface ProviderConfig {
    label: string;
    models: ProviderModel[];
    supportsModel: boolean;
    supportsEffort: boolean;
}

export const PROVIDER_CONFIGS: Record<string, ProviderConfig> = {
    claude: {
        label: "Claude",
        models: [
            { id: "claude-haiku-4-5-20251001", label: "Haiku 4.5" },
            { id: "claude-sonnet-4-6",         label: "Sonnet 4.6" },
            { id: "claude-opus-4-6",           label: "Opus 4.6" },
        ],
        supportsModel: true,
        supportsEffort: true,
    },
    gemini: {
        label: "Gemini",
        models: [
            { id: "gemini-2.5-pro",   label: "2.5 Pro" },
            { id: "gemini-2.0-flash", label: "2.0 Flash" },
        ],
        supportsModel: true,
        supportsEffort: false,
    },
    codex: {
        label: "Codex",
        models: [],
        supportsModel: false,
        supportsEffort: false,
    },
    opencode: {
        label: "OpenCode",
        models: [],
        supportsModel: false,
        supportsEffort: false,
    },
    cursor: {
        label: "Cursor",
        models: [],
        supportsModel: false,
        supportsEffort: false,
    },
};

/** Returns the default model ID for a provider, or empty string if none. */
export function defaultModel(provider: string): string {
    return PROVIDER_CONFIGS[provider]?.models[0]?.id ?? "";
}
```

- [ ] **Step 2: Verify TypeScript**

```bash
cd /Users/__deesh_reddy__/projects/personal_git/rust_builds/tables
pnpm check
```

Expected: 0 errors.

- [ ] **Step 3: Commit**

```bash
git add src/lib/agent/providers.ts
git commit -m "feat(agent): add PROVIDER_CONFIGS with per-provider model and capability definitions"
```

---

## Task 3: Settings — Add aiProvider

**Files:**
- Modify: `src/lib/stores/settings.svelte.ts`

Add `aiProvider: string` (default `"claude"`) following the exact same getter/setter pattern as `aiModel`.

- [ ] **Step 1: Add `aiProvider` to the `Settings` interface**

Find the `aiModel` line in the interface and add after it:

```ts
aiModel: string;
aiEffort: "auto" | "low" | "medium" | "high" | "max";
queryApproval: "auto" | "ask";
aiProvider: string;    // ← add this
lastActiveThreadId: string | null;
```

- [ ] **Step 2: Add default to `DEFAULT_SETTINGS`**

Find `aiModel: "claude-sonnet-4-6"` and add after it:

```ts
aiModel: "claude-sonnet-4-6",
aiEffort: "auto",
queryApproval: "ask",
aiProvider: "claude",    // ← add this
lastActiveThreadId: null,
```

- [ ] **Step 3: Add getter/setter in `createSettingsStore()` return object**

Add after the `aiEffort` getter/setter:

```ts
get aiProvider(): string {
    return settings.aiProvider;
},
set aiProvider(v: string) {
    settings.aiProvider = v;
    commandClient.updateAppSetting("ai_provider", v);
},
```

- [ ] **Step 4: Handle `"ai_provider"` in the `settings-changed` listener**

In the `switch (key)` block inside `listen("settings-changed", ...)`, add:

```ts
case "ai_provider":
    settings.aiProvider = value || "claude";
    return;
```

- [ ] **Step 5: Verify TypeScript**

```bash
cd /Users/__deesh_reddy__/projects/personal_git/rust_builds/tables
pnpm check
```

Expected: 0 errors.

- [ ] **Step 6: Commit**

```bash
git add src/lib/stores/settings.svelte.ts
git commit -m "feat(agent): add aiProvider setting with persistence"
```

---

## Task 4: AgentComposer UI Polish

**Files:**
- Modify: `src/lib/components/agent/AgentComposer.svelte`

Bump all bottom-bar icons from 11 → 13px, rename "Plan" → "Agent", and conditionally show model/effort pickers based on the active provider's capabilities.

- [ ] **Step 1: Read the current file**

```bash
cat src/lib/components/agent/AgentComposer.svelte
```

- [ ] **Step 2: Add import and new `provider` prop**

Add to the imports at the top of `<script>`:

```ts
import { PROVIDER_CONFIGS, defaultModel } from "$lib/agent/providers";
```

Update the `Props` interface and destructuring to add `provider`:

```ts
interface Props {
    onSend: (displayText: string, fullText: string, rawDoc: unknown) => void;
    onStop: () => void;
    onPlanModeToggle: () => void;
    onQueryApprovalToggle: () => void;
    running: boolean;
    disabled: boolean;
    planMode: boolean;
    queryApproval: "auto" | "ask";
    provider: string;
}

let { onSend, onStop, onPlanModeToggle, onQueryApprovalToggle, running, disabled, planMode, queryApproval, provider }: Props = $props();
```

- [ ] **Step 3: Replace hardcoded MODELS with derived from PROVIDER_CONFIGS**

Remove the `const MODELS = [...]` block and replace it with a derived:

```ts
const providerConfig = $derived(PROVIDER_CONFIGS[provider] ?? PROVIDER_CONFIGS.claude);
const currentModelLabel = $derived(
    providerConfig.models.find((m) => m.id === settingsStore.aiModel)?.label
    ?? providerConfig.models[0]?.label
    ?? "Model"
);
const currentEffortLabel = $derived(
    EFFORTS.find((e) => e.id === settingsStore.aiEffort)?.label ?? "Auto"
);
```

Keep `const EFFORTS = [...]` as-is.

- [ ] **Step 4: Update the bottom bar template**

Replace the entire bottom bar `<div class="flex items-center justify-between px-2 pb-2 pt-1">` block with:

```svelte
<!-- Bottom bar -->
<div class="flex items-center justify-between px-2 pb-2 pt-1">
    <div class="flex items-center gap-0.5">
        <!-- Agent mode toggle -->
        <button
            onclick={onPlanModeToggle}
            title={planMode ? "Agent mode on — run_query requires your approval before executing" : "Agent mode off — all tools auto-execute"}
            class="flex items-center gap-1 rounded px-1.5 py-1 text-[10.5px] transition-colors {planMode ? 'text-amber-400 hover:bg-amber-400/10' : 'text-muted-foreground/50 hover:bg-foreground/5 hover:text-muted-foreground'}"
        >
            <IconMap size={13} />
            {#if planMode}<span class="font-mono">Agent</span>{/if}
        </button>

        <!-- Query approval toggle -->
        <button
            onclick={onQueryApprovalToggle}
            title={queryApproval === "ask" ? "Queries need your approval before running" : "Queries run automatically"}
            class="flex items-center gap-1 rounded px-1.5 py-1 text-[10.5px] transition-colors {queryApproval === 'ask' ? 'text-amber-400 hover:bg-amber-400/10' : 'text-muted-foreground/50 hover:bg-foreground/5 hover:text-muted-foreground'}"
        >
            {#if queryApproval === "ask"}
                <IconShieldCheck size={13} />
            {:else}
                <IconShield size={13} />
            {/if}
        </button>

        <!-- Model picker (hidden for providers without model selection) -->
        {#if providerConfig.supportsModel}
            <Menu.Root>
                <Menu.Trigger>
                    <button
                        class="flex items-center gap-1 rounded px-1.5 py-1 text-[10.5px] text-muted-foreground/50 transition-colors hover:bg-foreground/5 hover:text-muted-foreground"
                        title="Switch model"
                    >
                        <IconCpu size={13} />
                        <span class="font-mono">{currentModelLabel}</span>
                        <IconChevronDown size={9} class="opacity-50" />
                    </button>
                </Menu.Trigger>
                <Menu.Content
                    class="w-40 border border-border bg-background shadow-md p-1"
                    align="start"
                    side="top"
                >
                    {#each providerConfig.models as m}
                        <Menu.Item
                            class="flex items-center justify-between gap-2 px-2 py-1.5 text-[11px] font-mono rounded cursor-pointer"
                            onclick={() => { settingsStore.aiModel = m.id; }}
                        >
                            {m.label}
                            {#if settingsStore.aiModel === m.id}
                                <IconCheck size={11} class="shrink-0 text-accent" />
                            {/if}
                        </Menu.Item>
                    {/each}
                </Menu.Content>
            </Menu.Root>
        {/if}

        <!-- Effort picker (Claude only) -->
        {#if providerConfig.supportsEffort}
            <Menu.Root>
                <Menu.Trigger>
                    <button
                        class="flex items-center gap-1 rounded px-1.5 py-1 text-[10.5px] transition-colors hover:bg-foreground/5 {settingsStore.aiEffort !== 'auto' && settingsStore.aiEffort !== 'low' ? 'text-accent/70 hover:text-accent' : 'text-muted-foreground/50 hover:text-muted-foreground'}"
                        title="Thinking effort"
                    >
                        <IconBrain size={13} />
                        <span class="font-mono">{currentEffortLabel}</span>
                        <IconChevronDown size={9} class="opacity-50" />
                    </button>
                </Menu.Trigger>
                <Menu.Content
                    class="w-36 border border-border bg-background shadow-md p-1"
                    align="start"
                    side="top"
                >
                    {#each EFFORTS as ef}
                        <Menu.Item
                            class="flex items-center justify-between gap-2 px-2 py-1.5 text-[11px] font-mono rounded cursor-pointer"
                            onclick={() => { settingsStore.aiEffort = ef.id; }}
                        >
                            {ef.label}
                            {#if settingsStore.aiEffort === ef.id}
                                <IconCheck size={11} class="shrink-0 text-accent" />
                            {/if}
                        </Menu.Item>
                    {/each}
                </Menu.Content>
            </Menu.Root>
        {/if}
    </div>

    <div class="flex items-center gap-1.5">
        {#if running}
            <button
                onclick={onStop}
                title="Stop"
                class="flex h-6 w-6 items-center justify-center rounded-full bg-red-500 text-white transition-colors hover:bg-red-600"
            >
                <IconSquare size={10} fill="currentColor" />
            </button>
        {:else}
            <button
                onclick={() => void handleSend()}
                disabled={disabled}
                title="Send (↵)"
                class="flex h-6 w-6 items-center justify-center rounded-full bg-foreground text-background transition-opacity hover:opacity-80 disabled:opacity-25"
            >
                <IconArrowUp size={13} stroke-width={2.5} />
            </button>
        {/if}
    </div>
</div>
```

- [ ] **Step 5: Verify TypeScript**

```bash
cd /Users/__deesh_reddy__/projects/personal_git/rust_builds/tables
pnpm check
```

Expected: 0 errors. If AgentPanel.svelte shows an error about missing `provider` prop, that will be fixed in Task 9.

- [ ] **Step 6: Commit**

```bash
git add src/lib/components/agent/AgentComposer.svelte
git commit -m "feat(agent): bigger icons, rename Plan→Agent, conditional model/effort pickers by provider"
```

---

## Task 5: DB Migration — Add provider to agent_threads

**Files:**
- Create: `src-tauri/migrations/008_agent_thread_provider.sql`
- Modify: `src-tauri/src/commands/agent_commands.rs`

- [ ] **Step 1: Read migrations.rs to understand how migrations are registered**

```bash
cat src-tauri/src/migrations.rs
```

Look for how `007_create_agent_tables.sql` is referenced — follow the same pattern.

- [ ] **Step 2: Create `src-tauri/migrations/008_agent_thread_provider.sql`**

```sql
-- Migration 008: Add provider column to agent_threads
-- Existing rows default to 'claude' (backward compatible)
ALTER TABLE agent_threads ADD COLUMN provider TEXT NOT NULL DEFAULT 'claude';
```

- [ ] **Step 3: Register the migration in migrations.rs**

Follow the same pattern used for `007_create_agent_tables.sql`. Add `008_agent_thread_provider.sql` as the next migration in the list.

- [ ] **Step 4: Add `provider` to `AgentThread` struct in agent_commands.rs**

Find the `AgentThread` struct and add:

```rust
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AgentThread {
    pub id: String,
    pub title: String,
    pub connection_id: String,
    pub database_name: Option<String>,
    pub model: String,
    pub effort: String,
    pub provider: String,          // ← add this
    pub sdk_session_id: Option<String>,
    pub summary: Option<String>,
    pub parent_thread_id: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}
```

- [ ] **Step 5: Update `create_agent_thread` command to accept and store provider**

Find the `create_agent_thread` function and update its signature and SQL:

```rust
#[tauri::command]
pub fn create_agent_thread(
    state: State<'_, DatabaseState>,
    id: String,
    connection_id: String,
    database_name: Option<String>,
    model: String,
    effort: String,
    provider: String,              // ← add this parameter
    parent_thread_id: Option<String>,
    now: i64,
) -> Result<(), String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO agent_threads (id, title, connection_id, database_name, model, effort, provider, parent_thread_id, created_at, updated_at)
         VALUES (?1, 'New chat', ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)",
        params![id, connection_id, database_name, model, effort, provider, parent_thread_id, now],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}
```

- [ ] **Step 6: Update `list_agent_threads` to read `provider` column**

Find the `list_agent_threads` SELECT and row mapping. Add `provider` to the SELECT:

```sql
SELECT id, title, connection_id, database_name, model, effort, provider,
       sdk_session_id, summary, parent_thread_id, created_at, updated_at
FROM agent_threads
WHERE connection_id = ?1
  AND (database_name IS ?2 OR ?2 IS NULL)
ORDER BY updated_at DESC
```

In the row mapping closure, add `provider: row.get(6)?` and shift subsequent indices by 1. The full mapping should be:

```rust
Ok(AgentThread {
    id: row.get(0)?,
    title: row.get(1)?,
    connection_id: row.get(2)?,
    database_name: row.get(3)?,
    model: row.get(4)?,
    effort: row.get(5)?,
    provider: row.get(6)?,
    sdk_session_id: row.get(7)?,
    summary: row.get(8)?,
    parent_thread_id: row.get(9)?,
    created_at: row.get(10)?,
    updated_at: row.get(11)?,
})
```

- [ ] **Step 7: Run Rust tests**

```bash
cd /Users/__deesh_reddy__/projects/personal_git/rust_builds/tables/src-tauri
cargo test 2>&1 | tail -10
```

Expected: All tests pass.

- [ ] **Step 8: Commit**

```bash
git add src-tauri/migrations/008_agent_thread_provider.sql src-tauri/src/commands/agent_commands.rs src-tauri/src/migrations.rs
git commit -m "feat(agent): add provider column to agent_threads with migration"
```

---

## Task 6: Threads Store — Add provider Field

**Files:**
- Modify: `src/lib/stores/threads.svelte.ts`

- [ ] **Step 1: Add `provider` to `AgentThread` interface**

```ts
export interface AgentThread {
    id: string;
    title: string;
    connectionId: string;
    databaseName: string | null;
    model: string;
    effort: string;
    provider: string;          // ← add this
    sdkSessionId: string | null;
    summary: string | null;
    parentThreadId: string | null;
    createdAt: number;
    updatedAt: number;
}
```

- [ ] **Step 2: Add `provider` to `createThread` opts and invocation**

Update the `createThread` method:

```ts
async createThread(opts: {
    connectionId: string;
    databaseName: string | null;
    model: string;
    effort: "auto" | "low" | "medium" | "high" | "max";
    provider: string;              // ← add this
    parentThreadId?: string | null;
}): Promise<AgentThread> {
    const id = crypto.randomUUID();
    const now = nowSecs();
    try {
        await invoke("create_agent_thread", {
            id,
            connectionId: opts.connectionId,
            databaseName: opts.databaseName,
            model: opts.model,
            effort: opts.effort,
            provider: opts.provider,   // ← add this
            parentThreadId: opts.parentThreadId ?? null,
            now,
        });
    } catch (e) {
        console.error("[threads] createThread failed:", e);
        throw e;
    }
    const thread: AgentThread = {
        id,
        title: "New chat",
        connectionId: opts.connectionId,
        databaseName: opts.databaseName,
        model: opts.model,
        effort: opts.effort,
        provider: opts.provider,   // ← add this
        sdkSessionId: null,
        summary: null,
        parentThreadId: opts.parentThreadId ?? null,
        createdAt: now,
        updatedAt: now,
    };
    this.threads = [thread, ...this.threads];
    return thread;
}
```

- [ ] **Step 3: Verify TypeScript**

```bash
cd /Users/__deesh_reddy__/projects/personal_git/rust_builds/tables
pnpm check
```

Expected: TypeScript errors about `createThread` call sites missing `provider` — these will be fixed in Task 9 (AgentPanel). If AgentPanel is the only caller, this is expected. Other files should be 0 errors.

- [ ] **Step 4: Commit**

```bash
git add src/lib/stores/threads.svelte.ts
git commit -m "feat(agent): add provider field to AgentThread store and createThread"
```

---

## Task 7: Harness — isAvailable() and GET /providers

**Files:**
- Modify: `packages/harness/src/providers/claude.ts`
- Modify: `packages/harness/src/providers/gemini.ts`
- Modify: `packages/harness/src/providers/codex.ts`
- Modify: `packages/harness/src/providers/opencode.ts`
- Modify: `packages/harness/src/providers/cursor.ts`
- Modify: `packages/harness/src/registry.ts`
- Modify: `packages/harness/src/index.ts`

- [ ] **Step 1: Read each provider file**

```bash
cat packages/harness/src/providers/claude.ts
cat packages/harness/src/providers/gemini.ts
cat packages/harness/src/providers/codex.ts
cat packages/harness/src/providers/opencode.ts
cat packages/harness/src/providers/cursor.ts
```

Understand what binary/server each provider connects to — this informs the `isAvailable()` check.

- [ ] **Step 2: Add `isAvailable()` to ClaudeProvider**

In `packages/harness/src/providers/claude.ts`, add the method to the class:

```ts
async isAvailable(): Promise<boolean> {
    // Check ~/.claude/local/claude first (preferred install location)
    const homePath = `${process.env.HOME ?? "~"}/.claude/local/claude`;
    try {
        await Bun.file(homePath).exists();
        const f = Bun.file(homePath);
        if (await f.exists()) return true;
    } catch { /* ignore */ }
    // Fall back to which claude
    try {
        const result = await Bun.$`which claude`.quiet();
        return result.exitCode === 0;
    } catch {
        return false;
    }
}
```

- [ ] **Step 3: Add `isAvailable()` to GeminiProvider**

```ts
async isAvailable(): Promise<boolean> {
    try {
        const result = await Bun.$`which gemini`.quiet();
        return result.exitCode === 0;
    } catch {
        return false;
    }
}
```

- [ ] **Step 4: Add `isAvailable()` to CodexProvider**

Read the file first to understand how it connects (JSON-RPC to `codex app-server`). Then check for the `codex` binary:

```ts
async isAvailable(): Promise<boolean> {
    try {
        const result = await Bun.$`which codex`.quiet();
        return result.exitCode === 0;
    } catch {
        return false;
    }
}
```

- [ ] **Step 5: Add `isAvailable()` to OpenCodeProvider**

Read the file to understand the server URL (likely `http://localhost:4321` or similar). Then:

```ts
async isAvailable(): Promise<boolean> {
    try {
        // Use the same base URL the provider already uses
        const res = await fetch(this.baseUrl, { method: "HEAD", signal: AbortSignal.timeout(500) });
        return res.ok || res.status < 500;
    } catch {
        return false;
    }
}
```

Use `this.baseUrl` — read the provider file to confirm the property name. Adjust if different.

- [ ] **Step 6: Add `isAvailable()` to CursorProvider**

Read the file to understand how it connects (ACP WebSocket). Then:

```ts
async isAvailable(): Promise<boolean> {
    try {
        const result = await Bun.$`which cursor`.quiet();
        return result.exitCode === 0;
    } catch {
        return false;
    }
}
```

- [ ] **Step 7: Update registry.ts to export availability check**

In `packages/harness/src/registry.ts`, add an interface that all providers must implement and export a helper:

```ts
export interface AvailableProvider {
    id: string;
    label: string;
    available: boolean;
}

const PROVIDER_LABELS: Record<string, string> = {
    claude:   "Claude",
    gemini:   "Gemini",
    codex:    "Codex",
    opencode: "OpenCode",
    cursor:   "Cursor",
};

export async function checkAvailability(): Promise<AvailableProvider[]> {
    const results: AvailableProvider[] = [];
    for (const [id, factory] of Object.entries(PROVIDERS)) {
        // Create a dummy session to call isAvailable()
        const instance = factory({ sessionId: "", threadId: "", systemPrompt: "", provider: id });
        const available = typeof (instance as any).isAvailable === "function"
            ? await (instance as any).isAvailable().catch(() => false)
            : false;
        results.push({ id, label: PROVIDER_LABELS[id] ?? id, available });
    }
    return results;
}
```

- [ ] **Step 8: Add `GET /providers` route to index.ts**

In `packages/harness/src/index.ts`, import and add the route before the catch-all `return new Response("harness ok")`:

Add import at top:
```ts
import { checkAvailability } from "./registry";
```

Add route inside the `fetch` handler, after the existing routes:

```ts
if (req.method === "GET" && url.pathname === "/providers") {
    const providers = await checkAvailability();
    return Response.json(providers, { headers: CORS });
}
```

Also update the CORS headers to allow `GET`:
```ts
const CORS = {
    "Access-Control-Allow-Origin": "*",
    "Access-Control-Allow-Methods": "GET, POST, OPTIONS",
    "Access-Control-Allow-Headers": "Content-Type",
};
```

- [ ] **Step 9: Build the harness to verify no TypeScript errors**

```bash
cd /Users/__deesh_reddy__/projects/personal_git/rust_builds/tables/packages/harness
bun run build 2>&1 | tail -20
```

Expected: build succeeds, binary written to `../../src-tauri/binaries/harness-aarch64-apple-darwin` (or relevant platform triple).

- [ ] **Step 10: Commit**

```bash
cd /Users/__deesh_reddy__/projects/personal_git/rust_builds/tables
git add packages/harness/src/providers/ packages/harness/src/registry.ts packages/harness/src/index.ts
git commit -m "feat(harness): add isAvailable() to all providers and GET /providers endpoint"
```

---

## Task 8: ProviderPicker Component

**Files:**
- Create: `src/lib/components/agent/ProviderPicker.svelte`

Tile grid shown in AgentPanel empty state. Installed tiles have a green ring; not-installed tiles are dimmed. Selected tile has an accent ring.

- [ ] **Step 1: Create `src/lib/components/agent/ProviderPicker.svelte`**

```svelte
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
```

- [ ] **Step 2: Verify TypeScript**

```bash
cd /Users/__deesh_reddy__/projects/personal_git/rust_builds/tables
pnpm check
```

Expected: 0 errors.

- [ ] **Step 3: Commit**

```bash
git add src/lib/components/agent/ProviderPicker.svelte
git commit -m "feat(agent): add ProviderPicker component with installed ring indicator"
```

---

## Task 9: AgentPanel Wiring

**Files:**
- Modify: `src/lib/components/agent/AgentPanel.svelte`

This is the integration task — fetch providers, show ProviderPicker on empty state, show locked badge in header, pass provider through session lifecycle.

- [ ] **Step 1: Read the current AgentPanel.svelte**

```bash
cat src/lib/components/agent/AgentPanel.svelte
```

Note: the file is large. Focus on:
- The imports section
- `getToolContext()` and how `port` is accessed
- `startThread()` and `createAndStartThread()` — how they call `threadsStore.createThread`
- The template: the header area (ThreadPicker), the empty state block, the `<AgentComposer>` usage
- How the session start/resume HTTP calls are made (look for `fetch(... /session/start`)

- [ ] **Step 2: Add imports and state for providers**

Add to imports:
```ts
import ProviderPicker from "./ProviderPicker.svelte";
import type { AvailableProvider } from "./ProviderPicker.svelte";
import { PROVIDER_CONFIGS, defaultModel } from "$lib/agent/providers";
```

Add after existing `$state` declarations:
```ts
let availableProviders = $state<AvailableProvider[]>([]);
```

- [ ] **Step 3: Fetch /providers when harness port is known**

Find the `$effect` or code that runs when the harness port becomes available (look for where `port` or `harnessPort` is first used). Add a fetch call there:

```ts
// After harness port is ready:
fetch(`http://127.0.0.1:${port}/providers`)
    .then((r) => r.json())
    .then((data) => { availableProviders = data as AvailableProvider[]; })
    .catch(() => { /* harness not running in dev — leave empty */ });
```

- [ ] **Step 4: Derive the current session's provider**

Add a derived:
```ts
const currentProvider = $derived(
    threadsStore.activeThread?.provider ?? settingsStore.aiProvider
);

const sessionHasMessages = $derived(agentStore.messages.length > 0);
```

- [ ] **Step 5: Update createAndStartThread to pass provider**

Find `createAndStartThread` and update the `threadsStore.createThread` call to pass provider:

```ts
async function createAndStartThread() {
    const conn = schemaStore.activeConnection;
    if (!conn) return;
    const thread = await threadsStore.createThread({
        connectionId: conn.id,
        databaseName: schemaStore.selectedDatabase,
        model: settingsStore.aiModel || "claude-sonnet-4-6",
        effort: settingsStore.aiEffort,
        provider: settingsStore.aiProvider,   // ← add this
    });
    await startThread(thread);
}
```

- [ ] **Step 6: Handle provider change from ProviderPicker**

Add a function:
```ts
function handleProviderChange(newProvider: string) {
    // Update settings immediately
    settingsStore.aiProvider = newProvider;
    // Reset model to first model for new provider (or clear if none)
    const firstModel = defaultModel(newProvider);
    settingsStore.aiModel = firstModel;
    // Discard current empty session and restart with new provider
    void createAndStartThread();
}
```

- [ ] **Step 7: Pass provider to session/start and session/resume HTTP calls**

Find the `fetch` call to `/session/start` and add `provider: currentProvider` to the JSON body:

```ts
await fetch(`http://127.0.0.1:${port}/session/start`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
        sessionId,
        threadId: thread.id,
        systemPrompt,
        model: settingsStore.aiModel,
        effort: settingsStore.aiEffort,
        provider: thread.provider,    // ← add this
    }),
});
```

Do the same for `/session/resume` — add `provider: thread.provider`.

- [ ] **Step 8: Update the template — provider badge in header**

Find the header area near `<ThreadPicker>` and add the badge after it:

```svelte
<!-- Provider badge — locked when session has messages -->
{#if sessionHasMessages}
    <span
        class="flex items-center gap-1 rounded-full border border-border/30 px-2 py-0.5 text-[9.5px] text-muted-foreground/60"
        title="Provider locked for this session"
    >
        {PROVIDER_CONFIGS[currentProvider]?.label ?? currentProvider}
        <span class="opacity-50">🔒</span>
    </span>
{/if}
```

- [ ] **Step 9: Update the template — ProviderPicker in empty state**

Find the empty state block (shown when there are no messages). Replace or augment it with `ProviderPicker`:

```svelte
{#if agentStore.messages.length === 0 && !agentStore.loading}
    <div class="flex flex-1 flex-col items-center justify-center">
        <ProviderPicker
            providers={availableProviders}
            selected={currentProvider}
            onProviderChange={handleProviderChange}
        />
    </div>
{/if}
```

- [ ] **Step 10: Pass provider prop to AgentComposer**

Find `<AgentComposer` in the template and add `provider={currentProvider}`:

```svelte
<AgentComposer
    onSend={send}
    onStop={stop}
    onPlanModeToggle={() => (planMode = !planMode)}
    onQueryApprovalToggle={() => {
        settingsStore.queryApproval = settingsStore.queryApproval === "ask" ? "auto" : "ask";
    }}
    running={agentStore.status === "running"}
    disabled={!sessionReady || !!sessionError}
    planMode={planMode}
    queryApproval={settingsStore.queryApproval}
    provider={currentProvider}
/>
```

- [ ] **Step 11: Verify TypeScript**

```bash
cd /Users/__deesh_reddy__/projects/personal_git/rust_builds/tables
pnpm check
```

Expected: 0 errors, 0 warnings.

- [ ] **Step 12: Run Rust tests**

```bash
cd src-tauri && cargo test 2>&1 | tail -5
```

Expected: All tests pass.

- [ ] **Step 13: Commit**

```bash
cd /Users/__deesh_reddy__/projects/personal_git/rust_builds/tables
git add src/lib/components/agent/AgentPanel.svelte
git commit -m "feat(agent): wire multi-provider support — ProviderPicker empty state, locked badge, provider in session lifecycle"
```
