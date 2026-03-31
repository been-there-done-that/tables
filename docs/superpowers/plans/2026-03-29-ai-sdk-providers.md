# AI SDK Providers Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add OpenRouter and Google as native AI SDK providers in the Tables harness, replacing the Gemini CLI provider with `@ai-sdk/google` and wiring up proper tool calling via a shared ToolBridge.

**Architecture:** Extract `pendingToolResults` from `index.ts` into a shared `tool-bridge.ts` module. Build `AiSdkSession` that wraps Vercel AI SDK's `streamText()` with `maxSteps` for automatic tool call looping — tools are Zod-typed closures that call `ToolBridge.callTool()`, which emits `tool.started` to SSE and awaits the frontend's `/tool-result/` POST. Add thin `GoogleProvider` and `OpenRouterProvider` wrappers. Update the frontend to pass API keys via `providerConfig`.

**Tech Stack:** Vercel AI SDK (`ai` v4), `@ai-sdk/google`, `@openrouter/ai-sdk-provider`, Bun, Zod, Svelte 5

---

## File Map

| Action | Path | Purpose |
|--------|------|---------|
| CREATE | `packages/harness/src/tool-bridge.ts` | Shared pending tool results + callTool/resolveToolResult/cancelSessionTools |
| CREATE | `packages/harness/src/tools/definitions.ts` | All 16 DB tools as AI SDK `tool()` schemas with Zod parameters |
| CREATE | `packages/harness/src/session/ai-sdk-session.ts` | `AiSdkSession` — `streamText()` wrapper + tool loop |
| CREATE | `packages/harness/src/providers/google.ts` | `GoogleProvider` — thin wrapper constructing `@ai-sdk/google` model |
| CREATE | `packages/harness/src/providers/openrouter.ts` | `OpenRouterProvider` — thin wrapper constructing `@openrouter/ai-sdk-provider` model |
| DELETE | `packages/harness/src/providers/gemini.ts` | Replaced by `google.ts` |
| DELETE | `packages/harness/src/providers/gemini.test.ts` | Tests for deleted provider |
| MODIFY | `packages/harness/src/index.ts` | Use ToolBridge instead of own pendingToolResults map |
| MODIFY | `packages/harness/src/registry.ts` | Add `google` + `openrouter`, remove `gemini` |
| MODIFY | `packages/harness/package.json` | Add `ai`, `@ai-sdk/google`, `@openrouter/ai-sdk-provider` |
| MODIFY | `src/lib/agent/providers.ts` | Add `requiresApiKey` field, `google` + `openrouter` configs |
| MODIFY | `src/lib/agent/claude.ts` | Add `providerConfig` param to `startAgentSession` |
| MODIFY | `src/lib/stores/settings.svelte.ts` | Add `googleApiKey` + `openrouterApiKey` fields |
| MODIFY | `src/lib/components/agent/AgentComposer.svelte` | Inline API key input for providers with `requiresApiKey` |
| MODIFY | `src/lib/components/agent/AgentPanel.svelte` | Pass `providerConfig` to `startAgentSession`; skip curl instructions for AI SDK providers |

---

## Task 1: Add Harness Dependencies

**Files:**
- Modify: `packages/harness/package.json`

- [ ] **Step 1: Install AI SDK packages**

```bash
cd packages/harness
bun add ai@^4.3.16 @ai-sdk/google@^1.2.0 @openrouter/ai-sdk-provider@^0.4.0
```

Expected: packages added to `node_modules`, `package.json` updated with three new dependencies.

- [ ] **Step 2: Verify install**

```bash
cd packages/harness
bun run -e "import { streamText } from 'ai'; import { createGoogleGenerativeAI } from '@ai-sdk/google'; import { createOpenRouter } from '@openrouter/ai-sdk-provider'; console.log('ok')"
```

Expected output: `ok`

- [ ] **Step 3: Commit**

```bash
git add packages/harness/package.json packages/harness/bun.lock
git commit -m "chore(harness): add ai-sdk, @ai-sdk/google, @openrouter/ai-sdk-provider"
```

---

## Task 2: Create ToolBridge

Extract `pendingToolResults` from `index.ts` into a standalone module with a clean API. This is the glue between AI SDK tool execution and the frontend's Tauri IPC.

**Files:**
- Create: `packages/harness/src/tool-bridge.ts`
- Create: `packages/harness/src/tool-bridge.test.ts`

- [ ] **Step 1: Write failing tests**

Create `packages/harness/src/tool-bridge.test.ts`:

```ts
import { describe, it, expect } from "bun:test";
import { callTool, resolveToolResult, cancelSessionTools } from "./tool-bridge";
import type { HarnessEvent } from "./types";

describe("ToolBridge", () => {
    it("resolves callTool when resolveToolResult is called", async () => {
        const emitted: HarnessEvent[] = [];
        const emit = (e: HarnessEvent) => emitted.push(e);

        const callPromise = callTool("session-1", "list_tables", { schema: "public" }, emit);

        // Simulate frontend posting result
        const started = emitted.find((e) => e.type === "tool.started");
        expect(started).toBeDefined();
        expect((started as any).toolName).toBe("list_tables");
        expect((started as any).requiresResponse).toBe(true);

        const requestId = (started as any).toolId;
        const resolved = resolveToolResult(requestId, [{ table_name: "users" }]);
        expect(resolved).toBe(true);

        const result = await callPromise;
        expect(result).toEqual([{ table_name: "users" }]);

        const completed = emitted.find((e) => e.type === "tool.completed");
        expect(completed).toBeDefined();
        expect((completed as any).toolId).toBe(requestId);
    });

    it("returns false from resolveToolResult for unknown requestId", () => {
        expect(resolveToolResult("nonexistent-id", {})).toBe(false);
    });

    it("cancelSessionTools rejects all pending tools for that session", async () => {
        const emit = (_e: HarnessEvent) => {};

        const p1 = callTool("session-cancel", "run_query", { sql: "SELECT 1" }, emit);
        const p2 = callTool("session-cancel", "list_tables", {}, emit);

        cancelSessionTools("session-cancel");

        const r1 = await p1;
        const r2 = await p2;
        expect((r1 as any).error).toContain("stopped");
        expect((r2 as any).error).toContain("stopped");
    });

    it("cancelSessionTools only affects the target session", async () => {
        const emittedA: HarnessEvent[] = [];
        const pA = callTool("session-A", "list_tables", {}, (e) => emittedA.push(e));

        cancelSessionTools("session-B");  // different session — should not cancel pA

        const startedA = emittedA.find((e) => e.type === "tool.started");
        const idA = (startedA as any).toolId;
        resolveToolResult(idA, { result: "ok" });

        const result = await pA;
        expect((result as any).result).toBe("ok");
    });
});
```

- [ ] **Step 2: Run tests to confirm they fail**

```bash
cd packages/harness && bun test src/tool-bridge.test.ts
```

Expected: module not found error (file doesn't exist yet).

- [ ] **Step 3: Create tool-bridge.ts**

Create `packages/harness/src/tool-bridge.ts`:

```ts
import type { HarnessEvent } from "./types";

type PendingTool = {
    sessionId: string;
    resolve: (v: unknown) => void;
    reject: (e: Error) => void;
};

const pending = new Map<string, PendingTool>();

/**
 * Called by AiSdkSession (and the /db/ HTTP handler) to dispatch a tool call.
 * Emits tool.started to SSE, waits for the frontend to POST /tool-result/:requestId,
 * then emits tool.completed and returns the result.
 */
export async function callTool(
    sessionId: string,
    toolName: string,
    input: unknown,
    emitFn: (e: HarnessEvent) => void,
): Promise<unknown> {
    const requestId = crypto.randomUUID();
    console.error(`[bridge] callTool: tool="${toolName}" requestId="${requestId}"`);

    emitFn({ type: "tool.started", toolId: requestId, toolName, input, requiresResponse: true });

    const result = await new Promise<unknown>((resolve, reject) => {
        pending.set(requestId, { sessionId, resolve, reject });
        setTimeout(() => {
            if (pending.has(requestId)) {
                pending.delete(requestId);
                console.error(`[bridge] tool "${toolName}" (${requestId}) timed out after 30s`);
                reject(new Error(`Tool "${toolName}" timed out after 30s`));
            }
        }, 30_000);
    }).catch((e) => ({ error: String(e) }));

    emitFn({
        type: "tool.completed",
        toolId: requestId,
        output: typeof result === "string" ? result : JSON.stringify(result),
    });

    return result;
}

/**
 * Called by the /tool-result/:requestId HTTP handler.
 * Returns true if a matching pending tool was found and resolved.
 */
export function resolveToolResult(requestId: string, result: unknown): boolean {
    const p = pending.get(requestId);
    if (!p) return false;
    pending.delete(requestId);
    p.resolve(result);
    return true;
}

/**
 * Called when a session's SSE stream is cancelled (user stops the turn).
 * Rejects all pending tool calls for the given session.
 */
export function cancelSessionTools(sessionId: string): void {
    for (const [reqId, p] of pending) {
        if (p.sessionId === sessionId) {
            pending.delete(reqId);
            p.reject(new Error("Turn stopped by user"));
        }
    }
}
```

- [ ] **Step 4: Run tests — expect pass**

```bash
cd packages/harness && bun test src/tool-bridge.test.ts
```

Expected: 4 passing tests.

- [ ] **Step 5: Update index.ts to use ToolBridge**

In `packages/harness/src/index.ts`, replace the existing `pendingToolResults` map and all references with the ToolBridge functions.

Replace the entire top of the file through the `pendingToolResults` declaration:

```ts
// BEFORE (remove these lines):
const pendingToolResults = new Map<string, { sessionId: string; resolve: (v: unknown) => void; reject: (e: Error) => void }>();

// AFTER (add this import at the top):
import { callTool, resolveToolResult, cancelSessionTools } from "./tool-bridge";
```

Replace the stream `cancel()` block in `/session/send`:
```ts
// BEFORE:
cancel() {
    console.error(`[harness] SSE stream cancelled for ${sessionId}`);
    session.setEmit(() => {});
    for (const [reqId, pending] of pendingToolResults) {
        if (pending.sessionId === sessionId) {
            pendingToolResults.delete(reqId);
            pending.reject(new Error("Turn stopped by user"));
        }
    }
},

// AFTER:
cancel() {
    console.error(`[harness] SSE stream cancelled for ${sessionId}`);
    session.setEmit(() => {});
    cancelSessionTools(sessionId);
},
```

Replace the entire `/db/` handler body (keep the path parsing, replace the pending logic):
```ts
// POST /db/:sessionId/:toolName — agent calls this via curl/Bash tool
if (req.method === "POST" && url.pathname.startsWith("/db/")) {
    const parts = url.pathname.split("/"); // ["", "db", sessionId, toolName]
    const pathSessionId = parts[2];
    const toolName = parts[3];

    if (!pathSessionId || !toolName) {
        return Response.json({ error: "invalid path" }, { status: 400, headers: CORS });
    }

    const session = sessions.get(pathSessionId);
    if (!session) {
        return Response.json({ error: "session not found" }, { status: 404, headers: CORS });
    }

    const input = await req.json().catch(() => ({}));
    console.error(`[harness] /db/ received — tool="${toolName}" session="${pathSessionId}" input=${JSON.stringify(input)}`);

    const result = await callTool(
        pathSessionId,
        toolName,
        input,
        (e) => session.emitToolEvent(e),
    );

    return Response.json(result, { headers: CORS });
}
```

Replace the `/tool-result/` handler:
```ts
// POST /tool-result/:requestId — frontend submits tool execution result
if (req.method === "POST" && url.pathname.startsWith("/tool-result/")) {
    const requestId = url.pathname.slice("/tool-result/".length);
    console.error(`[harness] /tool-result/${requestId} received`);
    const body = await req.json().catch(() => ({}));
    const resolved = resolveToolResult(requestId, body);
    if (!resolved) {
        console.error(`[harness] /tool-result/${requestId} — no matching pending tool`);
        return Response.json({ error: "no pending tool for this id" }, { status: 404, headers: CORS });
    }
    return Response.json({ ok: true }, { headers: CORS });
}
```

- [ ] **Step 6: Verify TypeScript compiles**

```bash
cd packages/harness && bun build src/index.ts --target bun 2>&1 | head -20
```

Expected: no TypeScript errors (warnings about unused imports in old code are ok, errors are not).

- [ ] **Step 7: Commit**

```bash
git add packages/harness/src/tool-bridge.ts packages/harness/src/tool-bridge.test.ts packages/harness/src/index.ts
git commit -m "feat(harness): extract ToolBridge from index.ts — shared callTool/resolveToolResult"
```

---

## Task 3: Create Tool Definitions

Define all 16 DB tools as Vercel AI SDK `tool()` schemas. Each tool closes over `sessionId` and `emitFn` so they can call `ToolBridge.callTool()` when executed.

**Files:**
- Create: `packages/harness/src/tools/definitions.ts`

- [ ] **Step 1: Create tools/definitions.ts**

Create `packages/harness/src/tools/definitions.ts`:

```ts
import { tool } from "ai";
import { z } from "zod";
import type { Tool } from "ai";
import { callTool } from "../tool-bridge";
import type { HarnessEvent } from "../types";

/**
 * Create all DB tools as AI SDK tool() definitions.
 * Tools close over sessionId + emitFn so callTool() can route to the right SSE stream.
 */
export function createDbTools(
    sessionId: string,
    emitFn: (e: HarnessEvent) => void,
): Record<string, Tool> {
    const call = (name: string, args: unknown) => callTool(sessionId, name, args, emitFn);

    return {
        list_tables: tool({
            description: "List all tables in a database schema. Call this before writing queries to know what tables exist.",
            parameters: z.object({
                schema: z.string().optional().describe("Schema name. Default: public"),
            }),
            execute: (args) => call("list_tables", args),
        }),

        describe_table: tool({
            description: "Get columns, types, primary keys, nullable flags, and default values for a table. Always call this before writing a query against an unfamiliar table.",
            parameters: z.object({
                table: z.string().describe("Table name"),
                schema: z.string().optional().describe("Schema name. Default: public"),
            }),
            execute: (args) => call("describe_table", args),
        }),

        run_query: tool({
            description: "Execute SQL against the live database. Returns columns + up to 50 rows. May require user approval.",
            parameters: z.object({
                sql: z.string().describe("SQL query to execute"),
            }),
            execute: (args) => call("run_query", args),
        }),

        sample_table: tool({
            description: "Sample N rows from a table to understand its data shape.",
            parameters: z.object({
                table: z.string(),
                schema: z.string().optional(),
                n: z.number().int().positive().optional().describe("Number of rows to sample. Default: 20"),
            }),
            execute: (args) => call("sample_table", args),
        }),

        count_rows: tool({
            description: "Count rows in a table with an optional WHERE filter.",
            parameters: z.object({
                table: z.string(),
                schema: z.string().optional(),
                where: z.string().optional().describe("SQL WHERE clause (without the WHERE keyword)"),
            }),
            execute: (args) => call("count_rows", args),
        }),

        explain_query: tool({
            description: "Get the execution plan for a SQL query to check for performance issues.",
            parameters: z.object({
                sql: z.string(),
                analyze: z.boolean().optional().describe("Use EXPLAIN ANALYZE to include actual runtimes. Default: false"),
            }),
            execute: (args) => call("explain_query", args),
        }),

        get_indexes: tool({
            description: "List indexes on a table including their columns and uniqueness.",
            parameters: z.object({
                table: z.string(),
                schema: z.string().optional(),
            }),
            execute: (args) => call("get_indexes", args),
        }),

        get_foreign_keys: tool({
            description: "List foreign key constraints for a table, including referenced tables and columns.",
            parameters: z.object({
                table: z.string(),
                schema: z.string().optional(),
            }),
            execute: (args) => call("get_foreign_keys", args),
        }),

        column_stats: tool({
            description: "Get statistics for a column: NULL%, distinct count, min value, max value.",
            parameters: z.object({
                table: z.string(),
                column: z.string(),
                schema: z.string().optional(),
            }),
            execute: (args) => call("column_stats", args),
        }),

        find_nulls: tool({
            description: "Find all columns in a table that contain NULL values and report the null count per column.",
            parameters: z.object({
                table: z.string(),
                schema: z.string().optional(),
            }),
            execute: (args) => call("find_nulls", args),
        }),

        get_distinct_values: tool({
            description: "Get the top N most frequent distinct values for a column along with their counts.",
            parameters: z.object({
                table: z.string(),
                column: z.string(),
                schema: z.string().optional(),
                limit: z.number().int().positive().optional().describe("Max distinct values to return. Default: 20"),
            }),
            execute: (args) => call("get_distinct_values", args),
        }),

        check_fk_integrity: tool({
            description: "Check for orphaned rows — foreign key values that reference non-existent parent rows.",
            parameters: z.object({
                table: z.string(),
                schema: z.string().optional(),
            }),
            execute: (args) => call("check_fk_integrity", args),
        }),

        read_file: tool({
            description: "Read the content of an open editor tab. Use fileId (from a previous write_file response) for precise targeting.",
            parameters: z.object({
                fileId: z.string().optional().describe("File ID from a previous write_file response"),
                fileName: z.string().optional().describe("File name if fileId is not available"),
                lineStart: z.number().int().optional(),
                lineEnd: z.number().int().optional(),
            }),
            execute: (args) => call("read_file", args),
        }),

        write_file: tool({
            description: "Create or update an editor tab with SQL or other content. NEVER output SQL in your text response — always use write_file. Use fileId from a previous write_file response to update the same file without creating duplicates.",
            parameters: z.object({
                fileId: z.string().optional().describe("File ID to update an existing file precisely"),
                fileName: z.string().describe("Descriptive filename e.g. find-null-users.sql or orders-analysis.sql"),
                content: z.string().describe("Full file content"),
            }),
            execute: (args) => call("write_file", args),
        }),

        list_files: tool({
            description: "List all files you have created in this session. Call at the start of a task to recover fileIds and avoid creating duplicates.",
            parameters: z.object({}),
            execute: (args) => call("list_files", args),
        }),

        get_query_history: tool({
            description: "Get recent SQL queries from the editor history.",
            parameters: z.object({
                limit: z.number().int().positive().optional().describe("Number of recent queries to return. Default: 20"),
            }),
            execute: (args) => call("get_query_history", args),
        }),
    };
}
```

- [ ] **Step 2: Verify it compiles**

```bash
cd packages/harness && bun build src/tools/definitions.ts --target bun 2>&1 | head -20
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add packages/harness/src/tools/definitions.ts
git commit -m "feat(harness): add DB tool definitions as AI SDK tool() schemas"
```

---

## Task 4: Create AiSdkSession

The shared session class for all Vercel AI SDK providers. Wraps `streamText()` with `maxSteps: 20` and maintains message history for multi-turn conversations.

**Files:**
- Create: `packages/harness/src/session/ai-sdk-session.ts`

- [ ] **Step 1: Create session/ai-sdk-session.ts**

Create `packages/harness/src/session/ai-sdk-session.ts`:

```ts
import { streamText, type LanguageModel, type CoreMessage } from "ai";
import { HttpAdapter } from "../adapters/http-adapter";
import type { SessionConfig } from "../types";
import { createDbTools } from "../tools/definitions";
import { cancelSessionTools } from "../tool-bridge";

export class AiSdkSession extends HttpAdapter {
    protected config: SessionConfig;
    private model: LanguageModel;
    private messages: CoreMessage[] = [];
    private ac = new AbortController();

    constructor(model: LanguageModel, config: SessionConfig) {
        super();
        this.model = model;
        this.config = config;
        this.messages = [{ role: "system", content: config.systemPrompt }];
    }

    protected onStop(): void {
        this.ac.abort();
        cancelSessionTools(this.config.sessionId);
    }

    async isAvailable(): Promise<boolean> {
        return true;
    }

    async send(text: string): Promise<void> {
        this.messages.push({ role: "user", content: text });

        try {
            const tools = createDbTools(this.config.sessionId, (e) => this.emitFn(e));
            let responseMessages: CoreMessage[] = [];

            const result = streamText({
                model: this.model,
                messages: this.messages,
                tools,
                maxSteps: 20,
                abortSignal: this.ac.signal,
                onFinish: ({ response }) => {
                    responseMessages = response.messages as CoreMessage[];
                },
            });

            for await (const chunk of result.fullStream) {
                if (this.isAborted()) break;
                if (chunk.type === "text-delta") {
                    this.emitFn({ type: "text.delta", content: chunk.textDelta });
                } else if (chunk.type === "reasoning") {
                    this.emitFn({ type: "thinking.delta", content: chunk.textDelta });
                }
            }

            if (!this.isAborted()) {
                this.messages.push(...responseMessages);
                this.emitFn({ type: "turn.done" });
            }
        } catch (e: unknown) {
            if (!this.isAborted()) {
                this.emitFn({ type: "error", message: String(e) });
            }
        }
    }
}
```

- [ ] **Step 2: Verify it compiles**

```bash
cd packages/harness && bun build src/session/ai-sdk-session.ts --target bun 2>&1 | head -20
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add packages/harness/src/session/ai-sdk-session.ts
git commit -m "feat(harness): add AiSdkSession — streamText wrapper with tool call loop"
```

---

## Task 5: Create Google Provider

**Files:**
- Create: `packages/harness/src/providers/google.ts`
- Delete: `packages/harness/src/providers/gemini.ts`
- Delete: `packages/harness/src/providers/gemini.test.ts`

- [ ] **Step 1: Create providers/google.ts**

Create `packages/harness/src/providers/google.ts`:

```ts
import { createGoogleGenerativeAI } from "@ai-sdk/google";
import { AiSdkSession } from "../session/ai-sdk-session";
import type { SessionConfig } from "../types";

export class GoogleProvider extends AiSdkSession {
    constructor(config: SessionConfig) {
        const apiKey = (config.providerConfig?.apiKey as string) ?? "";
        const google = createGoogleGenerativeAI({ apiKey });
        const modelId = config.model ?? "gemini-2.5-flash";
        super(google(modelId), config);
    }

    override async isAvailable(): Promise<boolean> {
        return true; // Frontend checks for API key; harness always reports available
    }
}
```

- [ ] **Step 2: Verify it compiles**

```bash
cd packages/harness && bun build src/providers/google.ts --target bun 2>&1 | head -20
```

Expected: no errors.

- [ ] **Step 3: Delete gemini files**

```bash
rm packages/harness/src/providers/gemini.ts
rm packages/harness/src/providers/gemini.test.ts
```

- [ ] **Step 4: Commit**

```bash
git add packages/harness/src/providers/google.ts
git rm packages/harness/src/providers/gemini.ts packages/harness/src/providers/gemini.test.ts
git commit -m "feat(harness): add GoogleProvider via @ai-sdk/google, remove Gemini CLI provider"
```

---

## Task 6: Create OpenRouter Provider

**Files:**
- Create: `packages/harness/src/providers/openrouter.ts`

- [ ] **Step 1: Create providers/openrouter.ts**

Create `packages/harness/src/providers/openrouter.ts`:

```ts
import { createOpenRouter } from "@openrouter/ai-sdk-provider";
import { AiSdkSession } from "../session/ai-sdk-session";
import type { SessionConfig } from "../types";

export class OpenRouterProvider extends AiSdkSession {
    constructor(config: SessionConfig) {
        const apiKey = (config.providerConfig?.apiKey as string) ?? "";
        const openrouter = createOpenRouter({ apiKey });
        const modelId = config.model ?? "openai/gpt-4o";
        super(openrouter.chat(modelId), config);
    }

    override async isAvailable(): Promise<boolean> {
        return true; // Frontend checks for API key; harness always reports available
    }
}
```

- [ ] **Step 2: Verify it compiles**

```bash
cd packages/harness && bun build src/providers/openrouter.ts --target bun 2>&1 | head -20
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add packages/harness/src/providers/openrouter.ts
git commit -m "feat(harness): add OpenRouterProvider via @openrouter/ai-sdk-provider"
```

---

## Task 7: Update Registry

Wire the new providers into the registry and remove `gemini`.

**Files:**
- Modify: `packages/harness/src/registry.ts`

- [ ] **Step 1: Update registry.ts**

Replace the full contents of `packages/harness/src/registry.ts`:

```ts
import type { Session, SessionConfig } from "./types";
import { ClaudeProvider } from "./providers/claude";
import { CodexProvider } from "./providers/codex";
import { GoogleProvider } from "./providers/google";
import { OpenRouterProvider } from "./providers/openrouter";
import { OpenCodeProvider } from "./providers/opencode";
import { CursorProvider } from "./providers/cursor";

type ProviderFactory = (config: SessionConfig) => Session;

const PROVIDERS: Record<string, ProviderFactory> = {
    claude:      (c) => new ClaudeProvider(c),
    codex:       (c) => new CodexProvider(c),
    google:      (c) => new GoogleProvider(c),
    openrouter:  (c) => new OpenRouterProvider(c),
    opencode:    (c) => new OpenCodeProvider(c),
    cursor:      (c) => new CursorProvider(c),
};

export function createSession(config: SessionConfig): Session {
    const factory = PROVIDERS[config.provider];
    if (!factory) throw new Error(`Unknown provider: "${config.provider}"`);
    return factory(config);
}

export interface AvailableProvider {
    id: string;
    label: string;
    available: boolean;
}

export const PROVIDER_LABELS: Record<string, string> = {
    claude:     "Claude",
    google:     "Google",
    openrouter: "OpenRouter",
    opencode:   "OpenCode",
    codex:      "Codex",
    cursor:     "Cursor",
};

export async function checkAvailability(): Promise<AvailableProvider[]> {
    return Promise.all(
        Object.entries(PROVIDERS).map(async ([id, factory]) => {
            const instance = factory({ sessionId: "", threadId: "", systemPrompt: "", provider: id });
            const available = await instance.isAvailable().catch(() => false);
            instance.stop();
            return { id, label: PROVIDER_LABELS[id] ?? id, available };
        })
    );
}
```

- [ ] **Step 2: Build the full harness to catch any import errors**

```bash
cd packages/harness && bun build src/index.ts --target bun 2>&1 | head -30
```

Expected: no errors. (Warnings about unused vars from debug logs are fine.)

- [ ] **Step 3: Build the harness binary**

```bash
cd packages/harness && bun run build
```

Expected: binary written to `../../src-tauri/binaries/harness-aarch64-apple-darwin` (or the correct triple for your platform).

- [ ] **Step 4: Commit**

```bash
git add packages/harness/src/registry.ts src-tauri/binaries/harness-aarch64-apple-darwin
git commit -m "feat(harness): register Google and OpenRouter providers, remove Gemini CLI"
```

---

## Task 8: Frontend — Settings + startAgentSession

Add API key storage and wire `providerConfig` through the agent session lifecycle.

**Files:**
- Modify: `src/lib/stores/settings.svelte.ts`
- Modify: `src/lib/agent/claude.ts`
- Modify: `src/lib/agent/providers.ts`

- [ ] **Step 1: Add API keys to settings store**

In `src/lib/stores/settings.svelte.ts`, add to the `Settings` interface (after `aiProvider: string`):

```ts
googleApiKey: string;
openrouterApiKey: string;
```

Add to `DEFAULT_SETTINGS`:
```ts
googleApiKey: "",
openrouterApiKey: "",
```

Add getters/setters after `set aiProvider(v: string)`:
```ts
get googleApiKey(): string {
    return settings.googleApiKey;
},
set googleApiKey(v: string) {
    settings.googleApiKey = v;
    commandClient.updateAppSetting("google_api_key", v);
},
get openrouterApiKey(): string {
    return settings.openrouterApiKey;
},
set openrouterApiKey(v: string) {
    settings.openrouterApiKey = v;
    commandClient.updateAppSetting("openrouter_api_key", v);
},
```

Add cases to the `settings-changed` listener switch statement:
```ts
case "google_api_key":
    settings.googleApiKey = value || "";
    return;
case "openrouter_api_key":
    settings.openrouterApiKey = value || "";
    return;
```

- [ ] **Step 2: Add providerConfig to startAgentSession in claude.ts**

In `src/lib/agent/claude.ts`, add `providerConfig?: Record<string, unknown>` to the opts interface:

```ts
export async function startAgentSession(opts: {
    systemPrompt: string;
    sessionId?: string;
    threadId: string;
    resumeSdkSessionId?: string;
    model?: string;
    effort?: "auto" | "low" | "medium" | "high" | "max";
    provider?: string;
    providerConfig?: Record<string, unknown>;   // ADD THIS
    onEvent: (event: AgentEventType) => void;
    abortController: AbortController;
}): Promise<AgentSession>
```

In the `body` object construction (around line 56), add `providerConfig`:
```ts
const body: Record<string, unknown> = {
    sessionId,
    threadId: opts.threadId,
    systemPrompt: opts.systemPrompt,
    model: opts.model,
    effort: opts.effort,
    provider: opts.provider,
    providerConfig: opts.providerConfig,   // ADD THIS
};
```

- [ ] **Step 3: Add google + openrouter to providers.ts**

In `src/lib/agent/providers.ts`, add `requiresApiKey?: boolean` and `apiKeySettingsKey?: string` to the `ProviderConfig` interface:

```ts
export interface ProviderConfig {
    label: string;
    models: ProviderModel[];
    supportsModel: boolean;
    supportsEffort: boolean;
    requiresApiKey?: boolean;
    apiKeyLabel?: string;
    apiKeySettingsKey?: "googleApiKey" | "openrouterApiKey";
}
```

Remove the `gemini` entry and add `google` + `openrouter` to `PROVIDER_CONFIGS`:

```ts
// REMOVE:
gemini: {
    label: "Gemini",
    models: [
        { id: "gemini-2.5-flash", label: "2.5 Flash" },
        { id: "gemini-2.5-pro",   label: "2.5 Pro" },
    ],
    supportsModel: true,
    supportsEffort: false,
},

// ADD:
google: {
    label: "Google",
    models: [
        { id: "gemini-2.5-flash", label: "Gemini 2.5 Flash" },
        { id: "gemini-2.5-pro",   label: "Gemini 2.5 Pro" },
        { id: "gemini-2.0-flash", label: "Gemini 2.0 Flash" },
    ],
    supportsModel: true,
    supportsEffort: false,
    requiresApiKey: true,
    apiKeyLabel: "Google API Key",
    apiKeySettingsKey: "googleApiKey",
},
openrouter: {
    label: "OpenRouter",
    models: [
        { id: "openai/gpt-4o",                  label: "GPT-4o" },
        { id: "anthropic/claude-sonnet-4-5",     label: "Claude Sonnet" },
        { id: "google/gemini-2.5-flash",         label: "Gemini 2.5 Flash" },
        { id: "meta-llama/llama-4-maverick",     label: "Llama 4 Maverick" },
        { id: "deepseek/deepseek-r1",            label: "DeepSeek R1" },
        { id: "mistralai/mistral-large-2407",    label: "Mistral Large" },
    ],
    supportsModel: true,
    supportsEffort: false,
    requiresApiKey: true,
    apiKeyLabel: "OpenRouter API Key",
    apiKeySettingsKey: "openrouterApiKey",
},
```

- [ ] **Step 4: Run svelte-check**

```bash
pnpm check 2>&1 | grep -E "Error|error" | head -20
```

Expected: no new errors (existing errors unrelated to this change are ok).

- [ ] **Step 5: Commit**

```bash
git add src/lib/stores/settings.svelte.ts src/lib/agent/claude.ts src/lib/agent/providers.ts
git commit -m "feat(frontend): add Google/OpenRouter provider configs + API key settings"
```

---

## Task 9: Frontend — AgentComposer API Key Input + AgentPanel providerConfig

Show an inline API key field when a provider requires one but has no key set. Pass `providerConfig` when starting sessions.

**Files:**
- Modify: `src/lib/components/agent/AgentComposer.svelte`
- Modify: `src/lib/components/agent/AgentPanel.svelte`

- [ ] **Step 1: Add API key input to AgentComposer.svelte**

In `src/lib/components/agent/AgentComposer.svelte`, find the `const providerConfig = $derived(...)` line (around line 57) and add a derived for API key state after it:

```ts
const apiKeySettingsKey = $derived(providerConfig.apiKeySettingsKey);
const hasApiKey = $derived(
    !providerConfig.requiresApiKey ||
    (apiKeySettingsKey ? !!settingsStore[apiKeySettingsKey] : true)
);
```

In the toolbar area (where model picker and effort picker live, around line 390), add the API key input directly before the closing `</div>` of the toolbar row:

```svelte
<!-- API key input (shown when provider requires a key but none is set) -->
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

- [ ] **Step 2: Update AgentPanel.svelte — pass providerConfig + skip curl instructions**

In `src/lib/components/agent/AgentPanel.svelte`, find `buildPrompt(sessionId: string)` (around line 81) and update it to skip tool instructions for AI SDK providers:

```ts
const AI_SDK_PROVIDERS = new Set(["google", "openrouter"]);

function buildPrompt(sessionId: string) {
    const conn = schemaStore.activeConnection!;
    const port = harnessStore.port ?? 0;
    const schema = schemaStore.activeSchema ?? "public";
    const openTabs = windowState.activeSession?.views
        .filter((v) => v.type === "editor")
        .map((v) => ({ id: v.id, title: v.title })) ?? [];
    // AI SDK providers use native function calling — no curl instructions needed
    const useNativeFunctionCalling = AI_SDK_PROVIDERS.has(currentProvider);
    return buildSystemPrompt(
        schemaStore.databases,
        schemaStore.selectedDatabase,
        conn.engine,
        (useNativeFunctionCalling || port <= 0) ? undefined : { port, sessionId, schema },
        openTabs,
        planMode,
    );
}
```

Find the `startAgentSession` call (around line 124) and add `providerConfig`:

```ts
const sess = await startAgentSession({
    sessionId,
    threadId: thread.id,
    systemPrompt: buildPrompt(sessionId),
    model: settingsStore.aiModel,
    effort: settingsStore.aiEffort,
    provider: thread.provider,
    providerConfig: buildProviderConfig(thread.provider),   // ADD
    resumeSdkSessionId: thread.sdkSessionId ?? undefined,
    onEvent: handleEvent,
    abortController: ac,
});
```

Add the `buildProviderConfig` helper function just before `startThread`:

```ts
function buildProviderConfig(provider: string): Record<string, unknown> | undefined {
    if (provider === "google")      return { apiKey: settingsStore.googleApiKey };
    if (provider === "openrouter")  return { apiKey: settingsStore.openrouterApiKey };
    return undefined;
}
```

- [ ] **Step 3: Run svelte-check**

```bash
pnpm check 2>&1 | grep -E "Error|error" | head -20
```

Expected: no new errors.

- [ ] **Step 4: Commit**

```bash
git add src/lib/components/agent/AgentComposer.svelte src/lib/components/agent/AgentPanel.svelte
git commit -m "feat(frontend): wire API key input and providerConfig for Google/OpenRouter"
```

---

## Task 10: End-to-End Test + Final Build

- [ ] **Step 1: Run all harness tests**

```bash
cd packages/harness && bun test
```

Expected: all tests pass (tool-bridge tests + codex tests).

- [ ] **Step 2: Build harness binary**

```bash
cd packages/harness && bun run build
```

Expected: binary written to `../../src-tauri/binaries/harness-aarch64-apple-darwin` with no errors.

- [ ] **Step 3: Run frontend type check**

```bash
pnpm check
```

Expected: no new TypeScript/Svelte errors.

- [ ] **Step 4: Start the app and smoke test Google**

```bash
pnpm tauri dev
```

1. Open agent panel → click provider picker → select **Google**
2. If no API key is set, an API key input should appear in the composer toolbar
3. Enter a valid Google API key (from https://aistudio.google.com/app/apikeys)
4. Type "list the tables in this database" — expect `list_tables` tool called, results shown
5. Type "write a query to count rows in the users table" — expect `describe_table` + `write_file` tool calls, SQL written to editor tab

- [ ] **Step 5: Smoke test OpenRouter**

1. Select **OpenRouter** in provider picker
2. Enter a valid OpenRouter API key (from https://openrouter.ai/keys)
3. Select model (e.g. GPT-4o)
4. Type "describe the schema" — expect tool calls, response

- [ ] **Step 6: Final commit**

```bash
git add -A
git commit -m "feat: AI SDK providers — Google + OpenRouter with native tool calling"
git push
```

---

## Self-Review

**Spec coverage check:**
- ✅ OpenRouter provider (`openrouter.ts`)
- ✅ Google provider (`google.ts`)
- ✅ Replaces Gemini CLI (delete `gemini.ts`, `gemini.test.ts`)
- ✅ ToolBridge extracted (`tool-bridge.ts`)
- ✅ AiSdkSession (`session/ai-sdk-session.ts`)
- ✅ Tool definitions (`tools/definitions.ts`)
- ✅ Frontend API key inputs (AgentComposer)
- ✅ Model pickers for both providers (providers.ts)
- ✅ providerConfig passed through session lifecycle
- ✅ System prompt skips curl instructions for AI SDK providers
- ✅ Multi-turn history via `this.messages`
- ✅ `isAvailable()` always returns true for API key providers
- ✅ Registry updated, gemini removed

**Placeholder scan:** No TBDs or TODOs found.

**Type consistency:**
- `AiSdkSession` defined in Task 4, extended in Tasks 5+6 ✅
- `callTool` / `resolveToolResult` / `cancelSessionTools` defined in Task 2, used in Tasks 3, 4, 7 ✅
- `createDbTools(sessionId, emitFn)` defined in Task 3, called in Task 4 ✅
- `providerConfig.apiKeySettingsKey` typed as `"googleApiKey" | "openrouterApiKey"` in Task 8, used in Task 9 ✅
- `AI_SDK_PROVIDERS` Set defined and used in Task 9 (`AgentPanel.svelte`) ✅
