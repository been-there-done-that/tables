# Agent Panel UX Improvements — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix 7 agent panel UX problems: remove spawn_subagent, better tool status icons, query approval toggle, query results returned to agent, file deduplication, thread persistence on restart, and delete confirmation.

**Architecture:** All changes are frontend-only except the `queryApproval` and `lastActiveThreadId` settings which persist via the existing `commandClient.updateAppSetting` mechanism. The `run_query` tool executor gains a direct query-execution path so the agent receives results. File deduplication uses a `source: "agent"` tag in `ViewState.data` to distinguish agent-created tabs from user tabs.

**Tech Stack:** SvelteKit + Svelte 5 runes, TypeScript, Tauri IPC (`invoke`), Tauri Icons (`@tabler/icons-svelte`), Bun (harness)

---

## File Map

| File | Change |
|------|--------|
| `src/lib/agent/tool-executor.ts` | Remove spawn_subagent; update run_query to execute and return results; add source tag to write_file; add list_files case; add executeQuery to ToolContext |
| `src/lib/agent/tools.ts` | Remove spawn_subagent from tool table; add list_files; update plan mode prompt re: query approval |
| `src/lib/stores/settings.svelte.ts` | Add queryApproval and lastActiveThreadId fields |
| `src/lib/components/agent/AgentComposer.svelte` | Add Shield toggle alongside Plan/Model/Effort; accept queryApproval prop |
| `src/lib/components/agent/AgentPanel.svelte` | Remove runChildAgent/spawnSubagent; update approval logic to use settingsStore.queryApproval; provide executeQuery in getToolContext; pass queryApproval to composer; persist/restore lastActiveThreadId in startThread/initForConnection |
| `src/lib/components/agent/ToolCallCard.svelte` | Replace done/error dot with IconCircleCheck/IconCircleX |
| `src/lib/components/agent/ThreadPicker.svelte` | Inline delete confirmation; move New Session to bottom |
| `packages/harness/src/index.ts` | Remove 300s spawn_subagent timeout special case |

---

## Task 1: Remove spawn_subagent

**Files:**
- Modify: `src/lib/agent/tool-executor.ts`
- Modify: `src/lib/agent/tools.ts`
- Modify: `src/lib/components/agent/AgentPanel.svelte`
- Modify: `packages/harness/src/index.ts`

- [ ] **Step 1: Remove spawnSubagent from ToolContext and tool-executor.ts**

In `src/lib/agent/tool-executor.ts`, remove `spawnSubagent?` from the interface and delete the `spawn_subagent` case:

```ts
export interface ToolContext {
    port: number;
    sessionId: string;
    connectionId: string;
    database: string;
    schema: string;
    openInEditor: (sql: string, title: string, autoRun?: boolean) => void;
    executeQuery: (sql: string) => Promise<{ columns: string[]; rows: unknown[]; totalRows: number }>;
}
```

Delete lines 317–328 (the `spawn_subagent` case in `executeTool`):
```ts
// DELETE THIS ENTIRE CASE:
case "spawn_subagent": {
    const goal = inp.goal as string | undefined;
    const model = inp.model as string | undefined;
    if (!goal) return { error: "spawn_subagent requires a 'goal' field" };
    if (!ctx.spawnSubagent) return { error: "spawn_subagent not available in this context" };
    try {
        const result = await ctx.spawnSubagent(goal, model);
        return { result };
    } catch (e) {
        return { error: String(e) };
    }
}
```

Note: `executeQuery` is added here so the interface is complete — the implementation goes in Task 4.

- [ ] **Step 2: Remove spawn_subagent from tools.ts**

In `src/lib/agent/tools.ts`, in `buildToolInstructions`, delete the `spawn_subagent` row from the tool table:
```
| \`spawn_subagent\` | \`goal\`, \`model?\` | Delegate a subtask to a child agent; returns the child's output when complete |
```

- [ ] **Step 3: Remove runChildAgent and spawnSubagent wiring from AgentPanel.svelte**

Delete the entire `runChildAgent` function (lines 159–210) from `src/lib/components/agent/AgentPanel.svelte`.

In `getToolContext()`, remove the `spawnSubagent` line:
```ts
// BEFORE:
return {
    port: sess.port,
    sessionId: sess.sessionId,
    connectionId: conn.id,
    database: schemaStore.selectedDatabase ?? "",
    schema: schemaStore.activeSchema ?? "public",
    openInEditor: (sql: string, _title: string, autoRun = false) => {
        handleRunQuery(sql, autoRun);
    },
    spawnSubagent: (goal: string, model?: string) => runChildAgent(goal, model),
};

// AFTER (spawnSubagent line removed, executeQuery placeholder added — filled in Task 4):
return {
    port: sess.port,
    sessionId: sess.sessionId,
    connectionId: conn.id,
    database: schemaStore.selectedDatabase ?? "",
    schema: schemaStore.activeSchema ?? "public",
    openInEditor: (sql: string, _title: string, _autoRun = false) => {
        handleRunQuery(sql);
    },
    executeQuery: async (_sql: string) => ({ columns: [], rows: [], totalRows: 0 }), // stub — replaced in Task 4
};
```

- [ ] **Step 4: Remove 300s timeout from harness**

In `packages/harness/src/index.ts`, replace:
```ts
// spawn_subagent runs a full child session — give it 5 minutes
const timeoutMs = toolName === "spawn_subagent" ? 300_000 : 30_000;
```
With:
```ts
const timeoutMs = 30_000;
```

- [ ] **Step 5: Verify TypeScript compiles**

```bash
cd /Users/__deesh_reddy__/projects/personal_git/rust_builds/tables
pnpm check
```
Expected: No errors related to spawn_subagent or spawnSubagent.

- [ ] **Step 6: Commit**

```bash
git add src/lib/agent/tool-executor.ts src/lib/agent/tools.ts src/lib/components/agent/AgentPanel.svelte packages/harness/src/index.ts
git commit -m "feat(agent): remove spawn_subagent tool and child session complexity"
```

---

## Task 2: Tool Status Icons — CircleCheck and CircleX

**Files:**
- Modify: `src/lib/components/agent/ToolCallCard.svelte`

- [ ] **Step 1: Add icon imports**

In `src/lib/components/agent/ToolCallCard.svelte`, add two imports after the existing icon imports:

```ts
import IconCircleCheck from "@tabler/icons-svelte/icons/circle-check";
import IconCircleX from "@tabler/icons-svelte/icons/circle-x";
```

- [ ] **Step 2: Update the status indicator in the left rail**

Find this block in the template (around line 269–271):
```svelte
{#if toolCall.status === "running"}
    <IconLoader2 size={11} class="mt-0.5 shrink-0 animate-spin text-accent" />
{:else}
    <div class="mt-0.5 h-2 w-2 shrink-0 rounded-full {dotClass}"></div>
{/if}
```

Replace with:
```svelte
{#if toolCall.status === "running"}
    <IconLoader2 size={11} class="mt-0.5 shrink-0 animate-spin text-accent" />
{:else if toolCall.status === "done"}
    <IconCircleCheck size={11} class="mt-0.5 shrink-0 text-foreground/40" />
{:else if toolCall.status === "error"}
    <IconCircleX size={11} class="mt-0.5 shrink-0 text-destructive/70" />
{:else}
    <!-- awaiting: keep the amber dot -->
    <div class="mt-0.5 h-2 w-2 shrink-0 rounded-full {dotClass}"></div>
{/if}
```

- [ ] **Step 3: Remove dotClass — it is now only used for awaiting**

The `dotClass` derived was:
```ts
const dotClass = $derived.by((): string => {
    switch (toolCall.status) {
        case "running":
            return "bg-accent animate-pulse";
        case "done":
            return "bg-foreground/40";
        case "error":
            return "bg-destructive/70";
        case "awaiting":
            return "bg-amber-400";
        default:
            return "bg-muted-foreground/30";
    }
});
```

Simplify it — only `awaiting` uses the dot now:
```ts
const dotClass = "bg-amber-400";
```

- [ ] **Step 4: Verify visually**

Run `pnpm dev` and trigger a tool call in the agent panel. Confirm:
- Running: spinning loader
- Done: circle with checkmark, muted color
- Error: circle with X, red
- Awaiting: amber dot (unchanged)

- [ ] **Step 5: Commit**

```bash
git add src/lib/components/agent/ToolCallCard.svelte
git commit -m "feat(agent): use CircleCheck/CircleX icons for tool done/error status"
```

---

## Task 3: Query Approval Setting — Shield Toggle

**Files:**
- Modify: `src/lib/stores/settings.svelte.ts`
- Modify: `src/lib/components/agent/AgentComposer.svelte`
- Modify: `src/lib/components/agent/AgentPanel.svelte`

- [ ] **Step 1: Add queryApproval and lastActiveThreadId to Settings interface**

In `src/lib/stores/settings.svelte.ts`, update the `Settings` interface:

```ts
export interface Settings {
    editorFontFamily: string;
    editorFontSize: number;
    sidebarLeftVisible: boolean;
    sidebarLeftRatio: number;
    sidebarRightVisible: boolean;
    sidebarRightRatio: number;
    sidebarBottomVisible: boolean;
    sidebarBottomRatio: number;
    selectedDatabase: string | null;
    expandedNodes: Record<string, string[]>;
    activeRightPanel: string | null;
    editorShowAllRunButtons: boolean;
    aiModel: string;
    aiEffort: "auto" | "low" | "medium" | "high" | "max";
    queryApproval: "auto" | "ask";
    lastActiveThreadId: string | null;
}
```

Update `DEFAULT_SETTINGS`:
```ts
const DEFAULT_SETTINGS: Settings = {
    // ... existing fields ...
    aiModel: "claude-sonnet-4-6",
    aiEffort: "auto",
    queryApproval: "ask",
    lastActiveThreadId: null,
};
```

- [ ] **Step 2: Add getter/setter for queryApproval**

Inside the `return { ... }` object of `createSettingsStore()`, after the `aiEffort` getter/setter, add:

```ts
get queryApproval(): "auto" | "ask" {
    return settings.queryApproval;
},
set queryApproval(v: "auto" | "ask") {
    settings.queryApproval = v;
    commandClient.updateAppSetting("query_approval", v);
},
get lastActiveThreadId(): string | null {
    return settings.lastActiveThreadId;
},
set lastActiveThreadId(v: string | null) {
    settings.lastActiveThreadId = v;
    commandClient.updateAppSetting("last_active_thread_id", v ?? "");
},
```

- [ ] **Step 3: Handle query_approval in settings-changed listener**

Inside the `listen("settings-changed", ...)` callback, in the global key switch block, add:

```ts
case "query_approval":
    settings.queryApproval = (value === "auto" ? "auto" : "ask") as "auto" | "ask";
    return;
case "last_active_thread_id":
    settings.lastActiveThreadId = value || null;
    return;
```

- [ ] **Step 4: Add queryApproval prop to AgentComposer and add Shield toggle**

In `src/lib/components/agent/AgentComposer.svelte`, add the import at the top of the script:

```ts
import IconShieldCheck from "@tabler/icons-svelte/icons/shield-check";
import IconShield from "@tabler/icons-svelte/icons/shield";
```

Update the `Props` interface to add the new prop and callback:

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
}

let { onSend, onStop, onPlanModeToggle, onQueryApprovalToggle, running, disabled, planMode, queryApproval }: Props = $props();
```

In the template bottom bar, add the Shield toggle **after** the Plan toggle button and **before** the Model picker:

```svelte
<!-- Query approval toggle -->
<button
    onclick={onQueryApprovalToggle}
    title={queryApproval === "ask" ? "Queries need your approval before running" : "Queries run automatically"}
    class="flex items-center gap-1 rounded px-1.5 py-1 text-[10.5px] transition-colors {queryApproval === 'ask' ? 'text-amber-400 hover:bg-amber-400/10' : 'text-muted-foreground/50 hover:bg-foreground/5 hover:text-muted-foreground'}"
>
    {#if queryApproval === "ask"}
        <IconShieldCheck size={11} />
    {:else}
        <IconShield size={11} />
    {/if}
</button>
```

- [ ] **Step 5: Wire queryApproval into AgentPanel**

In `src/lib/components/agent/AgentPanel.svelte`:

1. Remove the `const APPROVAL_REQUIRED = new Set(["run_query"])` line (keep the Set value inline below).

2. In `handleEvent` for `tool.started`, update the approval check:
```ts
// BEFORE:
const needsApproval = planMode && APPROVAL_REQUIRED.has(event.toolName) && !!ctx;

// AFTER:
const APPROVAL_REQUIRED = new Set(["run_query"]);
const needsApproval = settingsStore.queryApproval === "ask" && APPROVAL_REQUIRED.has(event.toolName) && !!ctx;
```

3. Update the `<AgentComposer>` usage in the template to pass the new props:
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
/>
```

- [ ] **Step 6: Update plan mode prompt to not hardcode approval**

In `src/lib/agent/tools.ts`, in `buildSystemPrompt`, update the `planSection` text to say "may require" instead of hardcoding approval:

```ts
4. **run_query** — executes SQL. Depending on the user's approval setting, it may require explicit approval before running. If rejected, the tool returns an error — acknowledge it, then revise in write_file and try again.
```

Replace lines 28–29:
```ts
// BEFORE:
4. **run_query requires user approval** — when you call run_query, the user is shown the SQL and must explicitly approve before it executes. Do not assume approval.
5. If a run_query is rejected, the tool returns an error. Acknowledge it, then either revise the query in write_file and try again, or ask the user what to change.

// AFTER:
4. **run_query** — executes SQL against the live database. The user may require explicit approval before it runs. If the tool returns a rejection error, revise the query in write_file and try again, or ask the user what to change.
```

- [ ] **Step 7: Verify TypeScript compiles**

```bash
pnpm check
```
Expected: No errors.

- [ ] **Step 8: Commit**

```bash
git add src/lib/stores/settings.svelte.ts src/lib/components/agent/AgentComposer.svelte src/lib/components/agent/AgentPanel.svelte src/lib/agent/tools.ts
git commit -m "feat(agent): add query approval shield toggle, decouple from plan mode"
```

---

## Task 4: Query Results Returned to Agent

**Files:**
- Modify: `src/lib/agent/tool-executor.ts`
- Modify: `src/lib/components/agent/AgentPanel.svelte`

- [ ] **Step 1: Replace the executeQuery stub in AgentPanel.getToolContext()**

In `src/lib/components/agent/AgentPanel.svelte`, replace the stub added in Task 1 with the real implementation.

In `getToolContext()`, replace:
```ts
executeQuery: async (_sql: string) => ({ columns: [], rows: [], totalRows: 0 }),
```
With:
```ts
executeQuery: async (sql: string) => {
    const result = await invoke<any>("execute_query", {
        connectionId: conn.id,
        sessionId: "agent",
        database: schemaStore.selectedDatabase ?? "",
        schema: schemaStore.activeSchema ?? "public",
        query: sql,
        component: "agent",
        limit: 50,
        offset: 0,
    });
    return {
        columns: (result?.columns ?? []).map((c: any) => c.name ?? c) as string[],
        rows: result?.rows ?? [],
        totalRows: result?.total ?? result?.rows?.length ?? 0,
    };
},
```

- [ ] **Step 2: Update run_query case in tool-executor.ts to return results**

In `src/lib/agent/tool-executor.ts`, replace:
```ts
case "run_query": {
    const sql = inp.sql as string;
    ctx.openInEditor(sql, "Query", true);
    return { opened: true, message: "Query opened in editor and running" };
}
```

With:
```ts
case "run_query": {
    const sql = inp.sql as string;
    ctx.openInEditor(sql, "Query");
    try {
        const { columns, rows, totalRows } = await ctx.executeQuery(sql);
        const truncated = totalRows > 50;
        return {
            columns,
            rows,
            rowCount: rows.length,
            totalRows,
            truncated,
            note: truncated ? `Showing first 50 of ${totalRows} rows. Use read_file on a write_file result or narrow the query for more.` : undefined,
        };
    } catch (e) {
        return { error: String(e) };
    }
}
```

- [ ] **Step 3: Update run_query in tools.ts description**

In `src/lib/agent/tools.ts`, in `buildToolInstructions`, update the run_query row in the tool table:

```ts
| \`run_query\` | \`sql\` | Execute SQL against the live database; returns columns, rows (up to 50), and totalRows. May require user approval. |
```

- [ ] **Step 4: Verify TypeScript compiles**

```bash
pnpm check
```
Expected: No errors.

- [ ] **Step 5: Commit**

```bash
git add src/lib/agent/tool-executor.ts src/lib/components/agent/AgentPanel.svelte src/lib/agent/tools.ts
git commit -m "feat(agent): run_query now returns actual query results to the agent"
```

---

## Task 5: File Deduplication — Agent-Scoped Name Upsert + list_files

**Files:**
- Modify: `src/lib/agent/tool-executor.ts`
- Modify: `src/lib/agent/tools.ts`

- [ ] **Step 1: Update write_file to tag agent files and upsert by agent-scoped name**

In `src/lib/agent/tool-executor.ts`, replace the `write_file` case:

```ts
case "write_file": {
    const { fileId: targetId, fileName, content } = inp as { fileId?: string; fileName: string; content: string };
    const session = windowState.activeSession;
    if (!session) return { error: "no active session" };

    // 1. Exact fileId match — highest priority
    if (targetId) {
        const byId = session.views.find((v) => v.id === targetId);
        if (byId) {
            byId.data = byId.data ?? {};
            (byId.data as Record<string, unknown>).content = content;
            byId.streamingContent = undefined;
            if (fileName && byId.title !== fileName) byId.title = fileName;
            return { ok: true, action: "updated", fileId: byId.id, fileName: byId.title, lines: content.split("\n").length };
        }
    }

    // 2. Agent-scoped name match — only update agent-created files
    const agentMatch = session.views.find(
        (v) => v.type === "editor" && v.title === fileName && (v.data as Record<string, unknown>)?.source === "agent"
    );
    if (agentMatch) {
        agentMatch.data = agentMatch.data ?? {};
        (agentMatch.data as Record<string, unknown>).content = content;
        agentMatch.streamingContent = undefined;
        return { ok: true, action: "updated", fileId: agentMatch.id, fileName: agentMatch.title, lines: content.split("\n").length };
    }

    // 3. Create new agent-tagged file
    const newId = session.openView("editor", fileName, { content, source: "agent" });
    return { ok: true, action: "created", fileId: newId, fileName, lines: content.split("\n").length };
}
```

- [ ] **Step 2: Add list_files case**

In `src/lib/agent/tool-executor.ts`, add a new case before the `default:` case:

```ts
case "list_files": {
    const session = windowState.activeSession;
    if (!session) return [];
    return session.views
        .filter((v) => v.type === "editor" && (v.data as Record<string, unknown>)?.source === "agent")
        .map((v) => ({ fileId: v.id, fileName: v.title }));
}
```

- [ ] **Step 3: Add list_files to the tool table in tools.ts**

In `src/lib/agent/tools.ts`, in `buildToolInstructions`, add `list_files` to the tool table after `write_file`:

```ts
| \`list_files\` | _(none)_ | List all files you have created in this session; returns [{fileId, fileName}] for reuse |
```

- [ ] **Step 4: Add list_files guidance to the File Writing section**

In `buildToolInstructions`, after the existing File Writing section, add:

```ts
## File Management

Use \`list_files\` at the start of a task to check what files you have already created. When updating an existing file, always prefer the fileId returned from a previous write_file — this avoids creating duplicates. If you have lost the fileId, call list_files to recover it by name.

Do NOT create a new file for each iteration or revision. Update the existing one.
`;
```

(Insert this text just before the closing backtick of the template literal in `buildToolInstructions`.)

- [ ] **Step 5: Verify TypeScript compiles**

```bash
pnpm check
```
Expected: No errors.

- [ ] **Step 6: Commit**

```bash
git add src/lib/agent/tool-executor.ts src/lib/agent/tools.ts
git commit -m "feat(agent): agent-scoped file upsert and list_files tool to prevent duplicate tabs"
```

---

## Task 6: Thread Persistence on Restart

**Files:**
- Modify: `src/lib/components/agent/AgentPanel.svelte`

(Settings fields were added in Task 3.)

- [ ] **Step 1: Save lastActiveThreadId in startThread**

In `src/lib/components/agent/AgentPanel.svelte`, at the top of `startThread()`, after `threadsStore.setActive(thread.id)`:

```ts
settingsStore.lastActiveThreadId = thread.id;
```

The full start of `startThread` should be:
```ts
async function startThread(thread: AgentThread) {
    if (abortController) abortController.abort();
    sessionReady = false;
    sessionError = null;
    streamingMsgId = null;
    titleSet = false;
    turnStartedAt = null;

    const conn = schemaStore.activeConnection;
    if (!conn) return;

    threadsStore.setActive(thread.id);
    settingsStore.lastActiveThreadId = thread.id;  // ← new line
    await agentStore.loadThread(thread.id);
    // ... rest unchanged
```

- [ ] **Step 2: Restore lastActiveThreadId in initForConnection**

In `src/lib/components/agent/AgentPanel.svelte`, update `initForConnection`:

```ts
async function initForConnection() {
    const conn = schemaStore.activeConnection;
    if (!conn) return;

    await threadsStore.load(conn.id, schemaStore.selectedDatabase);

    if (threadsStore.threads.length === 0) {
        await createAndStartThread();
    } else {
        const toResume =
            threadsStore.threads.find((t) => t.id === settingsStore.lastActiveThreadId)
            ?? threadsStore.threads[0];
        await startThread(toResume);
    }
}
```

- [ ] **Step 3: Verify TypeScript compiles**

```bash
pnpm check
```
Expected: No errors.

- [ ] **Step 4: Manual test**

Run `pnpm tauri dev`. Switch to a non-first thread in the agent panel. Restart the app. Verify the same thread is active on reload, not the most recent one.

- [ ] **Step 5: Commit**

```bash
git add src/lib/components/agent/AgentPanel.svelte
git commit -m "feat(agent): restore last active thread on app restart"
```

---

## Task 7: Delete Confirmation + New Session Button

**Files:**
- Modify: `src/lib/components/agent/ThreadPicker.svelte`

- [ ] **Step 1: Rewrite ThreadPicker with inline confirm state and New Session at bottom**

Replace the entire content of `src/lib/components/agent/ThreadPicker.svelte` with:

```svelte
<!-- src/lib/components/agent/ThreadPicker.svelte -->
<script lang="ts">
    import * as Menu from "$lib/components/ui/dropdown-menu";
    import { threadsStore, type AgentThread } from "$lib/stores/threads.svelte";
    import IconChevronDown from "@tabler/icons-svelte/icons/chevron-down";
    import IconPlus from "@tabler/icons-svelte/icons/plus";
    import IconTrash from "@tabler/icons-svelte/icons/trash";

    interface Props {
        onNewThread: () => void;
        onSelectThread: (thread: AgentThread) => void;
    }

    let { onNewThread, onSelectThread }: Props = $props();

    const activeThread = $derived(threadsStore.activeThread);
    const activeThreadId = $derived(threadsStore.activeThreadId);
    const threads = $derived(threadsStore.threads);
    const title = $derived(activeThread?.title ?? "New session");

    let confirmingId = $state<string | null>(null);
</script>

<Menu.Root>
    <Menu.Trigger>
        <button
            class="flex max-w-[160px] items-center gap-1 truncate rounded px-1.5 py-1 text-[11px] font-medium text-foreground/70 transition-colors hover:bg-foreground/5 hover:text-foreground"
            title={title}
        >
            <span class="truncate">{title}</span>
            <IconChevronDown size={9} class="shrink-0 opacity-50" />
        </button>
    </Menu.Trigger>
    <Menu.Content
        class="w-56 border border-border bg-background shadow-md p-1"
        align="start"
        side="bottom"
    >
        {#each threads as thread (thread.id)}
            <Menu.Item
                class="group flex items-center justify-between gap-2 rounded px-2 py-1.5 text-[11px] cursor-pointer {thread.id === activeThreadId ? 'bg-accent/10 text-foreground' : 'text-foreground/70 hover:bg-foreground/5 hover:text-foreground'}"
                onclick={() => { if (confirmingId !== thread.id) onSelectThread(thread); }}
            >
                {#if confirmingId === thread.id}
                    <!-- Inline confirm state -->
                    <span class="text-[10.5px] text-foreground/60">Delete this session?</span>
                    <div class="flex shrink-0 items-center gap-1">
                        <button
                            onclick={(e) => { e.stopPropagation(); confirmingId = null; }}
                            class="rounded px-1.5 py-0.5 text-[10px] text-muted-foreground hover:bg-foreground/5"
                        >Cancel</button>
                        <button
                            onclick={(e) => { e.stopPropagation(); threadsStore.deleteThread(thread.id); confirmingId = null; }}
                            class="rounded px-1.5 py-0.5 text-[10px] text-destructive hover:bg-destructive/10"
                        >Delete</button>
                    </div>
                {:else}
                    <span class="truncate">{thread.title}</span>
                    <button
                        class="shrink-0 opacity-0 group-hover:opacity-60 hover:!opacity-100 text-destructive transition-opacity"
                        title="Delete session"
                        onclick={(e) => { e.stopPropagation(); confirmingId = thread.id; }}
                    >
                        <IconTrash size={10} />
                    </button>
                {/if}
            </Menu.Item>
        {/each}

        {#if threads.length > 0}
            <Menu.Separator class="my-1 border-t border-border/50" />
        {/if}

        <!-- New session at bottom -->
        <Menu.Item
            class="flex items-center gap-2 px-2 py-1.5 text-[11px] rounded cursor-pointer text-accent/70 hover:bg-accent/10 hover:text-accent"
            onclick={onNewThread}
        >
            <IconPlus size={11} />
            <span>+ New session</span>
        </Menu.Item>
    </Menu.Content>
</Menu.Root>
```

- [ ] **Step 2: Verify TypeScript compiles**

```bash
pnpm check
```
Expected: No errors.

- [ ] **Step 3: Manual test**

Run `pnpm tauri dev`. Open the thread picker:
- Hover a thread → trash icon appears
- Click trash → row shows "Delete this session?" + Cancel + Delete inline
- Click Cancel → row returns to normal, no deletion
- Click Delete → thread is deleted
- "+ New session" appears at the bottom, separated by a divider

- [ ] **Step 4: Commit**

```bash
git add src/lib/components/agent/ThreadPicker.svelte
git commit -m "feat(agent): inline delete confirmation and New Session button at bottom of thread picker"
```
