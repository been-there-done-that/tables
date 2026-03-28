# Spec: Agent Panel UX Improvements

**Date:** 2026-03-29
**Branch:** ai
**Status:** Approved

---

## 1. Tool Status Icons

Replace the colored dot for `done` and `error` states in `ToolCallCard.svelte` with icon badges that carry semantic weight:

| Status | Visual | Color |
|--------|--------|-------|
| `running` | `IconLoader2` spinner (existing) | `text-accent` |
| `awaiting` | Amber dot (existing) | `bg-amber-400` |
| `done` | `IconCircleCheck` (circle + checkmark) | `text-foreground/50` |
| `error` | `IconCircleX` (circle + X) | `text-destructive/70` |

Size: 11px for all icons, matching the existing spinner. The filled-circle frame gives depth; the inner symbol communicates result at a glance. The `awaiting` state keeps the dot idiom because it represents "waiting on you" — not a terminal outcome.

---

## 2. Query Results Returned to Agent

### Problem

`run_query` in `tool-executor.ts` currently calls `ctx.openInEditor(sql, ..., autoRun=true)` and immediately returns `{opened: true}`. The agent never receives the query results and cannot reason about the data.

### Solution

Add `executeQuery(sql: string): Promise<QueryResult>` to `ToolContext`. The `run_query` tool executor:

1. Runs the SQL using the existing query execution path
2. Awaits the result
3. Returns to the agent:
   - Column names
   - First 50 rows
   - Total row count
   - Truncation note if `totalRows > 50`: `"(showing 50 of N rows)"`
4. On SQL error: returns the error message as tool output with `status: "error"`

The query still opens visibly in the editor tab. The user can inspect the full result grid; the agent sees the first 50 rows. No temp file needed at this stage — revisit if context bloat becomes an issue.

---

## 3. File Deduplication — Agent-Scoped Name Upsert

### Problem

The agent receives a `fileId` on the first `write_file` call. After several tool calls, it "forgets" the ID and calls `write_file` with the same `fileName` again, creating a duplicate tab. Option B (deduplicating by name unconditionally) risks silently overwriting user-created files.

### Solution

Tag each editor view with `source: "agent" | "user"`. Existing tabs default to `"user"`. New tabs created by the agent are tagged `source: "agent"`.

**`write_file` resolution order:**
1. If `fileId` is present → update that exact view (existing behavior)
2. If only `fileName` is present → search open views for an agent-created tab (`source === "agent"`) with that exact `fileName`
   - Found → update its content (no duplicate created)
   - Not found → create new tab tagged `source: "agent"`
3. User tabs (`source === "user"`) are never auto-matched by name — protected from silent overwrites

**New `list_files` tool:**
```
list_files() → [{fileId, fileName}]
```
Returns all open agent-created tabs. The agent calls this at the start of a task to recover `fileId` values if context was lost. Added to the tool table in `tools.ts` and the tool executor.

**System prompt addition (in `buildToolInstructions`):**
> "Use `list_files` at the start of a task to check what files you've already created. Reuse existing files for related work — do not create a new file for each query iteration or revision."

---

## 4. Remove spawn_subagent

Remove all sub-agent orchestration code. The main agent handles multi-step tasks natively. The 5-minute child session timeout is not worth the complexity at this stage.

**Files to change:**

| File | Change |
|------|--------|
| `tools.ts` | Remove `spawn_subagent` from tool table + system prompt instructions |
| `tool-executor.ts` | Remove `spawn_subagent` case; remove `spawnSubagent?` from `ToolContext` |
| `AgentPanel.svelte` | Remove `runChildAgent()`, `pendingChildSessions`, and `spawnSubagent` wiring in `getToolContext()` |
| `packages/harness/src/index.ts` | Remove the `300_000` special-case timeout for `spawn_subagent`; uniform 30s for all tools |

---

## 5. Query Run Permission — Shield Toggle

### Setting

Add `queryApproval: "auto" | "ask"` to `settingsStore`, defaulting to `"ask"`.

### UI

In `AgentComposer.svelte`, add a **Shield icon toggle** in the bottom bar alongside Plan / Model / Effort:

- **`"ask"` mode:** `IconShieldCheck` — amber/warning color, title "Queries need approval"
- **`"auto"` mode:** `IconShield` — muted color, title "Queries run automatically"

Single click cycles between modes (same pattern as the Plan toggle — no dropdown, only two states).

### Approval behavior

When `queryApproval === "ask"`:
- `run_query` emits `status: "awaiting"` regardless of `planMode`
- The existing amber approval panel in `ToolCallCard` is shown (SQL preview + Approve / Reject)
- Reject allows the user to type feedback before dismissing
- Approved → query executes and results return to agent

When `queryApproval === "auto"`:
- `run_query` executes immediately, no approval panel shown

**Decouples query approval from `planMode`** — plan mode remains about plan XML generation and step tracking, not about gate-keeping tool calls. The `APPROVAL_REQUIRED` set in `AgentPanel.svelte` is replaced by reading `settingsStore.queryApproval`.

---

## 6. Thread Persistence on Restart

### Problem

`initForConnection` always calls `startThread(threadsStore.threads[0])` — the most recently created thread — ignoring which thread was active before the app closed.

### Solution

Add `lastActiveThreadId: string | null` to `settingsStore` (persisted to disk via existing mechanism).

In `initForConnection`:
```ts
const toResume = threadsStore.threads.find(t => t.id === settingsStore.lastActiveThreadId)
    ?? threadsStore.threads[0];
await startThread(toResume);
```

Call `settingsStore.lastActiveThreadId = thread.id` at the top of `startThread()`. The ID is connection-agnostic — if the stored ID doesn't exist in the loaded thread list (different connection), it falls back gracefully to `threads[0]`.

---

## 7. Delete Confirmation + New Thread Button

### Delete confirmation

In `ThreadPicker.svelte`, replace the direct `deleteThread()` call with a two-step inline confirmation:

1. First trash icon click → row enters "confirm" state: shows `"Delete?"` label + Cancel + Delete buttons inline in the same row
2. Cancel → row returns to normal
3. Delete → calls `threadsStore.deleteThread(thread.id)`

No modal. The confirmation lives in the list row itself. Uses a local `$state<string | null>(null)` (`confirmingId`) to track which row is in confirm state.

### New Session button

Move the "New chat" action to the **bottom** of the thread list, separated by a divider:

```
[thread 1]
[thread 2]
[thread 3]
────────────
[+ New session]
```

Label: **"+ New session"**. Positioned at the bottom makes the list read as history-first, action-second.

---

## Implementation Order

1. Remove `spawn_subagent` (cleans up before adding features)
2. Tool status icons (isolated, visual only)
3. Query run permission toggle (`settingsStore` + composer UI + `APPROVAL_REQUIRED` refactor)
4. Query results returned to agent (`ToolContext.executeQuery` + tool executor change)
5. File deduplication (`source` tag + name-upsert logic + `list_files` tool)
6. Thread persistence on restart (`settingsStore.lastActiveThreadId`)
7. Delete confirmation + New session button (`ThreadPicker` UI changes)
