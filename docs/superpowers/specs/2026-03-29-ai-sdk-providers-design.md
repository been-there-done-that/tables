# AI SDK Providers Design (OpenRouter + Google)

**Date:** 2026-03-29
**Status:** Approved for implementation

---

## 1. Goal

Add **OpenRouter** and **Google** as first-class agent providers in the Tables harness, using the **Vercel AI SDK** (`ai` + `@ai-sdk/*`) as the backbone. Replace the existing Gemini CLI subprocess provider with the proper `@ai-sdk/google` integration.

The critical improvement over the current CLI-based providers: tools are executed via **native AI SDK function calling** — no curl commands in the system prompt, no sandbox issues, no network restrictions. The model returns structured tool calls; the harness executes them via a shared ToolBridge that communicates with the frontend.

---

## 2. Scope

| In scope | Out of scope |
|---|---|
| OpenRouter provider (`@openrouter/ai-sdk-provider`) | OpenAI, Bedrock, Groq, Mistral, etc. (add later) |
| Google provider (`@ai-sdk/google`) | Changing the Claude/Cursor/Codex/OpenCode providers |
| Replace Gemini CLI with `@ai-sdk/google` | Session resume / conversation persistence |
| ToolBridge extracted to shared module | MCP tool support |
| AiSdkSession shared session class | Model discovery from models.dev |
| Frontend: API key inputs + model pickers for both | OS keychain for API key storage |

---

## 3. Architecture

```
packages/harness/src/
  index.ts                      ← HTTP server (interface unchanged)
  types.ts                      ← SessionConfig (no changes needed)
  registry.ts                   ← Add "openrouter" + "google" factories
  tool-bridge.ts                ← NEW: extracted from index.ts

  session/
    ai-sdk-session.ts           ← NEW: streamText() wrapper, tool loop

  providers/
    claude.ts                   ← UNCHANGED
    cursor.ts                   ← UNCHANGED
    codex.ts                    ← UNCHANGED
    opencode.ts                 ← UNCHANGED
    gemini.ts                   ← DELETED (replaced by google)
    openrouter.ts               ← NEW: thin wrapper over AiSdkSession
    google.ts                   ← NEW: thin wrapper over AiSdkSession

  tools/
    definitions.ts              ← NEW: all DB tools as AI SDK tool() schemas

src/lib/
  agent/
    providers.ts                ← Add openrouter + google configs + model lists
    claude.ts                   ← Add providerConfig to startAgentSession opts
  stores/
    settings.svelte.ts          ← Add openrouterApiKey, googleApiKey
  components/agent/
    AgentComposer.svelte        ← API key inputs for openrouter + google
    AgentPanel.svelte           ← Pass providerConfig to startAgentSession
```

---

## 4. ToolBridge

Currently `index.ts` owns `pendingToolResults` — a `Map` of request IDs waiting for the frontend to POST `/tool-result/:id`. This needs to be extracted so `AiSdkSession` can use it directly (without HTTP self-calls).

**`tool-bridge.ts` exports:**

```ts
// Called by AiSdkSession when AI SDK returns a tool call.
// Emits tool.started to SSE, waits for /tool-result/:id, emits tool.completed.
export async function callTool(
    sessionId: string,
    toolName: string,
    input: unknown,
    emitFn: (e: HarnessEvent) => void,
): Promise<unknown>

// Called by the /tool-result/:requestId HTTP handler to unblock a pending callTool().
export function resolveToolResult(requestId: string, result: unknown): boolean

// Called when SSE stream is cancelled (user stops turn).
export function cancelSessionTools(sessionId: string): void
```

`index.ts` migrates its `pendingToolResults` map and `/db/` handler to use these functions. The `/db/` endpoint (used by Claude/Codex/Cursor via curl) continues working exactly as before — it just calls `callTool()` internally.

---

## 5. AiSdkSession

One shared session class for all Vercel AI SDK providers. Receives a pre-configured `LanguageModel` instance from the provider wrapper.

**Tool loop via `maxSteps`:**

AI SDK's `streamText()` with `maxSteps: 20` handles the entire tool call loop automatically:
- Model returns tool calls → `execute()` functions are called → results fed back → model continues
- Each `execute()` calls `toolBridge.callTool()` which awaits the frontend

```ts
export class AiSdkSession extends HttpAdapter implements Session {
    private model: LanguageModel;
    private messages: CoreMessage[] = [];
    private sessionId: string;

    constructor(model: LanguageModel, config: SessionConfig) {
        super();
        this.model = model;
        this.sessionId = config.sessionId;
        this.messages = [{ role: "system", content: config.systemPrompt }];
    }

    async send(text: string): Promise<void> {
        this.messages.push({ role: "user", content: text });
        try {
            const tools = createDbTools(this.sessionId, (e) => this.emitFn(e));
            const result = streamText({
                model: this.model,
                messages: this.messages,
                tools,
                maxSteps: 20,
                onChunk: ({ chunk }) => {
                    if (chunk.type === "text-delta") {
                        this.emitFn({ type: "text.delta", content: chunk.textDelta });
                    } else if (chunk.type === "reasoning") {
                        this.emitFn({ type: "thinking.delta", content: chunk.textDelta });
                    }
                },
                onFinish: ({ response }) => {
                    // Append full assistant message to history for multi-turn
                    this.messages.push(...response.messages);
                },
            });
            await result.consumeStream();
            this.emitFn({ type: "turn.done" });
        } catch (e) {
            if (!this.isAborted()) {
                this.emitFn({ type: "error", message: String(e) });
            }
        }
    }

    stop() { this.abortController.abort(); }
    protected onStop() { cancelSessionTools(this.sessionId); }
}
```

**Multi-turn:** `this.messages` grows with each turn. `onFinish` appends the assistant's response (including any tool calls and results) so context is preserved across multiple `send()` calls in the same session.

**System prompt:** For AI SDK providers, `buildSystemPrompt()` is called with `toolCtx = undefined` — no curl instructions. The tools are self-describing via their Zod schemas.

---

## 6. Tool Definitions

Tools are defined once in `tools/definitions.ts` and shared across all AI SDK providers. Each tool closes over `sessionId` and `emitFn` so `callTool()` can route the result to the right SSE stream.

```ts
export function createDbTools(
    sessionId: string,
    emitFn: (e: HarnessEvent) => void,
): Record<string, Tool> {
    const call = (name: string, args: unknown) =>
        callTool(sessionId, name, args, emitFn);

    return {
        list_tables: tool({
            description: "List all tables in a database schema",
            parameters: z.object({
                schema: z.string().optional().describe("Schema name, default: public"),
            }),
            execute: (args) => call("list_tables", args),
        }),
        describe_table: tool({
            description: "Get columns, types, primary keys, and nullable info for a table",
            parameters: z.object({
                table: z.string(),
                schema: z.string().optional(),
            }),
            execute: (args) => call("describe_table", args),
        }),
        run_query: tool({
            description: "Execute SQL against the live database. Returns columns + rows (up to 50).",
            parameters: z.object({ sql: z.string() }),
            execute: (args) => call("run_query", args),
        }),
        sample_table: tool({
            description: "Sample N rows from a table",
            parameters: z.object({
                table: z.string(),
                schema: z.string().optional(),
                n: z.number().optional().describe("Number of rows, default 20"),
            }),
            execute: (args) => call("sample_table", args),
        }),
        count_rows: tool({
            description: "Count rows in a table with optional WHERE filter",
            parameters: z.object({
                table: z.string(),
                schema: z.string().optional(),
                where: z.string().optional(),
            }),
            execute: (args) => call("count_rows", args),
        }),
        explain_query: tool({
            description: "Get the execution plan for a SQL query",
            parameters: z.object({
                sql: z.string(),
                analyze: z.boolean().optional().describe("Use EXPLAIN ANALYZE, default false"),
            }),
            execute: (args) => call("explain_query", args),
        }),
        get_indexes: tool({
            description: "List indexes on a table",
            parameters: z.object({ table: z.string(), schema: z.string().optional() }),
            execute: (args) => call("get_indexes", args),
        }),
        get_foreign_keys: tool({
            description: "List foreign key relationships for a table",
            parameters: z.object({ table: z.string(), schema: z.string().optional() }),
            execute: (args) => call("get_foreign_keys", args),
        }),
        column_stats: tool({
            description: "Get NULL%, distinct count, min, max for a column",
            parameters: z.object({
                table: z.string(),
                column: z.string(),
                schema: z.string().optional(),
            }),
            execute: (args) => call("column_stats", args),
        }),
        find_nulls: tool({
            description: "Find columns with NULL values in a table",
            parameters: z.object({ table: z.string(), schema: z.string().optional() }),
            execute: (args) => call("find_nulls", args),
        }),
        get_distinct_values: tool({
            description: "Get top N distinct values and their counts for a column",
            parameters: z.object({
                table: z.string(),
                column: z.string(),
                schema: z.string().optional(),
                limit: z.number().optional(),
            }),
            execute: (args) => call("get_distinct_values", args),
        }),
        check_fk_integrity: tool({
            description: "Check for orphaned foreign key rows",
            parameters: z.object({ table: z.string(), schema: z.string().optional() }),
            execute: (args) => call("check_fk_integrity", args),
        }),
        read_file: tool({
            description: "Read content of an open editor tab",
            parameters: z.object({
                fileId: z.string().optional(),
                fileName: z.string().optional(),
                lineStart: z.number().optional(),
                lineEnd: z.number().optional(),
            }),
            execute: (args) => call("read_file", args),
        }),
        write_file: tool({
            description: "Create or update an editor tab with SQL or other content",
            parameters: z.object({
                fileId: z.string().optional().describe("Use to update an existing file precisely"),
                fileName: z.string().describe("Descriptive filename e.g. find-null-users.sql"),
                content: z.string(),
            }),
            execute: (args) => call("write_file", args),
        }),
        list_files: tool({
            description: "List all files created in this session",
            parameters: z.object({}),
            execute: (args) => call("list_files", args),
        }),
        get_query_history: tool({
            description: "Get recent queries from the editor history",
            parameters: z.object({
                limit: z.number().optional().describe("Default 20"),
            }),
            execute: (args) => call("get_query_history", args),
        }),
    };
}
```

---

## 7. Provider Wrappers

Each provider is a thin file that constructs the `LanguageModel` from its credentials and delegates to `AiSdkSession`.

**`providers/openrouter.ts`:**
```ts
export class OpenRouterProvider extends AiSdkSession {
    constructor(config: SessionConfig) {
        const apiKey = config.providerConfig?.apiKey as string ?? "";
        const openrouter = createOpenRouter({ apiKey });
        const modelId = config.model ?? "openai/gpt-4o";
        super(openrouter.chat(modelId), config);
    }

    async isAvailable(): Promise<boolean> {
        return !!(config.providerConfig?.apiKey);
    }
}
```

**`providers/google.ts`:**
```ts
export class GoogleProvider extends AiSdkSession {
    constructor(config: SessionConfig) {
        const apiKey = config.providerConfig?.apiKey as string ?? "";
        const google = createGoogleGenerativeAI({ apiKey });
        const modelId = config.model ?? "gemini-2.5-flash";
        super(google(modelId), config);
    }

    async isAvailable(): Promise<boolean> {
        return !!(config.providerConfig?.apiKey);
    }
}
```

---

## 8. Frontend Changes

### `providers.ts` — add model lists

```ts
openrouter: {
    label: "OpenRouter",
    models: [
        { id: "openai/gpt-4o",                    label: "GPT-4o" },
        { id: "anthropic/claude-sonnet-4-5",       label: "Claude Sonnet" },
        { id: "google/gemini-2.5-flash",           label: "Gemini 2.5 Flash" },
        { id: "meta-llama/llama-4-maverick",       label: "Llama 4 Maverick" },
        { id: "deepseek/deepseek-r1",              label: "DeepSeek R1" },
        { id: "mistralai/mistral-large-2407",      label: "Mistral Large" },
        { id: "qwen/qwen3-235b-a22b",              label: "Qwen3 235B" },
    ],
    supportsModel: true,
    supportsEffort: false,
    requiresApiKey: true,
    apiKeyLabel: "OpenRouter API Key",
    apiKeySettingsKey: "openrouterApiKey",
},
google: {
    label: "Google",
    models: [
        { id: "gemini-2.5-flash",   label: "Gemini 2.5 Flash" },
        { id: "gemini-2.5-pro",     label: "Gemini 2.5 Pro" },
        { id: "gemini-2.0-flash",   label: "Gemini 2.0 Flash" },
    ],
    supportsModel: true,
    supportsEffort: false,
    requiresApiKey: true,
    apiKeyLabel: "Google API Key",
    apiKeySettingsKey: "googleApiKey",
},
```

The existing `gemini` provider entry is removed (replaced by `google`).

### `settings.svelte.ts` — add API key fields

Add `openrouterApiKey: string` and `googleApiKey: string` to `Settings` interface and the store. Persisted via `commandClient.updateAppSetting("openrouter_api_key", v)`.

### `AgentComposer.svelte` — API key input

When the selected provider has `requiresApiKey: true` and the key is empty, show a small inline input in the toolbar area to enter it. No separate settings page needed — inline is lower friction.

### `AgentPanel.svelte` — pass providerConfig

```ts
const sess = await startAgentSession({
    ...existingOpts,
    providerConfig: buildProviderConfig(thread.provider),
});

function buildProviderConfig(provider: string): Record<string, unknown> | undefined {
    if (provider === "openrouter") return { apiKey: settingsStore.openrouterApiKey };
    if (provider === "google")     return { apiKey: settingsStore.googleApiKey };
    return undefined;
}
```

### `claude.ts` (startAgentSession) — add providerConfig param

Add `providerConfig?: Record<string, unknown>` to the opts interface and include it in the `/session/start` body.

### `tools.ts` (buildSystemPrompt) — skip curl instructions for AI SDK providers

The system prompt for OpenRouter/Google providers should NOT include curl tool instructions. Since AI SDK tools are self-describing via Zod schemas, the system prompt only needs DB schema context + file writing guidelines. Pass `toolCtx = undefined` from `AgentPanel.buildPrompt()` when the provider uses AI SDK.

---

## 9. Migration: Gemini CLI → Google AI SDK

The `gemini.ts` CLI provider is deleted. Any existing threads with `provider: "gemini"` will fail gracefully (session not found). Since threads only store the provider name, users simply need to start a new thread with the `google` provider.

The `google` provider replaces `gemini` in:
- `registry.ts`
- `providers.ts` (frontend)
- `PROVIDER_LABELS` in registry

---

## 10. Dependencies to Add

```json
// packages/harness/package.json
"@openrouter/ai-sdk-provider": "^0.4.0",
"@ai-sdk/google": "^1.0.0",
"ai": "^4.0.0",
"zod": "^3.22.0"
```

`ai` and `zod` are likely already present transitively; add them explicitly for `streamText()` and `tool()`.

---

## 11. What Does NOT Change

- HTTP/SSE interface (`/session/start`, `/session/send`, `/session/stop`, `/session/resume`, `/db/`, `/tool-result/`, `/providers`)
- SSE event types (`text.delta`, `thinking.delta`, `tool.started`, `tool.completed`, `tool.input_delta`, `session.init`, `turn.done`, `error`)
- `tool-executor.ts` in the frontend — all tool implementations stay exactly the same
- `AgentPanel.svelte` tool dispatch logic
- Claude, Cursor, Codex, OpenCode providers

---

## 12. isAvailable() Behavior

For AI SDK providers, `isAvailable()` returns `true` if the API key is set in `providerConfig`, `false` otherwise. The `/providers` endpoint reflects this. The frontend ProviderPicker shows a lock icon for providers with no key, and clicking them prompts for the key inline.
