# AI Provider Settings — Design Spec

**Date:** 2026-03-30
**Status:** Approved

---

## Overview

Add a dedicated **Settings → AI** section that lets users configure each AI provider: set API keys, override base URLs, fetch live model lists, and pin models to the main picker. Remove the inline API key input from the chat composer. Add a collapsible harness debug log panel inside the AI settings page.

---

## Architecture

### Where config lives

- New `"AI"` entry in the `/settings` sidebar, between Editor and Shortcuts
- Provider-level tabs across the top of the AI section: **Claude | Google | OpenRouter | Codex | OpenCode | Cursor**
- Per-provider config stored in the existing SQLite settings store via `commandClient.updateAppSetting()`

### Settings keys (new)

| Key | Type | Description |
|-----|------|-------------|
| `google_api_key` | string | Google AI API key |
| `google_base_url` | string | Override base URL (default: Google AI endpoint) |
| `google_pinned_models` | JSON string | Array of pinned model IDs |
| `openrouter_api_key` | string | OpenRouter API key |
| `openrouter_base_url` | string | Override base URL (default: `https://openrouter.ai/api/v1`) |
| `openrouter_pinned_models` | JSON string | Array of pinned model IDs |

Claude, Codex, OpenCode, Cursor tabs show their own relevant config fields (no API key for Claude since it uses the CLI).

### Model fetch

Models are fetched **directly from the frontend** (not through harness) using `fetch()` with the stored API key:

- **OpenRouter:** `GET https://openrouter.ai/api/v1/models` — `Authorization: Bearer <key>`
  Returns `{ data: [{ id, context_length, pricing: { prompt, completion } }] }`
- **Google:** `GET https://generativelanguage.googleapis.com/v1beta/models` — `?key=<key>`
  Returns `{ models: [{ name, displayName, inputTokenLimit }] }`

Fetched model list is stored in component state only (not persisted). Pinned model IDs are persisted.

### Harness debug logs

Harness currently logs to stderr. Add a structured log emitter:
- Harness emits logs as Tauri events: `harness://log` with payload `{ ts, level, tag, message }`
- Frontend stores last N (e.g. 200) log lines in a Svelte store
- Settings → AI page renders them in a collapsible panel at the bottom

---

## Components

### `src/routes/settings/AI.svelte`

Top-level AI settings page. Contains:
1. Provider tab strip
2. The active provider's config panel (rendered based on selected tab)
3. Collapsible harness log panel at the bottom

### `src/routes/settings/ai/ProviderConfig.svelte`

Generic per-provider config panel. Props:
- `provider: string`
- `apiKeyLabel?: string` — shown above the key input
- `apiKeySettingKey?: string` — settings store key to read/write
- `baseUrlSettingKey?: string` — settings store key for base URL override
- `defaultBaseUrl?: string` — placeholder text
- `fetchModels?: () => Promise<ModelEntry[]>` — if present, shows Fetch button + model grid

### `src/routes/settings/ai/ModelGrid.svelte`

2-column scrollable model grid. Props:
- `models: ModelEntry[]`
- `pinned: string[]` — pinned model IDs
- `onToggle: (id: string) => void`

Each card (compact row layout):
- Model ID (monospace, truncated with `text-overflow: ellipsis`)
- Context chip, price-in chip, price-out chip
- Toggle switch (right side)
- Pinned cards: accent border + accent color name

### `src/routes/settings/ai/HarnessLog.svelte`

Collapsible log panel. Reads from a shared `harnessLogStore`. Displays last 200 lines. Controls: Clear, collapse/expand.

### `src/lib/stores/harnessLog.svelte.ts`

```ts
// Listens to "harness://log" Tauri events
// Stores last 200 entries: { ts, level, tag, message }
```

---

## UI Details

### API key display

- Input is a password field (`type="password"`) that shows dots when not focused
- When the field has a value and is not focused: display truncated form `sk-or-v1-785…266` (first 12 chars + `…` + last 3 chars) via a read-only display div layered over the input, or use CSS `text-security: disc` approach
- Simple approach: separate "display" and "edit" states — display shows truncated, clicking switches to an actual `<input type="password">` that shows `••••••••••`

### Status bar

Shown above the model grid when models have been fetched:
- Green dot + "Connected — N models available · Last fetched X min ago"
- Orange dot + "API key required" when no key set
- Red dot + error message on fetch failure

### Search

Plain text filter on `model.id`. Updates result count. Debounced 150ms.

### Model cards (compact)

Single-row layout inside each card:
- Left: model name (ellipsis) + chip row below
- Right: toggle
- Padding: `6px 9px`
- Font size: model name `10px`, chips `9px`

### Provider tab status dots

- Green: provider configured and usable
- Orange: API key required
- Grey: not yet supported / disabled

---

## Removals

- Remove inline API key input from `AgentComposer.svelte`
- Remove `$effect` restart-on-key-paste logic from `AgentPanel.svelte` (replaced by: session only starts once a key is set in settings)

---

## Harness changes

Add structured log emission in `packages/harness/src/index.ts`:
```ts
// Replace console.error/console.log with:
function emitLog(level: "info"|"warn"|"error", tag: string, message: string) {
  process.stderr.write(JSON.stringify({ ts: Date.now(), level, tag, message }) + "\n");
}
```

Tauri Rust side (`harness.rs`) reads harness stderr line-by-line, parses JSON log lines, and emits them as `harness://log` events to the frontend.

---

## File Map

| File | Action |
|------|--------|
| `src/routes/settings/+page.svelte` | Add "AI" sidebar item; route to `AI.svelte` |
| `src/routes/settings/AI.svelte` | NEW — provider tabs + harness log |
| `src/routes/settings/ai/ProviderConfig.svelte` | NEW |
| `src/routes/settings/ai/ModelGrid.svelte` | NEW |
| `src/routes/settings/ai/HarnessLog.svelte` | NEW |
| `src/lib/stores/harnessLog.svelte.ts` | NEW |
| `src/lib/stores/settings.svelte.ts` | Add base URL + pinned model keys |
| `src/lib/components/agent/AgentComposer.svelte` | Remove inline API key input |
| `src/lib/components/agent/AgentPanel.svelte` | Remove restart-on-key effect |
| `packages/harness/src/index.ts` | Structured log output to stderr |
| `src-tauri/src/harness.rs` | Read stderr logs, emit as Tauri events |
