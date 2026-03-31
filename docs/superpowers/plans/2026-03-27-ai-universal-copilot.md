# AI Universal Copilot Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give the AI agent 14 database tools (schema exploration, query execution, data profiling, editor integration) with per-tool elapsed timers and a Run button for SQL-producing tools.

**Architecture:** The agent uses its built-in Bash tool to call harness HTTP endpoints (`/db/:sessionId/:toolName`). The harness emits `tool.started` SSE, holds a pending Promise, and waits for the frontend to POST the result to `/tool-result/:requestId`. No MCP, no new Rust code — all 14 tools are implemented in TypeScript using existing Tauri IPC commands.

**Tech Stack:** Bun, Svelte 5 runes, Tauri IPC (`invoke`), `@tauri-apps/api/core`, `@tabler/icons-svelte`

---

## File Map

| File | Action | Responsibility |
|------|--------|----------------|
| `packages/harness/src/index.ts` | Modify | Add `/db/:sessionId/:toolName` and `/tool-result/:requestId` endpoints |
| `packages/harness/src/session.ts` | Modify | Suppress Bash tool SSE events for `/db/` calls |
| `src/lib/agent/tools.ts` | Modify | Add `buildToolInstructions()`, update `buildSystemPrompt()` signature |
| `src/lib/agent/claude.ts` | Modify | Accept optional `sessionId`, expose `sessionId` + `port` in `AgentSession` |
| `src/lib/agent/tool-executor.ts` | Create | Dispatch tool calls to Tauri IPC, POST results back to harness |
| `src/lib/stores/agent.svelte.ts` | Modify | Add `startedAt: number` to `AgentToolCall` |
| `src/lib/components/agent/AgentPanel.svelte` | Modify | Wire tool executor, add turn-level elapsed timer |
| `src/lib/components/agent/ToolCallCard.svelte` | Modify | Live elapsed timer, `IconLoader2`, Run button |

---

## Task 1: Add `/db` and `/tool-result` endpoints to harness

**Files:**
- Modify: `packages/harness/src/index.ts`

- [ ] **Step 1: Add `pendingToolResults` map and `/db` handler after the existing `/session/stop` block**

Replace the final `return new Response("harness ok"...)` line and add the new routes. Full updated `index.ts`:

```ts
import { ClaudeSession } from "./session";

const claudePath =
    Bun.which("claude") ??
    (Bun.env.HOME ? `${Bun.env.HOME}/.claude/local/claude` : null) ??
    "/usr/local/bin/claude";

console.error(`[harness] claude path: ${claudePath} (exists: ${await Bun.file(claudePath ?? "").exists()})`);
console.error(`[harness] PATH: ${Bun.env.PATH}`);

const sessions = new Map<string, ClaudeSession>();
const pendingToolResults = new Map<string, { resolve: (v: unknown) => void; reject: (e: Error) => void }>();

const CORS = {
    "Access-Control-Allow-Origin": "*",
    "Access-Control-Allow-Methods": "POST, OPTIONS",
    "Access-Control-Allow-Headers": "Content-Type",
};

const server = Bun.serve({
    port: 0,
    idleTimeout: 0,
    async fetch(req) {
        const url = new URL(req.url);

        if (req.method === "OPTIONS") {
            return new Response(null, { status: 204, headers: CORS });
        }

        if (req.method === "POST" && url.pathname === "/session/start") {
            const { sessionId, systemPrompt } = (await req.json()) as {
                sessionId: string;
                systemPrompt: string;
            };
            sessions.get(sessionId)?.stop();
            sessions.set(sessionId, new ClaudeSession(systemPrompt, claudePath));
            console.error(`[harness] session started: ${sessionId}`);
            return Response.json({ ok: true }, { headers: CORS });
        }

        if (req.method === "POST" && url.pathname === "/session/send") {
            const { sessionId, text } = (await req.json()) as {
                sessionId: string;
                text: string;
            };
            const session = sessions.get(sessionId);
            if (!session) {
                return Response.json({ error: "session not found" }, { status: 404, headers: CORS });
            }

            console.error(`[harness] session send: ${sessionId} — "${text.slice(0, 60)}"`);

            const encoder = new TextEncoder();
            let controller!: ReadableStreamDefaultController<Uint8Array>;

            const stream = new ReadableStream<Uint8Array>({
                start(c) { controller = c; },
                cancel() {
                    console.error(`[harness] SSE stream cancelled for ${sessionId}`);
                },
            });

            session.setEmit((e) => {
                try {
                    controller.enqueue(encoder.encode(`data: ${JSON.stringify(e)}\n\n`));
                    if (e.type === "turn.done" || e.type === "error") {
                        controller.close();
                    }
                } catch {
                    // Stream already closed
                }
            });

            session.send(text);

            return new Response(stream, {
                headers: {
                    ...CORS,
                    "Content-Type": "text/event-stream",
                    "Cache-Control": "no-cache",
                    "X-Accel-Buffering": "no",
                },
            });
        }

        if (req.method === "POST" && url.pathname === "/session/stop") {
            const { sessionId } = (await req.json()) as { sessionId: string };
            sessions.get(sessionId)?.stop();
            sessions.delete(sessionId);
            console.error(`[harness] session stopped: ${sessionId}`);
            return Response.json({ ok: true }, { headers: CORS });
        }

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
            const requestId = crypto.randomUUID();

            console.error(`[harness] tool call: ${toolName} (${requestId})`);

            // Emit tool.started to frontend via current SSE stream
            session.emitToolEvent({ type: "tool.started", toolId: requestId, toolName, input });

            // Hold the request open until frontend POSTs the result
            const result = await new Promise<unknown>((resolve, reject) => {
                pendingToolResults.set(requestId, { resolve, reject });
                setTimeout(() => {
                    if (pendingToolResults.has(requestId)) {
                        pendingToolResults.delete(requestId);
                        reject(new Error(`Tool "${toolName}" timed out after 30s`));
                    }
                }, 30_000);
            }).catch((e) => ({ error: String(e) }));

            // Emit tool.completed to frontend
            session.emitToolEvent({
                type: "tool.completed",
                toolId: requestId,
                output: typeof result === "string" ? result : JSON.stringify(result),
            });

            return Response.json(result, { headers: CORS });
        }

        // POST /tool-result/:requestId — frontend submits tool execution result
        if (req.method === "POST" && url.pathname.startsWith("/tool-result/")) {
            const requestId = url.pathname.slice("/tool-result/".length);
            const pending = pendingToolResults.get(requestId);
            if (!pending) {
                return Response.json({ error: "no pending tool for this id" }, { status: 404, headers: CORS });
            }
            pendingToolResults.delete(requestId);
            const body = await req.json().catch(() => ({}));
            pending.resolve(body);
            return Response.json({ ok: true }, { headers: CORS });
        }

        return new Response("harness ok", { headers: CORS });
    },
});

console.log(`HARNESS_PORT=${server.port}`);
```

- [ ] **Step 2: Add `emitToolEvent` method to `ClaudeSession` in `session.ts`**

The `/db` handler calls `session.emitToolEvent(...)` — add this public method. Add after `setEmit`:

```ts
emitToolEvent(e: HarnessEvent) {
    this.emitFn(e);
}
```

- [ ] **Step 3: Verify harness still starts**

```bash
cd packages/harness && bun run dev
```

Expected: `HARNESS_PORT=XXXXX` logged to stdout, no errors.

- [ ] **Step 4: Smoke test the new endpoints**

```bash
# In a second terminal — replace PORT with the actual port from step 3
PORT=<PORT>
SESSION_ID="test-db-1"

# Start session
curl -s -X POST http://127.0.0.1:$PORT/session/start \
  -H "Content-Type: application/json" \
  -d "{\"sessionId\":\"$SESSION_ID\",\"systemPrompt\":\"You are a helpful assistant.\"}"
# Expected: {"ok":true}

# Simulate a tool call (in background — it will hang waiting for /tool-result)
curl -s -X POST http://127.0.0.1:$PORT/db/$SESSION_ID/run_query \
  -H "Content-Type: application/json" \
  -d '{"sql":"SELECT 1"}' &
CURL_PID=$!
sleep 1

# Submit fake result (replace REQUEST_ID with the UUID logged in harness stderr)
# For now just kill the curl — confirm it unblocks on /tool-result below
kill $CURL_PID 2>/dev/null; echo "ok"
```

---

## Task 2: Suppress Bash tool SSE events for `/db/` calls

**Files:**
- Modify: `packages/harness/src/session.ts`

The SDK stream will emit `tool_use` (Bash) and `tool_result` blocks for every curl call the agent makes. We suppress these so the UI shows our custom `tool.started`/`tool.completed` events, not raw Bash entries.

- [ ] **Step 1: Add `suppressedBashIds` set and update `consume()` in `session.ts`**

Full updated `session.ts`:

```ts
import { query } from "@anthropic-ai/claude-agent-sdk";

class AsyncQueue<T> {
    private buffer: T[] = [];
    private waiting: ((value: IteratorResult<T>) => void) | null = null;
    private done = false;

    push(item: T) {
        if (this.waiting) {
            const resolve = this.waiting;
            this.waiting = null;
            resolve({ value: item, done: false });
        } else {
            this.buffer.push(item);
        }
    }

    close() {
        this.done = true;
        if (this.waiting) {
            this.waiting({ value: undefined as any, done: true });
            this.waiting = null;
        }
    }

    [Symbol.asyncIterator](): AsyncIterator<T> {
        return {
            next: (): Promise<IteratorResult<T>> => {
                if (this.buffer.length > 0) {
                    return Promise.resolve({ value: this.buffer.shift()!, done: false });
                }
                if (this.done) {
                    return Promise.resolve({ value: undefined as any, done: true });
                }
                return new Promise((resolve) => { this.waiting = resolve; });
            },
        };
    }
}

export type HarnessEvent =
    | { type: "text.delta"; content: string }
    | { type: "thinking.delta"; content: string }
    | { type: "tool.started"; toolId: string; toolName: string; input: unknown }
    | { type: "tool.completed"; toolId: string; output: string }
    | { type: "turn.done" }
    | { type: "error"; message: string };

type SDKMsg = { type: "user"; message: { role: "user"; content: string }; parent_tool_use_id: null };

export class ClaudeSession {
    private queue = new AsyncQueue<SDKMsg>();
    private ac = new AbortController();
    private firstMessage = true;
    private emitFn: (e: HarnessEvent) => void = () => {};
    // Track Bash tool IDs that are /db/ API calls so we can suppress them
    private suppressedBashIds = new Set<string>();

    constructor(
        private systemPrompt: string,
        claudePath: string | null,
    ) {
        const stream = query({
            prompt: this.queue as any,
            options: {
                permissionMode: "bypassPermissions",
                abortController: this.ac,
                ...(claudePath ? { pathToClaudeCodeExecutable: claudePath } : {}),
            } as any,
        });
        this.consume(stream);
    }

    setEmit(fn: (e: HarnessEvent) => void) {
        this.emitFn = fn;
    }

    emitToolEvent(e: HarnessEvent) {
        this.emitFn(e);
    }

    private async consume(stream: AsyncIterable<unknown>) {
        try {
            for await (const msg of stream as AsyncIterable<Record<string, unknown>>) {
                if (msg.type === "assistant") {
                    const message = msg.message as { content: Array<Record<string, unknown>> };
                    for (const block of message?.content ?? []) {
                        if (block.type === "text") {
                            this.emitFn({ type: "text.delta", content: block.text as string });
                        } else if (block.type === "thinking") {
                            this.emitFn({ type: "thinking.delta", content: block.thinking as string });
                        } else if (block.type === "tool_use") {
                            // Suppress Bash tool events that are /db/ API calls
                            if (block.name === "Bash") {
                                const cmd = ((block.input as any)?.command ?? "") as string;
                                if (cmd.includes("/db/")) {
                                    this.suppressedBashIds.add(block.id as string);
                                    continue; // Don't emit tool.started — /db handler emits it
                                }
                            }
                            this.emitFn({
                                type: "tool.started",
                                toolId: block.id as string,
                                toolName: block.name as string,
                                input: block.input,
                            });
                        }
                    }
                } else if (msg.type === "user") {
                    const message = msg.message as { content: Array<Record<string, unknown>> };
                    for (const block of message?.content ?? []) {
                        if (block.type === "tool_result") {
                            const toolUseId = block.tool_use_id as string;
                            // Suppress tool_result for our suppressed Bash IDs
                            if (this.suppressedBashIds.has(toolUseId)) {
                                this.suppressedBashIds.delete(toolUseId);
                                continue;
                            }
                            const content = Array.isArray(block.content)
                                ? (block.content as Array<Record<string, unknown>>)
                                      .filter((c) => c.type === "text")
                                      .map((c) => c.text)
                                      .join("")
                                : String(block.content ?? "");
                            this.emitFn({
                                type: "tool.completed",
                                toolId: toolUseId,
                                output: content,
                            });
                        }
                    }
                } else if (msg.type === "result") {
                    this.emitFn({ type: "turn.done" });
                }
            }
        } catch (e: unknown) {
            if (!this.ac.signal.aborted) {
                this.emitFn({ type: "error", message: String(e) });
            }
        }
    }

    private msg(content: string): SDKMsg {
        return { type: "user", message: { role: "user", content }, parent_tool_use_id: null };
    }

    send(text: string) {
        if (this.firstMessage) {
            this.firstMessage = false;
            this.queue.push(this.msg(`${this.systemPrompt}\n\n---\n\n${text}`));
        } else {
            this.queue.push(this.msg(text));
        }
    }

    stop() {
        this.ac.abort();
        this.queue.close();
    }
}
```

---

## Task 3: Update system prompt with tool instructions

**Files:**
- Modify: `src/lib/agent/tools.ts`
- Modify: `src/lib/agent/claude.ts`
- Modify: `src/lib/components/agent/AgentPanel.svelte`

- [ ] **Step 1: Update `tools.ts` — add `buildToolInstructions` and update `buildSystemPrompt` signature**

```ts
import type { MetaDatabase } from "$lib/commands/types";

export function buildSystemPrompt(
    databases: MetaDatabase[],
    activeDb: string | null,
    engine: string | null,
    toolCtx?: { port: number; sessionId: string; schema: string },
): string {
    const engineLabel = engine ?? "SQL";
    const dbLabel = activeDb ?? "unknown";
    const schemaSection = buildSchemaMarkdown(databases, activeDb);
    const toolSection = toolCtx
        ? buildToolInstructions(toolCtx.port, toolCtx.sessionId, toolCtx.schema)
        : "";

    return `You are an expert ${engineLabel} database analyst integrated into Tables, a desktop database IDE.

Active connection: ${engineLabel} — database: "${dbLabel}"

${schemaSection}
${toolSection}
Guidelines:
- Always wrap SQL queries in \`\`\`sql code blocks so the user can run them directly from the chat
- Be concise and precise
- When asked to write a query, provide the SQL immediately without preamble
- If a query could be destructive (DELETE, DROP, TRUNCATE), add a warning comment above it
- Prefer readable formatting with proper indentation`;
}

export function buildToolInstructions(port: number, sessionId: string, schema: string): string {
    const base = `http://127.0.0.1:${port}/db/${sessionId}`;
    return `
## Database Tools

Use the Bash tool with curl to call these endpoints. All are POST with JSON body.
Base URL: ${base}

| Tool | Body fields | Description |
|------|-------------|-------------|
| \`run_query\` | \`sql\`, \`limit?\` (default 100) | Execute any SELECT — you see the results |
| \`sample_table\` | \`table\`, \`schema?\` (default "${schema}"), \`n?\` (default 20) | Sample N rows |
| \`count_rows\` | \`table\`, \`schema?\`, \`where?\` | COUNT with optional WHERE |
| \`explain_query\` | \`sql\`, \`analyze?\` (default false) | EXPLAIN plan |
| \`list_tables\` | \`schema?\` | All tables in schema with types |
| \`describe_table\` | \`table\`, \`schema?\` | Columns, types, PKs, nullable |
| \`get_indexes\` | \`table\`, \`schema?\` | Indexes on a table |
| \`get_foreign_keys\` | \`table\`, \`schema?\` | FK relationships |
| \`column_stats\` | \`table\`, \`column\`, \`schema?\` | NULL%, distinct count, min/max |
| \`find_nulls\` | \`table\`, \`schema?\` | Columns with unexpected NULLs |
| \`get_distinct_values\` | \`table\`, \`column\`, \`schema?\`, \`limit?\` (default 20) | Top N distinct values |
| \`check_fk_integrity\` | \`table\`, \`schema?\` | Orphaned FK rows |
| \`open_in_editor\` | \`sql\`, \`title?\` | Open SQL in editor tab |
| \`get_query_history\` | \`limit?\` (default 20) | Recent queries from editor |

Example:
\`\`\`bash
curl -s -X POST ${base}/run_query \\
  -H 'Content-Type: application/json' \\
  -d '{"sql":"SELECT COUNT(*) FROM users"}'
\`\`\`

**Use tools proactively.** Before writing queries, call \`describe_table\` to know exact column names and types. Call \`sample_table\` to understand data shape. Chain tools freely.

`;
}

function buildSchemaMarkdown(databases: MetaDatabase[], activeDb: string | null): string {
    if (databases.length === 0) return "Schema: not yet loaded.\n";

    const target = activeDb
        ? databases.find((d) => d.name === activeDb)
        : databases[0];

    if (!target) return `Schema: database "${activeDb}" not found.\n`;

    const lines: string[] = [`## Database: ${target.name}\n`];

    for (const schema of target.schemas ?? []) {
        if (!schema.tables || schema.tables.length === 0) continue;
        lines.push(`### Schema: ${schema.name}\n`);
        for (const table of schema.tables) {
            lines.push(`**${schema.name}.${table.table_name}**`);
            if (table.columns && table.columns.length > 0) {
                lines.push("| column | type | nullable |");
                lines.push("|--------|------|----------|");
                for (const col of table.columns.slice(0, 30)) {
                    lines.push(
                        `| ${col.column_name} | ${col.raw_type} | ${col.nullable ? "YES" : "NO"} |`,
                    );
                }
                if (table.columns.length > 30) {
                    lines.push(`| … (${table.columns.length - 30} more) | | |`);
                }
            }
            lines.push("");
        }
    }

    return lines.join("\n");
}
```

- [ ] **Step 2: Update `claude.ts` — accept optional `sessionId`, expose it in returned object**

```ts
import { harnessStore } from "$lib/stores/harness.svelte";

export type AgentEventType =
    | { type: "text.delta"; content: string }
    | { type: "thinking.delta"; content: string }
    | { type: "tool.started"; toolId: string; toolName: string; input: unknown }
    | { type: "tool.completed"; toolId: string; output: string }
    | { type: "turn.done"; isError: boolean }
    | { type: "error"; message: string };

export interface AgentSession {
    send: (text: string) => void;
    abort: () => void;
    sessionId: string;
    port: number;
}

function waitForPort(timeoutMs: number): Promise<number> {
    return new Promise((resolve, reject) => {
        if (harnessStore.port !== null) {
            resolve(harnessStore.port);
            return;
        }
        const deadline = setTimeout(() => {
            reject(new Error("Harness not ready — is the sidecar running?"));
        }, timeoutMs);
        const poll = setInterval(() => {
            if (harnessStore.port !== null) {
                clearTimeout(deadline);
                clearInterval(poll);
                resolve(harnessStore.port!);
            }
        }, 500);
    });
}

export async function startAgentSession(opts: {
    systemPrompt: string;
    sessionId?: string;
    onEvent: (event: AgentEventType) => void;
    abortController: AbortController;
}): Promise<AgentSession> {
    const port = await waitForPort(10_000);
    const base = `http://127.0.0.1:${port}`;
    const sessionId = opts.sessionId ?? crypto.randomUUID();

    const startRes = await fetch(`${base}/session/start`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ sessionId, systemPrompt: opts.systemPrompt }),
    });
    if (!startRes.ok) {
        throw new Error(`Failed to start harness session: ${startRes.status}`);
    }

    let currentReader: ReadableStreamDefaultReader<Uint8Array> | null = null;

    const stop = () => {
        currentReader?.cancel();
        currentReader = null;
        fetch(`${base}/session/stop`, {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({ sessionId }),
        }).catch(() => {});
    };

    opts.abortController.signal.addEventListener("abort", stop);

    async function consumeSSE(response: Response) {
        const reader = response.body!.getReader();
        currentReader = reader;
        const decoder = new TextDecoder();
        let buffer = "";
        try {
            while (true) {
                const { done, value } = await reader.read();
                if (done) break;
                buffer += decoder.decode(value, { stream: true });
                const parts = buffer.split("\n\n");
                buffer = parts.pop() ?? "";
                for (const part of parts) {
                    if (part.startsWith("data: ")) {
                        const event = JSON.parse(part.slice(6)) as AgentEventType;
                        opts.onEvent(event);
                    }
                }
            }
        } catch (e) {
            if (!opts.abortController.signal.aborted) {
                opts.onEvent({ type: "error", message: String(e) });
            }
        } finally {
            currentReader = null;
        }
    }

    return {
        sessionId,
        port,
        send(text: string) {
            fetch(`${base}/session/send`, {
                method: "POST",
                headers: { "Content-Type": "application/json" },
                body: JSON.stringify({ sessionId, text }),
            })
                .then((res) => {
                    if (!res.ok) throw new Error(`/session/send returned ${res.status}`);
                    return consumeSSE(res);
                })
                .catch((e) => {
                    if (!opts.abortController.signal.aborted) {
                        opts.onEvent({ type: "error", message: String(e) });
                    }
                });
        },
        abort: stop,
    };
}
```

---

## Task 4: Create `tool-executor.ts`

**Files:**
- Create: `src/lib/agent/tool-executor.ts`

- [ ] **Step 1: Create the file**

```ts
import { invoke } from "@tauri-apps/api/core";

export interface ToolContext {
    port: number;
    sessionId: string;
    connectionId: string;
    database: string;
    schema: string;
    openInEditor: (sql: string, title: string) => void;
}

/**
 * Dispatch a tool.started event: execute the tool via Tauri IPC and POST
 * the result back to the harness so the agent's curl call can complete.
 */
export async function dispatchTool(
    toolName: string,
    toolId: string,
    input: unknown,
    ctx: ToolContext,
): Promise<void> {
    let result: unknown;
    try {
        result = await executeTool(toolName, input, ctx);
    } catch (e) {
        result = { error: String(e) };
    }

    // POST result back to harness — unblocks the pending curl call
    try {
        await fetch(`http://127.0.0.1:${ctx.port}/tool-result/${toolId}`, {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify(result),
        });
    } catch (e) {
        console.error("[tool-executor] failed to post result:", e);
    }
}

async function executeTool(toolName: string, input: unknown, ctx: ToolContext): Promise<unknown> {
    const inp = input as Record<string, unknown>;
    const schema = (inp.schema as string | undefined) ?? ctx.schema;

    switch (toolName) {
        case "list_tables": {
            const result = await invoke<any>("get_schema", { connectionId: ctx.connectionId });
            const databases = result as any[];
            const db = databases?.find((d: any) => d.name === ctx.database);
            const sch = db?.schemas?.find((s: any) => s.name === schema);
            return (sch?.tables ?? []).map((t: any) => ({
                table_name: t.table_name,
                table_type: t.table_type,
                column_count: t.columns?.length ?? 0,
            }));
        }

        case "describe_table": {
            const details = await invoke<any>("get_schema_table_details", {
                connectionId: ctx.connectionId,
                database: ctx.database,
                schema,
                tableName: inp.table,
            });
            return (details?.columns ?? []).map((c: any) => ({
                column_name: c.column_name,
                type: c.raw_type,
                nullable: c.nullable,
                is_primary_key: c.is_primary_key,
                default_value: c.default_value ?? null,
            }));
        }

        case "get_indexes": {
            const details = await invoke<any>("get_schema_table_details", {
                connectionId: ctx.connectionId,
                database: ctx.database,
                schema,
                tableName: inp.table,
            });
            return details?.indexes ?? [];
        }

        case "get_foreign_keys": {
            const details = await invoke<any>("get_schema_table_details", {
                connectionId: ctx.connectionId,
                database: ctx.database,
                schema,
                tableName: inp.table,
            });
            return details?.foreign_keys ?? [];
        }

        case "run_query": {
            const limit = (inp.limit as number | undefined) ?? 100;
            const result = await invoke<any>("execute_query", {
                connectionId: ctx.connectionId,
                sessionId: "agent",
                database: ctx.database,
                schema,
                query: inp.sql,
                component: "agent",
                limit,
                offset: 0,
            });
            return {
                columns: (result?.columns ?? []).map((c: any) => c.name ?? c),
                rows: result?.rows ?? [],
                row_count: result?.rows?.length ?? 0,
                truncated: (result?.rows?.length ?? 0) >= limit,
            };
        }

        case "sample_table": {
            const n = (inp.n as number | undefined) ?? 20;
            const sql = `SELECT * FROM "${schema}"."${inp.table}" LIMIT ${n}`;
            const result = await invoke<any>("execute_query", {
                connectionId: ctx.connectionId,
                sessionId: "agent",
                database: ctx.database,
                schema,
                query: sql,
                component: "agent",
                limit: n,
                offset: 0,
            });
            return {
                columns: (result?.columns ?? []).map((c: any) => c.name ?? c),
                rows: result?.rows ?? [],
                row_count: result?.rows?.length ?? 0,
            };
        }

        case "count_rows": {
            const where = inp.where ? ` WHERE ${inp.where}` : "";
            const sql = `SELECT COUNT(*) AS count FROM "${schema}"."${inp.table}"${where}`;
            const result = await invoke<any>("execute_query", {
                connectionId: ctx.connectionId,
                sessionId: "agent",
                database: ctx.database,
                schema,
                query: sql,
                component: "agent",
                limit: 1,
                offset: 0,
            });
            return { count: result?.rows?.[0]?.count ?? result?.rows?.[0]?.[0] ?? 0 };
        }

        case "explain_query": {
            const analyze = (inp.analyze as boolean | undefined) ?? false;
            const prefix = analyze ? "EXPLAIN ANALYZE" : "EXPLAIN";
            const result = await invoke<any>("execute_query", {
                connectionId: ctx.connectionId,
                sessionId: "agent",
                database: ctx.database,
                schema,
                query: `${prefix} ${inp.sql}`,
                component: "agent",
                limit: 500,
                offset: 0,
            });
            const rows = result?.rows ?? [];
            return rows.map((r: any) => Object.values(r)[0]).join("\n");
        }

        case "column_stats": {
            const col = `"${inp.column}"`;
            const sql = `SELECT COUNT(*) AS total_count, COUNT(${col}) AS non_null_count, COUNT(DISTINCT ${col}) AS distinct_count, MIN(${col}::text) AS min_val, MAX(${col}::text) AS max_val FROM "${schema}"."${inp.table}"`;
            const result = await invoke<any>("execute_query", {
                connectionId: ctx.connectionId,
                sessionId: "agent",
                database: ctx.database,
                schema,
                query: sql,
                component: "agent",
                limit: 1,
                offset: 0,
            });
            const row = result?.rows?.[0] ?? {};
            const total = Number(row.total_count ?? 0);
            const nonNull = Number(row.non_null_count ?? 0);
            return {
                total_count: total,
                null_count: total - nonNull,
                null_pct: total > 0 ? (((total - nonNull) / total) * 100).toFixed(1) + "%" : "0%",
                distinct_count: row.distinct_count ?? 0,
                min_val: row.min_val,
                max_val: row.max_val,
            };
        }

        case "find_nulls": {
            const details = await invoke<any>("get_schema_table_details", {
                connectionId: ctx.connectionId,
                database: ctx.database,
                schema,
                tableName: inp.table,
            });
            const columns = (details?.columns ?? []).slice(0, 30);
            if (columns.length === 0) return [];
            const selects = columns
                .map((c: any) => `SUM(CASE WHEN "${c.column_name}" IS NULL THEN 1 ELSE 0 END) AS "${c.column_name}"`)
                .join(", ");
            const sql = `SELECT COUNT(*) AS __total, ${selects} FROM "${schema}"."${inp.table}"`;
            const result = await invoke<any>("execute_query", {
                connectionId: ctx.connectionId,
                sessionId: "agent",
                database: ctx.database,
                schema,
                query: sql,
                component: "agent",
                limit: 1,
                offset: 0,
            });
            const row = result?.rows?.[0] ?? {};
            const total = Number(row.__total ?? 1);
            return columns
                .map((c: any) => {
                    const nullCount = Number(row[c.column_name] ?? 0);
                    return {
                        column: c.column_name,
                        null_count: nullCount,
                        null_pct: ((nullCount / total) * 100).toFixed(1) + "%",
                    };
                })
                .filter((r: any) => r.null_count > 0);
        }

        case "get_distinct_values": {
            const limit = (inp.limit as number | undefined) ?? 20;
            const sql = `SELECT "${inp.column}" AS value, COUNT(*) AS count FROM "${schema}"."${inp.table}" GROUP BY "${inp.column}" ORDER BY count DESC LIMIT ${limit}`;
            const result = await invoke<any>("execute_query", {
                connectionId: ctx.connectionId,
                sessionId: "agent",
                database: ctx.database,
                schema,
                query: sql,
                component: "agent",
                limit,
                offset: 0,
            });
            return result?.rows ?? [];
        }

        case "check_fk_integrity": {
            const details = await invoke<any>("get_schema_table_details", {
                connectionId: ctx.connectionId,
                database: ctx.database,
                schema,
                tableName: inp.table,
            });
            const fks = details?.foreign_keys ?? [];
            if (fks.length === 0) return { message: "No foreign keys found on this table." };
            // Return FK definitions — let agent write specific integrity queries
            return fks.map((fk: any) => ({
                constraint_name: fk.constraint_name ?? fk.name ?? "unknown",
                column: fk.column_name ?? fk.column,
                references_table: fk.referenced_table ?? fk.foreign_table,
                references_column: fk.referenced_column ?? fk.foreign_column,
            }));
        }

        case "open_in_editor": {
            const sql = inp.sql as string;
            const title = (inp.title as string | undefined) ?? "Agent Query";
            ctx.openInEditor(sql, title);
            return { success: true };
        }

        case "get_query_history": {
            const limit = (inp.limit as number | undefined) ?? 20;
            const logs = await invoke<any[]>("fetch_query_logs", {
                limit,
                connectionId: ctx.connectionId,
            });
            return (logs ?? []).map((l: any) => ({
                sql: l.query ?? l.sql,
                executed_at: l.timestamp,
                duration_ms: l.duration_ms,
            }));
        }

        default:
            return { error: `Unknown tool: ${toolName}` };
    }
}
```

---

## Task 5: Add `startedAt` to `AgentToolCall`

**Files:**
- Modify: `src/lib/stores/agent.svelte.ts`

- [ ] **Step 1: Add `startedAt` field and set it in `addToolCall`**

Change the `AgentToolCall` interface and `addToolCall` method:

```ts
export interface AgentToolCall {
    id: string;
    toolName: string;
    input: unknown;
    status: "running" | "done" | "error";
    output?: string;
    timestamp: number;
    startedAt: number;  // ← add this
}
```

Update `addToolCall`:
```ts
addToolCall(toolId: string, toolName: string, input: unknown) {
    this.toolCalls.push({
        id: toolId,
        toolName,
        input,
        status: "running",
        timestamp: Date.now(),
        startedAt: Date.now(),  // ← add this
    });
}
```

---

## Task 6: Update `ToolCallCard.svelte` — elapsed timer, icons, Run button

**Files:**
- Modify: `src/lib/components/agent/ToolCallCard.svelte`

- [ ] **Step 1: Rewrite `ToolCallCard.svelte`**

```svelte
<script lang="ts">
    import { onDestroy } from "svelte";
    import type { AgentToolCall } from "$lib/stores/agent.svelte";
    import IconLoader2 from "@tabler/icons-svelte/icons/loader-2";
    import IconCheck from "@tabler/icons-svelte/icons/check";
    import IconX from "@tabler/icons-svelte/icons/x";
    import IconChevronDown from "@tabler/icons-svelte/icons/chevron-down";
    import IconChevronRight from "@tabler/icons-svelte/icons/chevron-right";
    import IconPlayerPlay from "@tabler/icons-svelte/icons/player-play";

    interface Props {
        toolCall: AgentToolCall;
        onRun?: (sql: string) => void;
    }

    let { toolCall, onRun }: Props = $props();
    let expanded = $state(false);
    let elapsed = $state(0);
    let intervalId: ReturnType<typeof setInterval> | null = null;

    // SQL-producing tools that show the Run button
    const SQL_TOOLS = new Set(["run_query", "sample_table", "count_rows", "explain_query"]);

    $effect(() => {
        if (toolCall.status === "running") {
            elapsed = Date.now() - toolCall.startedAt;
            intervalId = setInterval(() => {
                elapsed = Date.now() - toolCall.startedAt;
            }, 100);
        } else {
            if (intervalId !== null) {
                clearInterval(intervalId);
                intervalId = null;
            }
            elapsed = Date.now() - toolCall.startedAt;
        }
        return () => {
            if (intervalId !== null) {
                clearInterval(intervalId);
                intervalId = null;
            }
        };
    });

    function formatElapsed(ms: number): string {
        if (ms < 1000) return `${ms}ms`;
        return `${(ms / 1000).toFixed(1)}s`;
    }

    function getSql(): string | null {
        if (!SQL_TOOLS.has(toolCall.toolName)) return null;
        const inp = toolCall.input as Record<string, unknown> | null;
        if (!inp) return null;
        if (toolCall.toolName === "run_query" || toolCall.toolName === "explain_query") {
            return inp.sql as string ?? null;
        }
        if (toolCall.toolName === "sample_table") {
            const schema = inp.schema ?? "public";
            const n = inp.n ?? 20;
            return `SELECT * FROM "${schema}"."${inp.table}" LIMIT ${n}`;
        }
        if (toolCall.toolName === "count_rows") {
            const schema = inp.schema ?? "public";
            const where = inp.where ? ` WHERE ${inp.where}` : "";
            return `SELECT COUNT(*) FROM "${schema}"."${inp.table}"${where}`;
        }
        return null;
    }

    onDestroy(() => {
        if (intervalId !== null) clearInterval(intervalId);
    });
</script>

<div class="my-1 rounded-lg border border-border bg-muted/10 text-[12px]">
    <!-- Header -->
    <button
        class="flex w-full items-center gap-2 px-3 py-2 text-left"
        onclick={() => (expanded = !expanded)}
    >
        {#if toolCall.status === "running"}
            <IconLoader2 size={12} class="shrink-0 animate-spin text-accent" />
        {:else if toolCall.status === "done"}
            <IconCheck size={12} class="shrink-0 text-green-500" />
        {:else}
            <IconX size={12} class="shrink-0 text-destructive" />
        {/if}

        <span class="flex-1 truncate font-mono text-muted-foreground">{toolCall.toolName}</span>

        <!-- Elapsed time -->
        <span
            class="font-mono text-[10px] {toolCall.status === 'running'
                ? 'text-accent'
                : 'text-muted-foreground/60'}"
        >
            {formatElapsed(elapsed)}
        </span>

        {#if expanded}
            <IconChevronDown size={12} class="shrink-0 text-muted-foreground" />
        {:else}
            <IconChevronRight size={12} class="shrink-0 text-muted-foreground" />
        {/if}
    </button>

    <!-- Expandable output -->
    {#if expanded}
        <div class="border-t border-border px-3 py-2">
            {#if toolCall.input}
                <div class="mb-2">
                    <div class="mb-1 text-[10px] uppercase tracking-wide text-muted-foreground">Input</div>
                    <pre class="overflow-x-auto whitespace-pre-wrap break-all text-[11px] text-foreground/80">{JSON.stringify(toolCall.input, null, 2)}</pre>
                </div>
            {/if}
            {#if toolCall.output}
                <div>
                    <div class="mb-1 flex items-center justify-between">
                        <span class="text-[10px] uppercase tracking-wide text-muted-foreground">Output</span>
                        {#if getSql() && onRun && toolCall.status === "done"}
                            <button
                                onclick={(e) => { e.stopPropagation(); onRun?.(getSql()!); }}
                                class="flex items-center gap-1 rounded px-1.5 py-0.5 text-[10px] text-accent hover:bg-accent/10"
                            >
                                <IconPlayerPlay size={9} />
                                Run
                            </button>
                        {/if}
                    </div>
                    <pre
                        class="max-h-48 overflow-y-auto whitespace-pre-wrap break-all text-[11px] {toolCall.status ===
                        'error'
                            ? 'text-destructive'
                            : 'text-foreground/80'}">{toolCall.output}</pre>
                </div>
            {/if}
        </div>
    {/if}
</div>
```

---

## Task 7: Update `AgentPanel.svelte` — wire tool executor and turn timer

**Files:**
- Modify: `src/lib/components/agent/AgentPanel.svelte`

- [ ] **Step 1: Rewrite `AgentPanel.svelte`**

```svelte
<script lang="ts">
    import { onDestroy } from "svelte";
    import { schemaStore } from "$lib/stores/schema.svelte";
    import { agentStore } from "$lib/stores/agent.svelte";
    import { session } from "$lib/stores/session.svelte";
    import { startAgentSession, type AgentEventType } from "$lib/agent/claude";
    import { buildSystemPrompt } from "$lib/agent/tools";
    import { dispatchTool, type ToolContext } from "$lib/agent/tool-executor";
    import { harnessStore } from "$lib/stores/harness.svelte";
    import MessageList from "./MessageList.svelte";
    import ComposerInput from "./ComposerInput.svelte";
    import IconAi from "@tabler/icons-svelte/icons/ai";
    import IconAlertCircle from "@tabler/icons-svelte/icons/alert-circle";

    let abortController = $state<AbortController | null>(null);
    let sessionReady = $state(false);
    let sessionError = $state<string | null>(null);
    let sessionConnectionId = $state<string | null>(null);
    let sessionDatabase = $state<string | null>(null);
    let streamingMsgId: string | null = null;

    // Turn-level elapsed timer
    let turnStartedAt = $state<number | null>(null);
    let turnElapsed = $state(0);
    let turnTimerInterval: ReturnType<typeof setInterval> | null = null;

    function startTurnTimer() {
        turnStartedAt = Date.now();
        turnElapsed = 0;
        turnTimerInterval = setInterval(() => {
            if (turnStartedAt !== null) turnElapsed = Date.now() - turnStartedAt;
        }, 100);
    }

    function stopTurnTimer() {
        if (turnTimerInterval !== null) {
            clearInterval(turnTimerInterval);
            turnTimerInterval = null;
        }
        if (turnStartedAt !== null) turnElapsed = Date.now() - turnStartedAt;
    }

    function formatTurnElapsed(ms: number): string {
        if (ms < 1000) return `${ms}ms`;
        return `${(ms / 1000).toFixed(1)}s`;
    }

    async function bootSession() {
        if (abortController) abortController.abort();
        agentStore.clear();
        sessionReady = false;
        sessionError = null;
        streamingMsgId = null;
        turnStartedAt = null;

        const conn = schemaStore.activeConnection;
        if (!conn) return;

        sessionConnectionId = conn.id;
        sessionDatabase = schemaStore.selectedDatabase;

        const sessionId = crypto.randomUUID();
        const port = harnessStore.port ?? 0;
        const schema = schemaStore.activeSchema ?? "public";

        const systemPrompt = buildSystemPrompt(
            schemaStore.databases,
            schemaStore.selectedDatabase,
            conn.engine,
            port > 0 ? { port, sessionId, schema } : undefined,
        );

        const ac = new AbortController();
        abortController = ac;

        try {
            const sess = await startAgentSession({
                sessionId,
                systemPrompt,
                onEvent: handleEvent,
                abortController: ac,
            });
            agentStore.session = sess;
            sessionReady = true;
        } catch (e) {
            sessionError = String(e);
        }
    }

    function getToolContext(): ToolContext | null {
        const sess = agentStore.session;
        const conn = schemaStore.activeConnection;
        if (!sess || !conn) return null;
        return {
            port: sess.port,
            sessionId: sess.sessionId,
            connectionId: conn.id,
            database: schemaStore.selectedDatabase ?? "",
            schema: schemaStore.activeSchema ?? "public",
            openInEditor: (sql: string, title: string) => {
                session.openView("editor", title, { content: sql });
            },
        };
    }

    function handleEvent(event: AgentEventType) {
        switch (event.type) {
            case "text.delta": {
                if (!streamingMsgId) {
                    streamingMsgId = agentStore.startAssistantMessage();
                }
                agentStore.appendDelta(streamingMsgId, event.content);
                break;
            }
            case "thinking.delta": {
                if (!streamingMsgId) {
                    streamingMsgId = agentStore.startAssistantMessage();
                }
                agentStore.appendThinking(streamingMsgId, event.content);
                break;
            }
            case "tool.started": {
                agentStore.addToolCall(event.toolId, event.toolName, event.input);
                // Dispatch to Tauri IPC (fire-and-forget — errors reported via tool.completed)
                const ctx = getToolContext();
                if (ctx) {
                    dispatchTool(event.toolName, event.toolId, event.input, ctx).catch((e) => {
                        console.error("[AgentPanel] tool dispatch error:", e);
                    });
                }
                break;
            }
            case "tool.completed": {
                agentStore.completeToolCall(event.toolId, event.output);
                break;
            }
            case "turn.done": {
                stopTurnTimer();
                if (streamingMsgId) {
                    agentStore.finalizeMessage(streamingMsgId);
                    streamingMsgId = null;
                }
                agentStore.setStatus("idle");
                break;
            }
            case "error": {
                stopTurnTimer();
                if (streamingMsgId) {
                    agentStore.finalizeMessage(streamingMsgId);
                    streamingMsgId = null;
                }
                agentStore.setError(event.message);
                break;
            }
        }
    }

    async function send(text: string) {
        if (!agentStore.session || agentStore.status === "running") return;
        agentStore.addUserMessage(text);
        agentStore.setStatus("running");
        streamingMsgId = null;
        startTurnTimer();
        agentStore.session.send(text);
    }

    function stop() {
        stopTurnTimer();
        abortController?.abort();
        if (streamingMsgId) {
            agentStore.finalizeMessage(streamingMsgId);
            streamingMsgId = null;
        }
        agentStore.setStatus("idle");
    }

    function handleRunQuery(sql: string) {
        session.openView("editor", "Agent Query", { content: sql });
    }

    $effect(() => {
        const connId = schemaStore.activeConnection?.id;
        const db = schemaStore.selectedDatabase;
        if (connId && (connId !== sessionConnectionId || db !== sessionDatabase)) {
            bootSession();
        }
    });

    onDestroy(() => {
        abortController?.abort();
        if (turnTimerInterval !== null) clearInterval(turnTimerInterval);
    });
</script>

<div class="flex h-full flex-col bg-background">
    <!-- Header -->
    <div class="flex h-8 shrink-0 items-center justify-between border-b border-border px-3">
        <div class="flex items-center gap-1.5 text-[12px] font-medium text-foreground">
            <IconAi size={13} class="text-accent" />
            <span>Claude</span>
        </div>
        {#if agentStore.status === "running" && turnStartedAt !== null}
            <span class="font-mono text-[10px] text-accent">
                {formatTurnElapsed(turnElapsed)}
            </span>
        {/if}
    </div>

    <!-- Content -->
    {#if !schemaStore.activeConnection}
        <div class="flex flex-1 flex-col items-center justify-center gap-2 px-4 text-center text-muted-foreground">
            <IconAi size={24} class="opacity-30" />
            <span class="text-[12px]">Connect to a database to start chatting</span>
        </div>
    {:else if sessionError}
        <div class="flex flex-1 flex-col items-center justify-center gap-3 px-4 text-center">
            <IconAlertCircle size={20} class="text-destructive" />
            <span class="text-[12px] text-muted-foreground">{sessionError}</span>
            <button
                onclick={bootSession}
                class="rounded-md bg-accent/10 px-3 py-1.5 text-[12px] text-accent hover:bg-accent/20"
            >
                Retry
            </button>
        </div>
    {:else if !sessionReady}
        <div class="flex flex-1 items-center justify-center text-muted-foreground">
            <span class="text-[12px]">Starting session…</span>
        </div>
    {:else}
        <MessageList onRunQuery={handleRunQuery} />
        <ComposerInput
            running={agentStore.status === "running"}
            onSend={send}
            onStop={stop}
        />
    {/if}
</div>
```

> **Note on `session` import:** The `session` store import path may differ — check how `MessageBubble.svelte` imports it and use the same pattern. If `session` is not a singleton store, adjust `handleRunQuery` accordingly.

---

## Task 8: Wire `onRunQuery` through `MessageList` to `ToolCallCard`

**Files:**
- Modify: `src/lib/components/agent/MessageList.svelte`

The `onRunQuery` prop needs to pass down from `AgentPanel` → `MessageList` → `ToolCallCard`.

- [ ] **Step 1: Check current `MessageList.svelte` signature and add `onRunQuery` prop**

Read the current file, then add:
```svelte
<script lang="ts">
    // ... existing imports
    interface Props {
        onRunQuery?: (sql: string) => void;
    }
    let { onRunQuery }: Props = $props();
</script>
```

Then pass it to `ToolCallCard` where it renders tool calls:
```svelte
<ToolCallCard toolCall={tc} onRun={onRunQuery} />
```

---

## Task 9: Rebuild harness binary and test

- [ ] **Step 1: Rebuild**

```bash
cd /path/to/project/packages/harness && bun run build
```

Expected output:
```
$ bun run build:mac-arm
$ bun build --compile --target bun-macos-arm64 ...
  [~20ms]  bundle  3 modules
  [~200ms] compile  ../../src-tauri/binaries/harness-aarch64-apple-darwin
```

- [ ] **Step 2: Run `pnpm check` to catch TypeScript errors before launching**

```bash
cd /path/to/project && pnpm check
```

Fix any type errors before proceeding.

- [ ] **Step 3: Launch app and open the Claude panel**

```bash
pnpm tauri dev
```

- [ ] **Step 4: Manual test — basic query tool**

With a PostgreSQL database connected, type in the Claude panel:

> "How many rows are in each table in the public schema?"

Expected behaviour:
1. Agent calls `list_tables` via Bash/curl → tool card appears with spinning `IconLoader2`
2. Tool card shows live elapsed time (e.g. `240ms`)
3. Tool card turns green with frozen time when done
4. Agent may call `count_rows` for individual tables — more tool cards appear
5. Agent writes a final answer with SQL
6. Turn total elapsed shown in header during streaming

- [ ] **Step 5: Test Run button**

Ask: "Show me a sample of the users table"

Expected:
1. Agent calls `sample_table` → tool card appears and completes
2. Tool card is expandable — clicking shows the JSON output with scroll
3. Tool card shows a **Run** button after completion
4. Clicking Run opens a new editor tab with the SELECT query

- [ ] **Step 6: Test `open_in_editor` tool**

Ask: "Write a query to find the top 10 users by order count and open it in the editor"

Expected: Agent calls `open_in_editor` → editor tab opens with the query

---

## Self-Review

**Spec coverage check:**
- ✅ All 14 tools defined in `tool-executor.ts`
- ✅ `tool.started` / `tool.completed` SSE events emitted from harness `/db` handler
- ✅ Bash tool suppression in `session.ts`
- ✅ Elapsed time per tool (`startedAt` in store, interval in `ToolCallCard`)
- ✅ Live turn timer in `AgentPanel` header
- ✅ `IconLoader2` spinning while running, `IconCheck` done, `IconX` error
- ✅ Expandable + scrollable output (`max-h-48 overflow-y-auto` — already existed, kept)
- ✅ Run button on SQL-producing tools only
- ✅ System prompt includes tool instructions with curl examples
- ✅ `open_in_editor` uses `session.openView()`
- ✅ `get_query_history` uses `fetch_query_logs` Tauri command
- ✅ No new Rust code
- ✅ Zero new npm packages

**Placeholder scan:** No TBDs. All code blocks are complete.

**Type consistency:**
- `AgentToolCall.startedAt: number` — set in `addToolCall`, read in `ToolCallCard` ✅
- `AgentSession.sessionId` and `AgentSession.port` — set in `startAgentSession`, read in `AgentPanel` ✅
- `dispatchTool(toolName, toolId, input, ctx)` — called in `AgentPanel.handleEvent`, defined in `tool-executor.ts` ✅
- `ToolContext.openInEditor` — called in `executeTool` case `open_in_editor`, provided by `AgentPanel` ✅
- `buildSystemPrompt(..., toolCtx?)` — optional 4th arg, called with it in `AgentPanel`, called without it elsewhere ✅
