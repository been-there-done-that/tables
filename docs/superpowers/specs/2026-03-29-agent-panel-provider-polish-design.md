# Agent Panel — Provider Support & UI Polish Design

## Goal

Three improvements to the agent panel: fix arrow-key scroll in the `@` chip dropdown, polish the composer bottom bar (bigger icons, rename "Plan" → "Agent"), and surface the harness's existing multi-provider support in the UI.

## Architecture

### 1. Arrow Key Scroll Fix

**File:** `src/lib/components/agent/ComposerDropdown.svelte`

The dropdown tracks `activeIndex` but never scrolls the list container to follow. Fix: add a `$effect` that watches `activeIndex` and calls `el.scrollIntoView({ block: "nearest" })` on the active item element. No structural change — one effect, one line.

### 2. AgentComposer UI Polish

**File:** `src/lib/components/agent/AgentComposer.svelte`

- All icon sizes in the bottom bar: `11` → `13` (plan toggle, shield, model picker, effort picker)
- Plan toggle label: `"Plan"` → `"Agent"`
- Model picker: conditionally hidden when `PROVIDER_CONFIGS[provider].supportsModel === false` (OpenCode, Cursor)
- Effort picker: conditionally hidden when `PROVIDER_CONFIGS[provider].supportsEffort === false` (all non-Claude providers)
- All colors use existing theme tokens (`text-accent`, `text-foreground/70`, `text-muted-foreground`, `bg-accent/10`, etc.) — no hardcoded hex values

### 3. Multi-Provider Support

#### Harness — provider detection endpoint

**File:** `packages/harness/src/index.ts`

New route: `GET /providers` — iterates `PROVIDERS` registry, calls each provider's `isAvailable()` method, returns:
```json
[
  { "id": "claude",    "label": "Claude",    "available": true  },
  { "id": "gemini",    "label": "Gemini",    "available": true  },
  { "id": "codex",     "label": "Codex",     "available": false },
  { "id": "opencode",  "label": "OpenCode",  "available": false },
  { "id": "cursor",    "label": "Cursor",    "available": false }
]
```

**File:** `packages/harness/src/registry.ts`

Each provider class gets `isAvailable(): Promise<boolean>`:
- `ClaudeProvider`: checks `~/.claude/local/claude` or `claude` in PATH via `which claude`
- `GeminiProvider`: `which gemini`
- `CodexProvider`: attempts TCP connect to codex app-server port (default 8080), resolves false on timeout
- `OpenCodeProvider`: HTTP HEAD to OpenCode server URL, resolves false on error
- `CursorProvider`: checks if Cursor ACP socket/port is reachable

#### Frontend — provider config

**File:** `src/lib/agent/providers.ts` *(new)*

```ts
export interface ProviderConfig {
  label: string;
  models: { id: string; label: string }[];
  supportsModel: boolean;
  supportsEffort: boolean;
}

export const PROVIDER_CONFIGS: Record<string, ProviderConfig> = {
  claude: {
    label: "Claude",
    models: [
      { id: "claude-haiku-4-5-20251001", label: "Haiku 4.5" },
      { id: "claude-sonnet-4-6",         label: "Sonnet 4.6" },
      { id: "claude-opus-4-6",           label: "Opus 4.6"   },
    ],
    supportsModel: true,
    supportsEffort: true,
  },
  gemini: {
    label: "Gemini",
    models: [
      { id: "gemini-2.5-pro",   label: "2.5 Pro"   },
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
```

#### Frontend — settings

**File:** `src/lib/stores/settings.svelte.ts`

Add `aiProvider: string` (default `"claude"`) to `Settings` interface and `DEFAULT_SETTINGS`. Add getter/setter that persists via `commandClient.updateAppSetting("ai_provider", v)`. Handle `"ai_provider"` in the `settings-changed` listener.

#### Frontend — threads store

**File:** `src/lib/stores/threads.svelte.ts`

Add `provider: string` field to `AgentThread`. When loading threads from DB, read `provider` (default `"claude"` if missing). When creating a new thread, accept and store `provider`. Persist `provider` to the `agent_threads` DB table.

#### Backend — DB migration

**File:** `src-tauri/src/commands/agent_commands.rs` (or migrations file)

Add `provider TEXT NOT NULL DEFAULT 'claude'` column to `agent_threads` table via a new migration.

#### Frontend — ProviderPicker component

**File:** `src/lib/components/agent/ProviderPicker.svelte` *(new)*

Shown in the AgentPanel empty state (no messages in thread). Renders a 2-column tile grid of all 5 providers:
- **Installed** tiles: normal opacity, green ring (`box-shadow: 0 0 0 1.5px var(--color-success, #22c55e)`)
- **Not installed** tiles: `opacity-35`, `cursor-not-allowed`, clicking does nothing
- **Selected** tile: ring turns `var(--color-accent)` with amber glow
- Default selected = `settingsStore.aiProvider`

On tile click (installed only): update `settingsStore.aiProvider`, emit `onProviderChange(id)` to parent. The parent (AgentPanel) resets `settingsStore.aiModel` to the first model in the new provider's model list, or clears it if `supportsModel === false`.

Props: `providers: { id, label, available }[]`, `selected: string`, `onProviderChange: (id: string) => void`

#### Frontend — AgentPanel wiring

**File:** `src/lib/components/agent/AgentPanel.svelte`

- On mount / harness ready: `GET /providers` from harness, store result in `let availableProviders`
- Pass `availableProviders` and current provider to `ProviderPicker` when showing empty state
- Show locked badge in the panel header (next to ThreadPicker) when thread has messages: `<span>{provider} 🔒</span>`; no lock and clickable when thread is empty
- When provider changes on empty session: call `createAndStartThread()` with new provider (discards current empty session)
- Pass `provider` to `session/start` in harness HTTP call

**Session lifecycle rules:**
- New thread → provider = `settingsStore.aiProvider`
- 0 messages sent → provider badge is clickable; switching discards session and restarts with new provider; saves to `settingsStore.aiProvider`
- ≥1 message sent → badge shows `🔒`, provider locked for this thread's lifetime
- Thread restored from DB → reads `thread.provider`, badge reflects it

## File Map

| File | Change |
|---|---|
| `src/lib/components/agent/ComposerDropdown.svelte` | Add `scrollIntoView` effect on activeIndex |
| `src/lib/components/agent/AgentComposer.svelte` | Icons 13px, "Agent" label, conditional model/effort pickers |
| `src/lib/agent/providers.ts` | New: PROVIDER_CONFIGS map |
| `src/lib/stores/settings.svelte.ts` | Add `aiProvider` field |
| `src/lib/stores/threads.svelte.ts` | Add `provider` field on AgentThread |
| `src/lib/components/agent/ProviderPicker.svelte` | New: tile grid with ring indicator |
| `src/lib/components/agent/AgentPanel.svelte` | Fetch /providers, show ProviderPicker, locked badge, pass provider to session |
| `packages/harness/src/registry.ts` | Add `isAvailable()` to each provider class |
| `packages/harness/src/index.ts` | Add `GET /providers` endpoint |
| `src-tauri/src/commands/agent_commands.rs` | Add `provider` column migration to agent_threads |
