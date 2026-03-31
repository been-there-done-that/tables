# Agent Panel Polish Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Apply plan mode, file tools, approval gate, and UX polish to the merged agent panel codebase.

**Architecture:** This plan runs *after* `multi-provider-harness` has been merged into `ai`. The merge brings in thread persistence, provider registry, `tool-executor.ts`, and SSE-based `claude.ts`. This plan adds what the merge does NOT bring: plan mode enforcement, `read_file` tool, `fileId`-based targeting, Monaco line-focus, approval UI in `ToolCallCard`, and the three bug fixes for stale-session coercion, stop-draining approvals, and planMode restart.

**Tech Stack:** Svelte 5 runes, TypeScript strict, Tauri 2 invoke(), Monaco editor, bits-ui

---

## Post-Merge Baseline (what the merged codebase has)

- `src/lib/agent/tool-executor.ts` — `dispatchTool()` + `executeTool()` with 16 tools. **Missing**: `read_file`, `fileId` in `write_file`.
- `src/lib/stores/agent.svelte.ts` — `persistMessage`, `persistToolCall`, `loadThread`. **Missing**: `"awaiting"` status, `setToolCallRunning`, `initialStatus` param.
- `src/lib/components/agent/AgentPanel.svelte` — thread management (`startThread`, `ThreadPicker`), turn timer. **Missing**: `planMode`, `pendingApprovals`, `onFocusFile`, approval dispatch.
- `src/lib/components/agent/AgentComposer.svelte` — model/effort selectors, TipTap composer. **Missing**: `planMode` prop, plan toggle button.
- `src/lib/components/agent/ToolCallCard.svelte` — status icons, elapsed time. **Missing**: `"awaiting"` UI, approve/reject buttons, file-focus link.
- `src/lib/components/agent/MessageList.svelte` — message/tool timeline. **Missing**: `onApprove`, `onReject` props.
- `src/lib/agent/tools.ts` — `buildSystemPrompt`. **Missing**: `openTabs`, `planMode` params, `read_file` in tool table.
- `src/lib/components/SqlTestingEditor.svelte` — Monaco editor. **Missing**: `revealAt` effect for line-range focus.

---

## File Map

| File | Action | What changes |
|---|---|---|
| `src/lib/agent/tools.ts` | Modify | Add `openTabs`, `planMode` params; plan section; tabs section; `read_file` in tool table |
| `src/lib/agent/tool-executor.ts` | Modify | Add `read_file` case; add `fileId` targeting to `write_file` |
| `src/lib/stores/agent.svelte.ts` | Modify | Add `"awaiting"` status; `initialStatus` param; `setToolCallRunning`; stale-session coercion in `loadThread`; `failToolCall` with output |
| `src/lib/components/agent/AgentPanel.svelte` | Modify | Plan mode state; pendingApprovals map; approval dispatch; stop drain; focus handler; restart on toggle |
| `src/lib/components/agent/AgentComposer.svelte` | Modify | `planMode` + `onPlanModeToggle` props; plan toggle button |
| `src/lib/components/agent/ToolCallCard.svelte` | Modify | `"awaiting"` icon; approval panel; file-focus link; `onApprove`/`onReject`/`onFocusFile` props |
| `src/lib/components/agent/MessageList.svelte` | Modify | `onApprove`, `onReject` props; pass through to `ToolCallCard` |
| `src/lib/components/SqlTestingEditor.svelte` | Modify | `revealAt` effect |

---

## Task 1: tools.ts — openTabs + planMode + read_file

**Files:**
- Modify: `src/lib/agent/tools.ts`

The merged `tools.ts` has the original `buildSystemPrompt(databases, activeDb, engine, toolCtx?)` signature. We extend it to accept `openTabs` and `planMode`.

- [ ] **Step 1: Update the signature and add plan + tabs sections**

Replace the entire `buildSystemPrompt` function:

```ts
export function buildSystemPrompt(
    databases: MetaDatabase[],
    activeDb: string | null,
    engine: string | null,
    toolCtx?: { port: number; sessionId: string; schema: string },
    openTabs?: Array<{ id: string; title: string }>,
    planMode?: boolean,
): string {
    const engineLabel = engine ?? "SQL";
    const dbLabel = activeDb ?? "unknown";
    const schemaSection = buildSchemaMarkdown(databases, activeDb);
    const toolSection = toolCtx
        ? buildToolInstructions(toolCtx.port, toolCtx.sessionId, toolCtx.schema)
        : "";
    const tabsSection =
        openTabs && openTabs.length > 0
            ? `## Open Editor Tabs\n\nThe user currently has these files open:\n${openTabs.map((t) => `- ${t.title} (id: ${t.id})`).join("\n")}\n\nUse read_file or write_file with the id for precise targeting (avoids ambiguity when multiple tabs share a name).\n\n`
            : "";
    const planSection = planMode
        ? `## Plan Mode

You are operating in Plan Mode. Follow this protocol strictly:

1. Before using any tools, write a brief **## Plan** section listing what you intend to do and why
2. Read-only tools (describe_table, sample_table, count_rows, column_stats, etc.) execute automatically — use them freely
3. write_file executes automatically — use it to draft and iterate on queries before running them
4. **run_query requires user approval** — when you call run_query, the user is shown the SQL and must explicitly approve before it executes. Do not assume approval.
5. If a run_query is rejected, the tool returns an error. Acknowledge it, then either revise the query in write_file and try again, or ask the user what to change.

The approval gate exists so the user can review every SQL execution against their live data. Work WITH this — gather info first, draft in write_file, then run with run_query.\n\n`
        : "";

    return `You are an expert ${engineLabel} database analyst integrated into Tables, a desktop database IDE.

Active connection: ${engineLabel} — database: "${dbLabel}"

${schemaSection}
${planSection}${tabsSection}${toolSection}
Guidelines:
- NEVER output SQL or code directly in your chat response text — always use write_file to write or update files
- Be concise and precise
- When asked to write a query, use write_file immediately without preamble
- If a query could be destructive (DELETE, DROP, TRUNCATE), add a warning comment inside the file
- Prefer readable formatting with proper indentation
- If a request is ambiguous or could be interpreted multiple ways, ask one targeted clarifying question before proceeding`;
}
```

- [ ] **Step 2: Add read_file to buildToolInstructions**

In the `buildToolInstructions` function, find the tool table markdown and add `read_file` row after `write_file`:

```ts
| \`read_file\` | \`fileId?\`, \`fileName?\`, \`lineStart?\`, \`lineEnd?\` | Read content of an open tab; returns fileId for future reference |
| \`write_file\` | \`fileId?\`, \`fileName\`, \`content\` | Create or update a tab; use fileId from a previous write_file response to update the same file precisely |
```

Also update the File Writing section to document that `write_file` returns `fileId`:

```ts
Response: {"ok": true, "action": "created"|"updated", "fileId": "...", "fileName": "...", "lines": N}
```

- [ ] **Step 3: Verify TypeScript — no errors**

```bash
pnpm check 2>&1 | grep -i "tools.ts"
```

Expected: no errors for `tools.ts`.

- [ ] **Step 4: Commit**

```bash
git add src/lib/agent/tools.ts
git commit -m "feat(agent): add openTabs, planMode params and read_file to system prompt builder"
```

---

## Task 2: tool-executor.ts — read_file + fileId write_file

**Files:**
- Modify: `src/lib/agent/tool-executor.ts`

The merged `tool-executor.ts` has `write_file` that only matches by title. We add `fileId` targeting and a new `read_file` case. The `windowState.activeSession.views` array has items with shape `{ id: string; type: string; title: string; data: Record<string, unknown> }`.

- [ ] **Step 1: Add read_file case**

In `executeTool`, after the `open_in_editor` case (line ~250), add:

```ts
case "read_file": {
    const { fileId, fileName, lineStart, lineEnd } = inp as {
        fileId?: string;
        fileName?: string;
        lineStart?: number;
        lineEnd?: number;
    };
    const session = windowState.activeSession;
    if (!session) return { error: "no active session" };

    const view = fileId
        ? session.views.find((v) => v.id === fileId)
        : session.views.find((v) => v.type === "editor" && v.title === fileName);

    if (!view) {
        const available = session.views
            .filter((v) => v.type === "editor")
            .map((v) => `${v.title} (id: ${v.id})`)
            .join(", ");
        return { error: `File not found. Open tabs: ${available || "none"}` };
    }

    const raw = (view.data as Record<string, unknown>)?.content as string | undefined;
    const content = raw ?? "";
    const lines = content.split("\n");
    const slice =
        lineStart != null
            ? lines.slice(lineStart - 1, (lineEnd ?? lineStart) - 1 + 1).join("\n")
            : content;

    return {
        fileId: view.id,
        fileName: view.title,
        content: slice,
        lines: lines.length,
    };
}
```

- [ ] **Step 2: Update write_file to use fileId**

Replace the existing `write_file` case (~line 257):

```ts
case "write_file": {
    const { fileId: targetId, fileName, content } = inp as {
        fileId?: string;
        fileName: string;
        content: string;
    };
    const session = windowState.activeSession;
    if (!session) return { error: "no active session" };

    // Prefer fileId match for precision; fall back to title match
    const existing = targetId
        ? session.views.find((v) => v.id === targetId)
        : session.views.find((v) => v.title === fileName);

    if (existing) {
        existing.data = existing.data ?? {};
        (existing.data as Record<string, unknown>).content = content;
        existing.streamingContent = undefined;
        return {
            ok: true,
            action: "updated",
            fileId: existing.id,
            fileName: existing.title,
            lines: content.split("\n").length,
        };
    } else {
        session.openView("editor", fileName, { content });
        const created = session.views.find((v) => v.title === fileName);
        return {
            ok: true,
            action: "created",
            fileId: created?.id ?? null,
            fileName,
            lines: content.split("\n").length,
        };
    }
}
```

- [ ] **Step 3: Type-check**

```bash
pnpm check 2>&1 | grep -i "tool-executor"
```

Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add src/lib/agent/tool-executor.ts
git commit -m "feat(agent): add read_file tool and fileId targeting for write_file"
```

---

## Task 3: agent.svelte.ts — awaiting status + stale coercion + failToolCall output

**Files:**
- Modify: `src/lib/stores/agent.svelte.ts`

Three changes: (a) add `"awaiting"` to the status union, (b) `addToolCall` accepts `initialStatus`, (c) `setToolCallRunning` to transition awaiting→running, (d) `failToolCall` accepts optional `output` string, (e) `loadThread` coerces any `"running"` or `"awaiting"` status loaded from SQLite → `"error"` (Bug #3).

- [ ] **Step 1: Update AgentToolCall type**

Find the interface:
```ts
export interface AgentToolCall {
    ...
    status: "running" | "done" | "error";
    ...
}
```

Change to:
```ts
export interface AgentToolCall {
    id: string;
    toolName: string;
    input: unknown;
    status: "running" | "awaiting" | "done" | "error";
    output?: string;
    timestamp: number;
    startedAt: number;
    completedAt?: number;
}
```

- [ ] **Step 2: Update addToolCall to accept initialStatus**

Find:
```ts
addToolCall(toolId: string, toolName: string, input: unknown) {
    const tc: AgentToolCall = {
        ...
        status: "running",
        ...
    };
```

Replace with:
```ts
addToolCall(toolId: string, toolName: string, input: unknown, initialStatus: "running" | "awaiting" = "running") {
    const tc: AgentToolCall = {
        id: toolId,
        toolName,
        input,
        status: initialStatus,
        timestamp: Date.now(),
        startedAt: Date.now(),
    };
    this.toolCalls.push(tc);
    this.persistToolCall(tc);
}
```

- [ ] **Step 3: Add setToolCallRunning**

After `addToolCall`, add:
```ts
setToolCallRunning(toolId: string) {
    const tc = this.toolCalls.find((t) => t.id === toolId);
    if (!tc) return;
    tc.status = "running";
    this.persistToolCall(tc);
}
```

- [ ] **Step 4: Update failToolCall to accept output**

Find:
```ts
failToolCall(toolId: string) {
    const tc = this.toolCalls.find((t) => t.id === toolId);
    if (!tc) return;
    tc.status = "error";
```

Replace with:
```ts
failToolCall(toolId: string, output?: string) {
    const tc = this.toolCalls.find((t) => t.id === toolId);
    if (!tc) return;
    tc.status = "error";
    tc.completedAt = Date.now();
    if (output !== undefined) tc.output = output;
    this.persistToolCall(tc);
}
```

- [ ] **Step 5: Coerce stale statuses in loadThread (Bug #3)**

In `loadThread`, find where `this.toolCalls` is assigned from the SQLite rows:

```ts
this.toolCalls = tools.map((t) => ({
    id: t.id,
    toolName: t.toolName,
    input: JSON.parse(t.input),
    status: t.status as "running" | "done" | "error",
    output: t.output ?? undefined,
    timestamp: t.startedAt * 1000,
    startedAt: t.startedAt * 1000,
    completedAt: t.completedAt != null ? t.completedAt * 1000 : undefined,
}));
```

Replace with:
```ts
this.toolCalls = tools.map((t) => {
    // Any tool that was "running" or "awaiting" when the session ended
    // will never complete — coerce to "error" with an explanation.
    const rawStatus = t.status as "running" | "awaiting" | "done" | "error";
    const isStale = rawStatus === "running" || rawStatus === "awaiting";
    return {
        id: t.id,
        toolName: t.toolName,
        input: JSON.parse(t.input),
        status: isStale ? ("error" as const) : rawStatus,
        output: isStale
            ? "Session ended while tool was pending"
            : (t.output ?? undefined),
        timestamp: t.startedAt * 1000,
        startedAt: t.startedAt * 1000,
        completedAt: t.completedAt != null ? t.completedAt * 1000 : undefined,
    };
});
```

- [ ] **Step 6: Type-check**

```bash
pnpm check 2>&1 | grep -i "agent.svelte"
```

Expected: no errors.

- [ ] **Step 7: Commit**

```bash
git add src/lib/stores/agent.svelte.ts
git commit -m "feat(agent): add awaiting status, setToolCallRunning, stale session coercion on load"
```

---

## Task 4: AgentPanel.svelte — plan mode + approval gate

**Files:**
- Modify: `src/lib/components/agent/AgentPanel.svelte`

This is the central change. We add plan mode state, the approval map, the modified `tool.started` handler, approve/reject functions, stop drain (Bug #2), focus handler, and a `$effect` to restart the session when planMode is toggled mid-session (Bug #1).

- [ ] **Step 1: Add plan mode state and types at the top of `<script>`**

After the existing `let abortController = $state<AbortController | null>(null);` block, add:

```ts
// Plan mode — approval gate for run_query
let planMode = $state(false);
const APPROVAL_REQUIRED = new Set(["run_query"]);

interface PendingApproval {
    toolName: string;
    input: unknown;
    ctx: ToolContext;
}
const pendingApprovals = new Map<string, PendingApproval>();
```

- [ ] **Step 2: Update buildPrompt to pass openTabs and planMode**

Replace the existing `buildPrompt` function:

```ts
function buildPrompt(sessionId: string) {
    const conn = schemaStore.activeConnection!;
    const port = harnessStore.port ?? 0;
    const schema = schemaStore.activeSchema ?? "public";
    const session = windowState.activeSession;
    const openTabs = session?.views
        .filter((v) => v.type === "editor")
        .map((v) => ({ id: v.id, title: v.title }));
    return buildSystemPrompt(
        schemaStore.databases,
        schemaStore.selectedDatabase,
        conn.engine,
        port > 0 ? { port, sessionId, schema } : undefined,
        openTabs,
        planMode,
    );
}
```

- [ ] **Step 3: Update the tool.started handler to check for approval**

Find the `tool.started` case in `handleEvent`:
```ts
case "tool.started": {
    if (streamingMsgId) {
        agentStore.finalizeMessage(streamingMsgId);
        streamingMsgId = null;
    }
    agentStore.addToolCall(event.toolId, event.toolName, event.input);
    const ctx = getToolContext();
    if (ctx) {
        dispatchTool(event.toolName, event.toolId, event.input, ctx).catch((e) => {
            console.error("[AgentPanel] tool dispatch error:", e);
        });
    }
    break;
}
```

Replace with:
```ts
case "tool.started": {
    if (streamingMsgId) {
        agentStore.finalizeMessage(streamingMsgId);
        streamingMsgId = null;
    }
    const ctx = getToolContext();
    const needsApproval = planMode && APPROVAL_REQUIRED.has(event.toolName) && !!ctx;
    agentStore.addToolCall(
        event.toolId,
        event.toolName,
        event.input,
        needsApproval ? "awaiting" : "running",
    );
    if (needsApproval) {
        pendingApprovals.set(event.toolId, {
            toolName: event.toolName,
            input: event.input,
            ctx: ctx!,
        });
    } else if (ctx) {
        dispatchTool(event.toolName, event.toolId, event.input, ctx).catch((e) => {
            console.error("[AgentPanel] tool dispatch error:", e);
        });
    }
    break;
}
```

- [ ] **Step 4: Add approveToolCall and rejectToolCall functions**

After the `handleEvent` function, add:

```ts
function approveToolCall(toolId: string) {
    const pending = pendingApprovals.get(toolId);
    if (!pending) return;
    pendingApprovals.delete(toolId);
    agentStore.setToolCallRunning(toolId);
    dispatchTool(pending.toolName, toolId, pending.input, pending.ctx).catch((e) => {
        console.error("[AgentPanel] approved tool dispatch error:", e);
    });
}

function rejectToolCall(toolId: string) {
    const pending = pendingApprovals.get(toolId);
    if (!pending) return;
    pendingApprovals.delete(toolId);
    agentStore.failToolCall(toolId, "User rejected execution");
    // POST error to harness so the blocked curl call unblocks
    const { port, sessionId } = pending.ctx;
    fetch(`http://127.0.0.1:${port}/tool-result/${toolId}`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ error: "User rejected SQL execution in plan mode" }),
    }).catch(console.error);
}
```

- [ ] **Step 5: Update stop() to drain pendingApprovals (Bug #2)**

Find the existing `stop()` function:
```ts
function stop() {
    stopTurnTimer();
    abortController?.abort();
    ...
}
```

Add the drain at the top of `stop()`:
```ts
function stop() {
    // Drain any pending approvals so the harness unblocks
    for (const [toolId, pending] of pendingApprovals) {
        agentStore.failToolCall(toolId, "Session stopped");
        const { port, sessionId } = pending.ctx;
        fetch(`http://127.0.0.1:${port}/tool-result/${toolId}`, {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({ error: "Session stopped by user" }),
        }).catch(console.error);
    }
    pendingApprovals.clear();

    stopTurnTimer();
    abortController?.abort();
    if (streamingMsgId) {
        agentStore.finalizeMessage(streamingMsgId);
        streamingMsgId = null;
    }
    agentStore.addTurnSummary(turnElapsed, sessionModel ?? "", true);
    agentStore.setStatus("idle");
}
```

- [ ] **Step 6: Add handleFocusFile**

```ts
function handleFocusFile(fileId: string, lineStart?: number, lineEnd?: number) {
    const session = windowState.activeSession;
    if (!session) return;
    // Find by UUID first (precise), fall back to title
    let view = session.views.find((v) => v.id === fileId);
    if (!view) view = session.views.find((v) => v.title === fileId);
    if (!view) return;
    windowState.setActiveView(session.id, view.id);
    const prevSeq = ((view.data as Record<string, unknown>)?.revealAt as { seq?: number } | undefined)?.seq ?? 0;
    view.data = {
        ...(view.data as Record<string, unknown>),
        revealAt: {
            start: lineStart ?? 1,
            end: lineEnd ?? lineStart ?? 1,
            seq: prevSeq + 1,
        },
    };
}
```

- [ ] **Step 7: Add planMode restart $effect (Bug #1)**

After the existing connection-change `$effect`, add:

```ts
// Restart the harness session when planMode is toggled mid-conversation,
// so the system prompt reflects the new mode. Guard: only restart if the
// thread has messages (avoid restarting a fresh empty thread).
let prevPlanMode = planMode;
$effect(() => {
    const current = planMode;
    if (current === prevPlanMode) return;
    prevPlanMode = current;
    if (agentStore.messages.length > 0 && threadsStore.activeThread) {
        startThread(threadsStore.activeThread);
    }
});
```

- [ ] **Step 8: Pass planMode props down to children in template**

Find the `<AgentComposer>` usage in the template:
```svelte
<AgentComposer
    onSend={(displayText, fullText, doc) => send(displayText, fullText, doc)}
    onStop={stop}
    running={agentStore.status === "running"}
    disabled={!sessionReady || !!sessionError}
/>
```

Replace with:
```svelte
<AgentComposer
    onSend={(displayText, fullText, doc) => send(displayText, fullText, doc)}
    onStop={stop}
    running={agentStore.status === "running"}
    disabled={!sessionReady || !!sessionError}
    planMode={planMode}
    onPlanModeToggle={() => (planMode = !planMode)}
/>
```

Find the `<MessageList>` usage:
```svelte
<MessageList onRunQuery={handleRunQuery} />
```

Replace with:
```svelte
<MessageList
    onRunQuery={handleRunQuery}
    onFocusFile={handleFocusFile}
    onApprove={approveToolCall}
    onReject={rejectToolCall}
/>
```

- [ ] **Step 9: Add pending approval count badge to header**

In the header `<div class="flex items-center gap-2 shrink-0">`, after the elapsed timer span, add:

```svelte
{#if planMode}
    {@const pendingCount = pendingApprovals.size}
    <span class="flex items-center gap-1 rounded-full bg-amber-400/15 px-2 py-0.5 text-[9.5px] font-medium text-amber-400">
        Plan{pendingCount > 0 ? ` · ${pendingCount}` : ""}
    </span>
{/if}
```

- [ ] **Step 10: Type-check**

```bash
pnpm check 2>&1 | grep -i "AgentPanel"
```

Expected: no errors.

- [ ] **Step 11: Commit**

```bash
git add src/lib/components/agent/AgentPanel.svelte
git commit -m "feat(agent): add plan mode approval gate, stop drain, focus handler, planMode restart"
```

---

## Task 5: AgentComposer.svelte — planMode prop + toggle button

**Files:**
- Modify: `src/lib/components/agent/AgentComposer.svelte`

The merged `AgentComposer` has model/effort selectors as local state but no plan mode. We add it as a prop (AgentPanel owns the state).

- [ ] **Step 1: Add planMode and onPlanModeToggle to Props interface**

Find:
```ts
interface Props {
    onSend: (displayText: string, fullText: string, rawDoc: unknown) => void;
    onStop: () => void;
    running: boolean;
    disabled: boolean;
}
let { onSend, onStop, running, disabled }: Props = $props();
```

Replace with:
```ts
interface Props {
    onSend: (displayText: string, fullText: string, rawDoc: unknown) => void;
    onStop: () => void;
    running: boolean;
    disabled: boolean;
    planMode?: boolean;
    onPlanModeToggle?: () => void;
}
let { onSend, onStop, running, disabled, planMode = false, onPlanModeToggle }: Props = $props();
```

- [ ] **Step 2: Add import for IconMap**

In the imports block, add:
```ts
import IconMap from "@tabler/icons-svelte/icons/map";
```

- [ ] **Step 3: Add plan toggle button in the toolbar**

Find the bottom toolbar row in the template (where the model/effort selectors are). After the effort selector, add the plan toggle:

```svelte
<button
    onclick={onPlanModeToggle}
    title={planMode
        ? "Plan Mode ON — run_query requires your approval before executing"
        : "Plan Mode OFF — click to enable approval gate for SQL execution"}
    class="flex items-center gap-1 rounded px-1.5 py-0.5 text-[10px] transition-colors {planMode
        ? 'text-amber-400 bg-amber-400/10 hover:bg-amber-400/20'
        : 'text-muted-foreground/50 hover:text-muted-foreground hover:bg-muted/50'}"
>
    <IconMap size={11} />
    Plan
</button>
```

- [ ] **Step 4: Type-check**

```bash
pnpm check 2>&1 | grep -i "AgentComposer"
```

Expected: no errors.

- [ ] **Step 5: Commit**

```bash
git add src/lib/components/agent/AgentComposer.svelte
git commit -m "feat(agent): add planMode prop and plan toggle button to AgentComposer"
```

---

## Task 6: ToolCallCard.svelte — awaiting UI + approval buttons + file focus

**Files:**
- Modify: `src/lib/components/agent/ToolCallCard.svelte`

Three additions: (a) `"awaiting"` status renders amber hourglass and hides elapsed time, (b) inline approval panel with SQL preview + Approve/Reject buttons, (c) file name chip in header is a clickable link that calls `onFocusFile` with `fileId` + optional line range.

- [ ] **Step 1: Add new props and imports**

Find the Props interface:
```ts
interface Props {
    toolCall: AgentToolCall;
    onRun?: (sql: string) => void;
    onFocusFile?: (fileId: string, lineStart?: number, lineEnd?: number) => void;
}
let { toolCall, onRun, onFocusFile }: Props = $props();
```

Replace with:
```ts
interface Props {
    toolCall: AgentToolCall;
    onRun?: (sql: string) => void;
    onFocusFile?: (fileId: string, lineStart?: number, lineEnd?: number) => void;
    onApprove?: (toolId: string) => void;
    onReject?: (toolId: string) => void;
}
let { toolCall, onRun, onFocusFile, onApprove, onReject }: Props = $props();
```

Add imports for the new icons at the top of `<script>`:
```ts
import IconHourglassHigh from "@tabler/icons-svelte/icons/hourglass-high";
import IconShieldCheck from "@tabler/icons-svelte/icons/shield-check";
import IconShieldX from "@tabler/icons-svelte/icons/shield-x";
```

- [ ] **Step 2: Update the elapsed time $effect to handle awaiting**

Find the `$effect` that manages `elapsed`. Update the else branch:

```ts
} else {
    if (intervalId !== null) {
        clearInterval(intervalId);
        intervalId = null;
    }
    // "awaiting" shows no elapsed time — the wait is on the user, not the tool.
    elapsed = toolCall.status === "awaiting"
        ? 0
        : toolCall.completedAt != null
            ? toolCall.completedAt - toolCall.startedAt
            : 0;
}
```

- [ ] **Step 3: Update the status icon section**

Find the status icon block in the header:
```svelte
{#if toolCall.status === "running"}
    <IconLoader2 size={10} class="shrink-0 animate-spin text-accent" />
{:else if toolCall.status === "done"}
    <IconCheck size={10} class="shrink-0 text-green-500/80" />
{:else}
    <IconX size={10} class="shrink-0 text-destructive/80" />
{/if}
```

Replace with:
```svelte
{#if toolCall.status === "awaiting"}
    <IconHourglassHigh size={10} class="shrink-0 text-amber-400" />
{:else if toolCall.status === "running"}
    <IconLoader2 size={10} class="shrink-0 animate-spin text-accent" />
{:else if toolCall.status === "done"}
    <IconCheck size={10} class="shrink-0 text-green-500/80" />
{:else}
    <IconX size={10} class="shrink-0 text-destructive/80" />
{/if}
```

- [ ] **Step 4: Update the file name section to be a clickable link**

The header currently has a plain `<span>` for the file name. The file chip section (for write_file/read_file) should become a link that calls `onFocusFile`. Note: this must be a sibling of the expand button, NOT nested inside it (nested buttons are invalid HTML).

Replace the file chip block:
```svelte
{#if toolCall.toolName === "write_file" || toolCall.toolName === "read_file"}
    {@const inp = toolCall.input as Record<string, unknown>}
    {@const out = (() => { try { return JSON.parse(toolCall.output ?? "{}"); } catch { return {}; } })()}
    {@const fileId = (out.fileId ?? inp.fileId ?? inp.fileName) as string | undefined}
    {@const displayName = (inp.fileName ?? out.fileName ?? fileId ?? "") as string}
    {@const lineStart = inp.lineStart as number | undefined}
    {@const lineEnd = inp.lineEnd as number | undefined}
    {@const lineLabel = lineStart != null ? `:${lineStart}${lineEnd != null && lineEnd !== lineStart ? `-${lineEnd}` : ""}` : ""}
    {#if fileId && onFocusFile}
        <button
            onclick={() => onFocusFile?.(fileId, lineStart, lineEnd)}
            class="truncate font-mono text-[10.5px] text-accent/60 hover:text-accent hover:underline transition-colors"
            title="Jump to {displayName}{lineLabel}"
        > — {displayName}{lineLabel}</button>
    {:else if displayName}
        <span class="truncate font-mono text-[10.5px] text-foreground/50"> — {displayName}{lineLabel}</span>
    {/if}
{:else}
    <span class="flex-1"></span>
{/if}
```

- [ ] **Step 5: Hide elapsed time when awaiting**

Find the elapsed time span:
```svelte
<span class="shrink-0 font-mono text-[9px] ...">
    {formatElapsed(elapsed)}
</span>
```

Wrap it:
```svelte
{#if toolCall.status !== "awaiting"}
    <span
        class="shrink-0 font-mono text-[9px] {toolCall.status === 'running'
            ? 'text-accent'
            : 'text-muted-foreground/40'}"
    >
        {formatElapsed(elapsed)}
    </span>
{/if}
```

- [ ] **Step 6: Add the approval panel after the header div**

After the closing `</div>` of the header section, and before the expandable output section, add:

```svelte
<!-- Approval panel — shown inline when awaiting (not inside the expand section) -->
{#if toolCall.status === "awaiting"}
    <div class="border-t border-amber-400/20 bg-amber-400/5 px-2 py-2">
        {#if toolCall.toolName === "run_query"}
            {@const sql = (toolCall.input as Record<string, unknown>)?.sql as string | undefined}
            {#if sql}
                <pre class="mb-2 max-h-32 overflow-y-auto whitespace-pre-wrap break-all rounded bg-background/60 px-2 py-1.5 text-[10.5px] font-mono text-foreground/80 border border-border/30">{sql}</pre>
            {/if}
        {/if}
        <div class="flex items-center gap-2">
            <span class="flex-1 text-[10px] text-amber-400/80">Awaiting approval to execute</span>
            <button
                onclick={() => onReject?.(toolCall.id)}
                class="flex items-center gap-1 rounded px-2 py-1 text-[10px] text-destructive/70 hover:bg-destructive/10 hover:text-destructive transition-colors"
            >
                <IconShieldX size={11} />
                Reject
            </button>
            <button
                onclick={() => onApprove?.(toolCall.id)}
                class="flex items-center gap-1 rounded bg-green-500/15 px-2 py-1 text-[10px] text-green-500 hover:bg-green-500/25 transition-colors"
            >
                <IconShieldCheck size={11} />
                Approve
            </button>
        </div>
    </div>
{/if}
```

- [ ] **Step 7: Type-check**

```bash
pnpm check 2>&1 | grep -i "ToolCallCard"
```

Expected: no errors.

- [ ] **Step 8: Commit**

```bash
git add src/lib/components/agent/ToolCallCard.svelte
git commit -m "feat(agent): add awaiting status, approval panel, and file focus link to ToolCallCard"
```

---

## Task 7: MessageList.svelte — thread approval props

**Files:**
- Modify: `src/lib/components/agent/MessageList.svelte`

Pass `onApprove`, `onReject` down from AgentPanel through MessageList to ToolCallCard. Update `onFocusFile` signature to include line range.

- [ ] **Step 1: Update Props interface**

Find:
```ts
interface Props {
    onRunQuery?: (sql: string) => void;
    onFocusFile?: (fileId: string) => void;
}
let { onRunQuery, onFocusFile }: Props = $props();
```

Replace with:
```ts
interface Props {
    onRunQuery?: (sql: string) => void;
    onFocusFile?: (fileId: string, lineStart?: number, lineEnd?: number) => void;
    onApprove?: (toolId: string) => void;
    onReject?: (toolId: string) => void;
}
let { onRunQuery, onFocusFile, onApprove, onReject }: Props = $props();
```

- [ ] **Step 2: Pass new props to ToolCallCard**

Find the `<ToolCallCard>` usage in the template:
```svelte
<ToolCallCard
    toolCall={entry.item}
    onRun={onRunQuery}
    onFocusFile={onFocusFile}
/>
```

Replace with:
```svelte
<ToolCallCard
    toolCall={entry.item}
    onRun={onRunQuery}
    onFocusFile={onFocusFile}
    onApprove={onApprove}
    onReject={onReject}
/>
```

- [ ] **Step 3: Type-check**

```bash
pnpm check 2>&1 | grep -i "MessageList"
```

Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add src/lib/components/agent/MessageList.svelte
git commit -m "feat(agent): pass onApprove, onReject, and updated onFocusFile through MessageList"
```

---

## Task 8: SqlTestingEditor.svelte — revealAt line-range focus

**Files:**
- Modify: `src/lib/components/SqlTestingEditor.svelte`

When `view.data.revealAt` is set by `handleFocusFile`, Monaco should scroll to and select the line range. A `seq` counter ensures the effect re-fires even if the same line is targeted twice.

- [ ] **Step 1: Locate where the Monaco editor handle is available**

In `SqlTestingEditor.svelte`, find where `editorHandle` or the Monaco `editor` instance is bound. Look for something like:

```ts
let editorHandle: { editor: monaco.editor.IStandaloneCodeEditor } | null = $state(null);
```

- [ ] **Step 2: Add the revealAt effect**

After the existing editor initialization effects, add:

```ts
$effect(() => {
    const revealAt = view?.data?.revealAt as
        | { start: number; end: number; seq: number }
        | undefined;
    if (!revealAt || !editorHandle?.editor) return;
    const ed = editorHandle.editor;
    void revealAt.seq; // track seq so the effect re-runs on repeated reveals of the same line
    ed.revealLineInCenter(revealAt.start);
    ed.setSelection(
        new monaco.Range(
            revealAt.start,
            1,
            revealAt.end,
            ed.getModel()?.getLineMaxColumn(revealAt.end) ?? 9999,
        ),
    );
    ed.focus();
});
```

- [ ] **Step 3: Type-check**

```bash
pnpm check 2>&1 | grep -i "SqlTestingEditor"
```

Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add src/lib/components/SqlTestingEditor.svelte
git commit -m "feat(editor): reveal and select line range via revealAt signal from agent file focus"
```

---

## Task 9: Rebuild harness binary (Bug #4)

**Files:**
- `packages/harness/` (build artifact)
- `src-tauri/binaries/harness-aarch64-apple-darwin`

After the merge the harness source changed (provider registry, SSE protocol). The binary in `src-tauri/binaries/` is stale and must be rebuilt before `pnpm tauri dev` works.

- [ ] **Step 1: Rebuild**

```bash
cd packages/harness && bun run build:mac-arm
```

Expected output ends with something like:
```
 [3.12s] harness-aarch64-apple-darwin
```

The binary lands at `src-tauri/binaries/harness-aarch64-apple-darwin`.

- [ ] **Step 2: Smoke-test the harness**

```bash
src-tauri/binaries/harness-aarch64-apple-darwin &
HARNESS_PID=$!
sleep 0.5
# Should print a port number
kill $HARNESS_PID
```

Expected: process prints `HARNESS_PORT=XXXXX` to stdout before being killed.

- [ ] **Step 3: Commit the binary**

```bash
git add src-tauri/binaries/harness-aarch64-apple-darwin
git commit -m "chore: rebuild harness binary after multi-provider-harness merge"
```

---

## Task 10: End-to-end smoke test

No automated tests for the UI — manual verification checklist.

- [ ] **Step 1: Start dev mode**

```bash
pnpm tauri dev
```

Expected: app starts, no console errors about harness connection.

- [ ] **Step 2: Verify plan mode toggle**

1. Open agent panel, connect to a database
2. Click "Plan" button in composer toolbar — button goes amber
3. "Plan" pill appears in agent panel header
4. Type "count all rows in users" and send
5. When Claude calls `run_query`, a ToolCallCard appears with amber hourglass and amber approval panel showing the SQL
6. Click "Approve" — card transitions to running → done
7. Click "Reject" instead — card transitions to error with "User rejected" message

- [ ] **Step 3: Verify read_file + file focus**

1. Ask Claude to "write a query to select users" — it should call `write_file`, creating a tab
2. The ToolCallCard header shows the file name as a clickable link
3. Click the link — Monaco editor focuses and scrolls to the file
4. Ask Claude to "read the file you just wrote" — it calls `read_file` and returns the content

- [ ] **Step 4: Verify stop drains approvals**

1. Enable plan mode, send a query that generates multiple `run_query` calls
2. While approval cards are showing, click the stop button
3. All pending approval cards should transition to error
4. Harness should not be left with blocked pending tool results

- [ ] **Step 5: Verify thread load coerces stale tools**

1. Start a session, trigger a `run_query` approval, do NOT approve
2. Restart the app (simulates session end while awaiting)
3. Load the thread — the tool card should show as "error" with "Session ended while tool was pending"

---

## Self-Review

**Spec coverage check:**
- ✅ Bug #1 (planMode restart) → Task 4 Step 7
- ✅ Bug #2 (stop drains approvals) → Task 4 Step 5
- ✅ Bug #3 (awaiting→error on loadThread) → Task 3 Step 5
- ✅ Bug #4 (harness rebuild) → Task 9
- ✅ UX #5 (plan mode header indicator) → Task 4 Step 9
- ✅ read_file tool → Task 2 Step 1 + Task 1 Step 2
- ✅ fileId in write_file → Task 2 Step 2
- ✅ openTabs in system prompt → Task 1 Step 1
- ✅ planMode in system prompt → Task 1 Step 1
- ✅ awaiting status in agent store → Task 3
- ✅ Approval UI in ToolCallCard → Task 6
- ✅ File focus link in ToolCallCard → Task 6 Step 4
- ✅ Monaco revealAt → Task 8
- ✅ MessageList prop threading → Task 7
- ✅ AgentComposer plan toggle → Task 5

**Not in this plan (deferred to Plan B):**
- ❌ `## Plan` section rendering with amber border (UX #6) — deferred
- ❌ `agent_plans` + `agent_plan_steps` tables (Architecture #7) — deferred to Plan B
- ❌ `spawn_subagent` tool (Phase 3) — deferred to Plan B

**Placeholder scan:** None found.

**Type consistency:** All uses of `AgentToolCall.status` with `"awaiting"` added consistently across agent.svelte.ts (Step 1), addToolCall (Step 2), and ToolCallCard (Step 3 of Task 6).
