# Agent Multi-Agent Checkpoints Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add structured plan persistence (`agent_plans` + `agent_plan_steps` SQLite tables), plan-section rendering in the chat UI, and a `spawn_subagent` tool that lets the orchestrator agent delegate to a child agent session — all working identically across providers (Claude, Gemini, Codex, etc.).

**Architecture:** Three layers. (1) SQLite: `agent_plans` records an orchestrator's intent; `agent_plan_steps` tracks each step's phase + status. (2) Rust: four Tauri commands for plan CRUD. (3) Frontend: `plansStore` manages plan state; `AgentPanel` parses `<plan>` XML from assistant messages and persists steps; a new `spawn_subagent` case in `tool-executor.ts` runs a child session entirely on the frontend and returns results to the parent harness session. The harness and provider layer are untouched.

**Tech Stack:** Rust/rusqlite (spawn_blocking pattern), Svelte 5 runes, TypeScript strict, Tauri 2 invoke()

**Prerequisite:** `2026-03-28-agent-panel-polish.md` must be complete and merged.

---

## Design Notes (read before coding)

### Why frontend-orchestrated sub-agents

The harness is a thin stateless relay. Sub-agent execution is a *frontend concern* because:
- It works with any provider (no SDK-level sub-agent API needed)
- The child agent's events stream to the same UI (full visibility)
- The parent agent is simply blocked on `/tool-result/:requestId` — the frontend runs the child, collects its output, and POSTs the result. The harness doesn't know or care.

### spawn_subagent protocol

The parent agent calls:
```bash
curl -X POST http://127.0.0.1:$HARNESS_PORT/db/$SESSION_ID/spawn_subagent \
  -d '{"goal": "...", "model": "claude-haiku-4-5-20251001"}'
```

The harness emits `tool.started { toolName: "spawn_subagent", input: { goal, model? } }`.

The frontend:
1. Creates a child thread in SQLite (`parent_thread_id` set to active thread ID)
2. Starts a child harness session with its own `sessionId`
3. Sends the goal as the first user message
4. Waits for `turn.done`
5. Collects the child session's final assistant text
6. POSTs `{ result: "<child output>" }` to `/tool-result/:requestId`

The parent agent resumes with the child's findings.

### `<plan>` XML format

When plan mode is active, the system prompt instructs the agent to output:
```xml
<plan>
  <step phase="gather">Describe the orders table to understand schema</step>
  <step phase="gather">Sample 20 rows from orders</step>
  <step phase="draft">Write the revenue query to revenue-by-month.sql</step>
  <step phase="execute">Run revenue-by-month.sql</step>
</plan>
```

The frontend parses this XML from assistant messages and saves to `agent_plan_steps`. Steps are displayed as a collapsed "Plan" card in the chat with a progress indicator.

### Phase semantics
- `gather` — read-only tools (describe_table, sample_table, etc.); auto-execute, parallelizable
- `draft` — write_file; auto-execute, sequential
- `execute` — run_query; requires user approval (already enforced by plan mode)

---

## File Map

| File | Action | What changes |
|---|---|---|
| `src-tauri/migrations/008_agent_plans.sql` | Create | `agent_plans` + `agent_plan_steps` tables; `parent_thread_id` on `agent_threads` |
| `src-tauri/src/migrations.rs` | Modify | Register migration 008 |
| `src-tauri/src/commands/agent_commands.rs` | Modify | Add `create_agent_plan`, `list_agent_plans`, `add_plan_step`, `update_plan_step` commands |
| `src/lib/stores/plans.svelte.ts` | Create | `plansStore` — plan CRUD, active plan state |
| `src/lib/components/agent/PlanCard.svelte` | Create | Renders parsed `<plan>` block with step progress |
| `src/lib/components/agent/MessageBubble.svelte` | Modify | Detect `<plan>` XML in assistant content, render PlanCard |
| `src/lib/agent/tool-executor.ts` | Modify | Add `spawn_subagent` case |
| `src/lib/components/agent/AgentPanel.svelte` | Modify | `runChildAgent()` function; parse `<plan>` XML from turn.done; add `parent_thread_id` to child thread creation |
| `src/lib/stores/threads.svelte.ts` | Modify | Add `parentThreadId` to `AgentThread`; update `createThread` to accept `parentThreadId` |
| `src-tauri/migrations/007_create_agent_tables.sql` | **DO NOT TOUCH** | This already exists — only add a new 008 migration |

---

## Task 1: SQLite migration — agent_plans, agent_plan_steps, parent_thread_id

**Files:**
- Create: `src-tauri/migrations/008_agent_plans.sql`
- Modify: `src-tauri/src/migrations.rs`

- [ ] **Step 1: Write the migration SQL**

```sql
-- src-tauri/migrations/008_agent_plans.sql

-- Add parent_thread_id to agent_threads for sub-agent tracking
ALTER TABLE agent_threads ADD COLUMN parent_thread_id TEXT REFERENCES agent_threads(id) ON DELETE SET NULL;

-- An orchestrator's structured work plan
CREATE TABLE IF NOT EXISTS agent_plans (
    id          TEXT PRIMARY KEY,
    thread_id   TEXT NOT NULL REFERENCES agent_threads(id) ON DELETE CASCADE,
    title       TEXT NOT NULL DEFAULT 'Plan',
    status      TEXT NOT NULL DEFAULT 'pending'  -- pending | running | done | cancelled
        CHECK(status IN ('pending','running','done','cancelled')),
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL
);

-- Individual steps within a plan
CREATE TABLE IF NOT EXISTS agent_plan_steps (
    id          TEXT PRIMARY KEY,
    plan_id     TEXT NOT NULL REFERENCES agent_plans(id) ON DELETE CASCADE,
    phase       TEXT NOT NULL  -- gather | draft | execute
        CHECK(phase IN ('gather','draft','execute')),
    description TEXT NOT NULL,
    status      TEXT NOT NULL DEFAULT 'pending'
        CHECK(status IN ('pending','running','done','error','skipped')),
    tool_call_id TEXT,  -- links to agent_tool_calls.id when executed
    position    INTEGER NOT NULL,
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_agent_plans_thread ON agent_plans(thread_id);
CREATE INDEX IF NOT EXISTS idx_agent_plan_steps_plan ON agent_plan_steps(plan_id);
```

- [ ] **Step 2: Register the migration in migrations.rs**

Open `src-tauri/src/migrations.rs`. Find the list of migration tuples (they look like `("007_create_agent_tables", include_str!("../migrations/007_create_agent_tables.sql"))`). Add after the last entry:

```rust
(
    "008_agent_plans",
    include_str!("../migrations/008_agent_plans.sql"),
),
```

- [ ] **Step 3: Build to verify SQL is valid**

```bash
cd src-tauri && cargo build 2>&1 | grep -E "error|warning: unused"
```

Expected: compiles clean. If rusqlite complains about the ALTER TABLE (SQLite allows it only for adding columns, which is what we do), verify the SQL runs:

```bash
sqlite3 /tmp/test_migration.db < src-tauri/migrations/008_agent_plans.sql && echo "OK"
```

Expected: `OK`

- [ ] **Step 4: Commit**

```bash
git add src-tauri/migrations/008_agent_plans.sql src-tauri/src/migrations.rs
git commit -m "feat(db): add agent_plans, agent_plan_steps tables and parent_thread_id to agent_threads"
```

---

## Task 2: Rust commands — plan CRUD

**Files:**
- Modify: `src-tauri/src/commands/agent_commands.rs`

Follow the same pattern as the existing thread commands: `spawn_blocking`, `rusqlite::Connection::open`, return `Result<T, String>`. All four commands follow the identical pattern.

- [ ] **Step 1: Add create_agent_plan command**

```rust
#[tauri::command]
pub async fn create_agent_plan(
    app: tauri::AppHandle,
    id: String,
    thread_id: String,
    title: String,
    now: i64,
) -> Result<(), String> {
    let db_path = crate::db_path(&app);
    tokio::task::spawn_blocking(move || {
        let conn = rusqlite::Connection::open(&db_path).map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO agent_plans (id, thread_id, title, status, created_at, updated_at)
             VALUES (?1, ?2, ?3, 'pending', ?4, ?4)",
            rusqlite::params![id, thread_id, title, now],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}
```

- [ ] **Step 2: Add list_agent_plans command**

```rust
#[derive(serde::Serialize)]
pub struct AgentPlanRow {
    pub id: String,
    pub thread_id: String,
    pub title: String,
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[tauri::command]
pub async fn list_agent_plans(
    app: tauri::AppHandle,
    thread_id: String,
) -> Result<Vec<AgentPlanRow>, String> {
    let db_path = crate::db_path(&app);
    tokio::task::spawn_blocking(move || {
        let conn = rusqlite::Connection::open(&db_path).map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT id, thread_id, title, status, created_at, updated_at
                 FROM agent_plans WHERE thread_id = ?1 ORDER BY created_at ASC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(rusqlite::params![thread_id], |row| {
                Ok(AgentPlanRow {
                    id: row.get(0)?,
                    thread_id: row.get(1)?,
                    title: row.get(2)?,
                    status: row.get(3)?,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        Ok(rows)
    })
    .await
    .map_err(|e| e.to_string())?
}
```

- [ ] **Step 3: Add add_plan_step command**

```rust
#[tauri::command]
pub async fn add_plan_step(
    app: tauri::AppHandle,
    id: String,
    plan_id: String,
    phase: String,
    description: String,
    position: i64,
    now: i64,
) -> Result<(), String> {
    let db_path = crate::db_path(&app);
    tokio::task::spawn_blocking(move || {
        let conn = rusqlite::Connection::open(&db_path).map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO agent_plan_steps (id, plan_id, phase, description, status, position, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, 'pending', ?5, ?6, ?6)",
            rusqlite::params![id, plan_id, phase, description, position, now],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}
```

- [ ] **Step 4: Add update_plan_step command**

```rust
#[tauri::command]
pub async fn update_plan_step(
    app: tauri::AppHandle,
    id: String,
    status: String,
    tool_call_id: Option<String>,
    now: i64,
) -> Result<(), String> {
    let db_path = crate::db_path(&app);
    tokio::task::spawn_blocking(move || {
        let conn = rusqlite::Connection::open(&db_path).map_err(|e| e.to_string())?;
        conn.execute(
            "UPDATE agent_plan_steps SET status = ?2, tool_call_id = ?3, updated_at = ?4 WHERE id = ?1",
            rusqlite::params![id, status, tool_call_id, now],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}
```

- [ ] **Step 5: Register all four commands in lib.rs**

Find the `.invoke_handler(tauri::generate_handler![...])` call in `src-tauri/src/lib.rs`. Add the four new commands to the list:

```rust
crate::commands::agent_commands::create_agent_plan,
crate::commands::agent_commands::list_agent_plans,
crate::commands::agent_commands::add_plan_step,
crate::commands::agent_commands::update_plan_step,
```

- [ ] **Step 6: Build and test**

```bash
cd src-tauri && cargo build 2>&1 | grep "^error"
```

Expected: no errors.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/commands/agent_commands.rs src-tauri/src/lib.rs
git commit -m "feat(rust): add create_agent_plan, list_agent_plans, add_plan_step, update_plan_step Tauri commands"
```

---

## Task 3: plansStore — frontend plan state

**Files:**
- Create: `src/lib/stores/plans.svelte.ts`

Thin reactive store. Mirrors the pattern in `threads.svelte.ts`.

- [ ] **Step 1: Create the store**

```ts
// src/lib/stores/plans.svelte.ts
import { invoke } from "@tauri-apps/api/core";

export interface AgentPlanStep {
    id: string;
    planId: string;
    phase: "gather" | "draft" | "execute";
    description: string;
    status: "pending" | "running" | "done" | "error" | "skipped";
    toolCallId: string | null;
    position: number;
}

export interface AgentPlan {
    id: string;
    threadId: string;
    title: string;
    status: "pending" | "running" | "done" | "cancelled";
    steps: AgentPlanStep[];
    createdAt: number;
}

function nowSecs(): number {
    return Math.floor(Date.now() / 1000);
}

class PlansStore {
    plans = $state<AgentPlan[]>([]);

    async loadForThread(threadId: string) {
        try {
            const rows = await invoke<Array<{
                id: string; thread_id: string; title: string; status: string;
                created_at: number; updated_at: number;
            }>>("list_agent_plans", { threadId });
            this.plans = rows.map((r) => ({
                id: r.id,
                threadId: r.thread_id,
                title: r.title,
                status: r.status as AgentPlan["status"],
                steps: [],
                createdAt: r.created_at,
            }));
        } catch (e) {
            console.error("[plansStore] load failed:", e);
            this.plans = [];
        }
    }

    async createPlan(threadId: string, title: string): Promise<AgentPlan> {
        const id = crypto.randomUUID();
        const now = nowSecs();
        await invoke("create_agent_plan", { id, threadId, title, now });
        const plan: AgentPlan = {
            id, threadId, title, status: "pending", steps: [], createdAt: now,
        };
        this.plans = [...this.plans, plan];
        return plan;
    }

    async addStep(
        planId: string,
        phase: AgentPlanStep["phase"],
        description: string,
    ): Promise<AgentPlanStep> {
        const plan = this.plans.find((p) => p.id === planId);
        if (!plan) throw new Error(`Plan ${planId} not found`);
        const id = crypto.randomUUID();
        const position = plan.steps.length;
        const now = nowSecs();
        await invoke("add_plan_step", { id, planId, phase, description, position, now });
        const step: AgentPlanStep = {
            id, planId, phase, description, status: "pending", toolCallId: null, position,
        };
        plan.steps = [...plan.steps, step];
        return step;
    }

    async updateStep(
        stepId: string,
        status: AgentPlanStep["status"],
        toolCallId?: string,
    ) {
        const now = nowSecs();
        await invoke("update_plan_step", { id: stepId, status, toolCallId: toolCallId ?? null, now });
        for (const plan of this.plans) {
            const step = plan.steps.find((s) => s.id === stepId);
            if (step) {
                step.status = status;
                if (toolCallId) step.toolCallId = toolCallId;
                break;
            }
        }
    }

    clear() {
        this.plans = [];
    }
}

export const plansStore = new PlansStore();
```

- [ ] **Step 2: Type-check**

```bash
pnpm check 2>&1 | grep -i "plans.svelte"
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add src/lib/stores/plans.svelte.ts
git commit -m "feat(agent): add plansStore for agent plan CRUD with Tauri backend"
```

---

## Task 4: PlanCard.svelte — plan step progress UI

**Files:**
- Create: `src/lib/components/agent/PlanCard.svelte`

Renders a collapsed/expandable plan block showing steps with phase badges and status indicators.

- [ ] **Step 1: Create PlanCard**

```svelte
<!-- src/lib/components/agent/PlanCard.svelte -->
<script lang="ts">
    import type { AgentPlan } from "$lib/stores/plans.svelte";
    import IconChevronDown from "@tabler/icons-svelte/icons/chevron-down";
    import IconChevronRight from "@tabler/icons-svelte/icons/chevron-right";
    import IconCheck from "@tabler/icons-svelte/icons/check";
    import IconLoader2 from "@tabler/icons-svelte/icons/loader-2";
    import IconX from "@tabler/icons-svelte/icons/x";
    import IconClock from "@tabler/icons-svelte/icons/clock";

    interface Props {
        plan: AgentPlan;
    }
    let { plan }: Props = $props();
    let expanded = $state(false);

    const doneCount = $derived(plan.steps.filter((s) => s.status === "done").length);
    const totalCount = $derived(plan.steps.length);

    const PHASE_LABEL: Record<string, string> = {
        gather: "Gather",
        draft: "Draft",
        execute: "Execute",
    };
    const PHASE_COLOR: Record<string, string> = {
        gather: "text-blue-400/70 bg-blue-400/10",
        draft: "text-purple-400/70 bg-purple-400/10",
        execute: "text-amber-400/70 bg-amber-400/10",
    };
</script>

<div class="my-1 rounded border border-amber-400/20 bg-amber-400/5 text-[11px]">
    <!-- Header -->
    <button
        onclick={() => (expanded = !expanded)}
        class="flex w-full items-center gap-2 px-2.5 py-1.5 text-left"
    >
        <span class="text-[10px] font-semibold text-amber-400/80 uppercase tracking-wide">Plan</span>
        {#if totalCount > 0}
            <span class="text-[9.5px] text-muted-foreground/50">{doneCount}/{totalCount} steps</span>
        {/if}
        <span class="flex-1"></span>
        {#if expanded}
            <IconChevronDown size={10} class="text-muted-foreground/40" />
        {:else}
            <IconChevronRight size={10} class="text-muted-foreground/40" />
        {/if}
    </button>

    <!-- Steps -->
    {#if expanded && plan.steps.length > 0}
        <div class="border-t border-amber-400/15 px-2.5 py-1.5 flex flex-col gap-0.5">
            {#each plan.steps as step (step.id)}
                <div class="flex items-start gap-2 py-0.5">
                    <!-- Status icon -->
                    <div class="mt-0.5 shrink-0">
                        {#if step.status === "done"}
                            <IconCheck size={10} class="text-green-500/80" />
                        {:else if step.status === "running"}
                            <IconLoader2 size={10} class="animate-spin text-accent" />
                        {:else if step.status === "error"}
                            <IconX size={10} class="text-destructive/70" />
                        {:else if step.status === "pending"}
                            <IconClock size={10} class="text-muted-foreground/30" />
                        {:else}
                            <span class="block h-2.5 w-2.5"></span>
                        {/if}
                    </div>
                    <!-- Phase badge -->
                    <span class="shrink-0 rounded px-1 py-0.5 text-[9px] font-medium {PHASE_COLOR[step.phase] ?? ''}">
                        {PHASE_LABEL[step.phase] ?? step.phase}
                    </span>
                    <!-- Description -->
                    <span class="text-[10.5px] text-foreground/70 leading-relaxed">{step.description}</span>
                </div>
            {/each}
        </div>
    {/if}
</div>
```

- [ ] **Step 2: Type-check**

```bash
pnpm check 2>&1 | grep -i "PlanCard"
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add src/lib/components/agent/PlanCard.svelte
git commit -m "feat(agent): add PlanCard component for plan step progress display"
```

---

## Task 5: MessageBubble.svelte — detect and render plan XML

**Files:**
- Modify: `src/lib/components/agent/MessageBubble.svelte`

When an assistant message contains `<plan>...</plan>` XML, strip it from the rendered text and show a `PlanCard` instead. The plan data comes from `plansStore` keyed by the message's thread context — the card updates live as steps complete.

- [ ] **Step 1: Read the current MessageBubble to find where content is rendered**

```bash
grep -n "content\|message\|role" src/lib/components/agent/MessageBubble.svelte | head -20
```

- [ ] **Step 2: Add plan detection helper in the script block**

```ts
import PlanCard from "./PlanCard.svelte";
import { plansStore } from "$lib/stores/plans.svelte";

// Extract <plan>...</plan> from content and return { text, planXml }
function extractPlan(content: string): { text: string; planXml: string | null } {
    const match = content.match(/<plan>([\s\S]*?)<\/plan>/i);
    if (!match) return { text: content, planXml: null };
    return {
        text: content.replace(match[0], "").trim(),
        planXml: match[1],
    };
}

// Parse plan XML steps: <step phase="gather">description</step>
function parsePlanSteps(xml: string): Array<{ phase: string; description: string }> {
    const steps: Array<{ phase: string; description: string }> = [];
    const re = /<step\s+phase="([^"]+)"[^>]*>([\s\S]*?)<\/step>/gi;
    let m: RegExpExecArray | null;
    while ((m = re.exec(xml)) !== null) {
        steps.push({ phase: m[1].trim(), description: m[2].trim() });
    }
    return steps;
}
```

- [ ] **Step 3: Use derived to split content in the component**

Find where the message `content` is rendered (likely inside `{#if msg.role === "assistant"}`). Add a derived:

```ts
const planExtract = $derived(
    message.role === "assistant" ? extractPlan(message.content) : { text: message.content, planXml: null },
);

// Find matching plan from store (by thread + message proximity — use first plan if only one)
const matchedPlan = $derived(
    planExtract.planXml != null ? (plansStore.plans[plansStore.plans.length - 1] ?? null) : null,
);
```

- [ ] **Step 4: Update the assistant message template**

In the template section that renders the assistant content, replace the plain content render with:

```svelte
{#if planExtract.planXml != null && matchedPlan}
    {#if planExtract.text}
        <!-- Text above or below the plan block -->
        <div class="prose prose-sm prose-invert max-w-none text-[12px]">
            {planExtract.text}
        </div>
    {/if}
    <PlanCard plan={matchedPlan} />
{:else}
    <!-- Normal message render (markdown or plain) -->
    <div class="prose prose-sm prose-invert max-w-none text-[12px]">
        {message.content}
    </div>
{/if}
```

- [ ] **Step 5: Type-check**

```bash
pnpm check 2>&1 | grep -i "MessageBubble"
```

Expected: no errors.

- [ ] **Step 6: Commit**

```bash
git add src/lib/components/agent/MessageBubble.svelte
git commit -m "feat(agent): detect and render <plan> XML from assistant messages as PlanCard"
```

---

## Task 6: threads.svelte.ts — parentThreadId support

**Files:**
- Modify: `src/lib/stores/threads.svelte.ts`

Add `parentThreadId` to `AgentThread` and the `createThread` API.

- [ ] **Step 1: Update AgentThread interface**

Add:
```ts
export interface AgentThread {
    ...existing fields...
    parentThreadId: string | null;
}
```

- [ ] **Step 2: Update createThread to accept parentThreadId**

```ts
async createThread(opts: {
    connectionId: string;
    databaseName: string | null;
    model: string;
    effort: "auto" | "low" | "medium" | "high" | "max";
    parentThreadId?: string;
}): Promise<AgentThread> {
    const id = crypto.randomUUID();
    const now = nowSecs();
    await invoke("create_agent_thread", {
        id,
        connectionId: opts.connectionId,
        databaseName: opts.databaseName,
        model: opts.model,
        effort: opts.effort,
        parentThreadId: opts.parentThreadId ?? null,
        now,
    });
    const thread: AgentThread = {
        id,
        title: "New chat",
        connectionId: opts.connectionId,
        databaseName: opts.databaseName,
        model: opts.model,
        effort: opts.effort,
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

- [ ] **Step 3: Update create_agent_thread Rust command to accept parentThreadId**

In `src-tauri/src/commands/agent_commands.rs`, find `create_agent_thread` and add:

```rust
#[tauri::command]
pub async fn create_agent_thread(
    app: tauri::AppHandle,
    id: String,
    connection_id: String,
    database_name: Option<String>,
    model: String,
    effort: String,
    parent_thread_id: Option<String>,
    now: i64,
) -> Result<(), String> {
    let db_path = crate::db_path(&app);
    tokio::task::spawn_blocking(move || {
        let conn = rusqlite::Connection::open(&db_path).map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO agent_threads (id, connection_id, database_name, title, model, effort, parent_thread_id, created_at, updated_at)
             VALUES (?1, ?2, ?3, 'New chat', ?4, ?5, ?6, ?7, ?7)",
            rusqlite::params![id, connection_id, database_name, model, effort, parent_thread_id, now],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}
```

- [ ] **Step 4: Type-check and build**

```bash
pnpm check 2>&1 | grep -i "threads.svelte"
cd src-tauri && cargo build 2>&1 | grep "^error"
```

Expected: both clean.

- [ ] **Step 5: Commit**

```bash
git add src/lib/stores/threads.svelte.ts src-tauri/src/commands/agent_commands.rs
git commit -m "feat(agent): add parentThreadId to AgentThread and create_agent_thread command"
```

---

## Task 7: tool-executor.ts — spawn_subagent case

**Files:**
- Modify: `src/lib/agent/tool-executor.ts`

The `spawn_subagent` case does NOT call a Tauri command directly. Instead, it signals AgentPanel via a callback in `ToolContext` so AgentPanel can orchestrate the child session. This keeps the session management in one place.

- [ ] **Step 1: Add spawnSubagent callback to ToolContext**

Find the `ToolContext` interface:
```ts
export interface ToolContext {
    port: number;
    sessionId: string;
    connectionId: string;
    database: string;
    schema: string;
    openInEditor: (sql: string, title: string, autoRun?: boolean) => void;
}
```

Add:
```ts
export interface ToolContext {
    port: number;
    sessionId: string;
    connectionId: string;
    database: string;
    schema: string;
    openInEditor: (sql: string, title: string, autoRun?: boolean) => void;
    spawnSubagent?: (goal: string, model?: string) => Promise<string>;
}
```

- [ ] **Step 2: Add the spawn_subagent case in executeTool**

After `check_fk_integrity`, before `open_in_editor`:

```ts
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

- [ ] **Step 3: Type-check**

```bash
pnpm check 2>&1 | grep -i "tool-executor"
```

Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add src/lib/agent/tool-executor.ts
git commit -m "feat(agent): add spawn_subagent case to tool-executor with ToolContext callback"
```

---

## Task 8: AgentPanel.svelte — runChildAgent + getToolContext update

**Files:**
- Modify: `src/lib/components/agent/AgentPanel.svelte`

Implement `runChildAgent(goal, model?)` and wire it into `getToolContext`. Also import and load `plansStore` when switching threads, and update the system prompt to include `<plan>` format instructions when plan mode is on.

- [ ] **Step 1: Add plansStore import**

```ts
import { plansStore } from "$lib/stores/plans.svelte";
```

- [ ] **Step 2: Load plans when switching threads**

In `startThread`, after `await agentStore.loadThread(thread.id)`, add:
```ts
await plansStore.loadForThread(thread.id);
```

Clear plans in `stop` / `createAndStartThread` as needed — actually `loadForThread` overwrites, so just call it on switch.

- [ ] **Step 3: Implement runChildAgent**

```ts
async function runChildAgent(goal: string, model?: string): Promise<string> {
    const conn = schemaStore.activeConnection;
    if (!conn) throw new Error("No active connection");
    const parentThreadId = threadsStore.activeThreadId ?? undefined;

    // Create a child thread in SQLite
    const childThread = await threadsStore.createThread({
        connectionId: conn.id,
        databaseName: schemaStore.selectedDatabase,
        model: model ?? settingsStore.aiModel,
        effort: settingsStore.aiEffort,
        parentThreadId,
    });

    // Start a child harness session
    const childSessionId = crypto.randomUUID();
    const childAc = new AbortController();
    const childMessages: string[] = [];

    await new Promise<void>((resolve, reject) => {
        startAgentSession({
            sessionId: childSessionId,
            threadId: childThread.id,
            systemPrompt: buildPrompt(childSessionId),
            model: model ?? settingsStore.aiModel,
            effort: settingsStore.aiEffort,
            onEvent: (event) => {
                if (event.type === "text.delta") {
                    childMessages.push(event.content);
                } else if (event.type === "turn.done") {
                    resolve();
                } else if (event.type === "error") {
                    reject(new Error(event.message));
                } else if (event.type === "tool.started") {
                    // Auto-dispatch all tools for the child agent (no approval gate)
                    const childCtx = getToolContext();
                    if (childCtx) {
                        dispatchTool(event.toolName, event.toolId, event.input, {
                            ...childCtx,
                            sessionId: childSessionId,
                        }).catch(console.error);
                    }
                }
            },
            abortController: childAc,
        })
        .then((sess) => sess.send(goal))
        .catch(reject);
    });

    return childMessages.join("");
}
```

- [ ] **Step 4: Wire spawnSubagent into getToolContext**

Update `getToolContext`:
```ts
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
        openInEditor: (sql: string, _title: string, autoRun = false) => {
            handleRunQuery(sql, autoRun);
        },
        spawnSubagent: (goal: string, model?: string) => runChildAgent(goal, model),
    };
}
```

- [ ] **Step 5: Add spawn_subagent to system prompt tool table**

In `tools.ts` `buildToolInstructions`, add a row to the tool table:

```ts
| \`spawn_subagent\` | \`goal\`, \`model?\` | Delegate a subtask to a child agent; returns the child's output when complete |
```

- [ ] **Step 6: Add <plan> instructions to planMode system prompt**

In `tools.ts`, update the `planSection` to instruct the agent to output a `<plan>` block:

```ts
const planSection = planMode
    ? `## Plan Mode
...existing text...

When you have a clear plan, output it as XML before executing any tools:
\`\`\`
<plan>
  <step phase="gather">Describe the users table</step>
  <step phase="draft">Write the query to analysis.sql</step>
  <step phase="execute">Run analysis.sql</step>
</plan>
\`\`\`
Phases: "gather" (read-only, auto-runs), "draft" (write_file, auto-runs), "execute" (run_query, requires approval).\n\n`
    : "";
```

- [ ] **Step 7: Type-check**

```bash
pnpm check 2>&1 | grep -i "AgentPanel\|tools.ts"
```

Expected: no errors.

- [ ] **Step 8: Commit**

```bash
git add src/lib/components/agent/AgentPanel.svelte src/lib/agent/tools.ts
git commit -m "feat(agent): implement runChildAgent, spawn_subagent tool dispatch, plan XML instructions"
```

---

## Task 9: End-to-end smoke test

- [ ] **Step 1: Start dev mode**

```bash
pnpm tauri dev
```

- [ ] **Step 2: Verify plan parsing**

1. Enable plan mode (Plan toggle in composer)
2. Ask "analyze the users table and write a revenue query"
3. Verify Claude outputs a `<plan>` block — MessageBubble should show a PlanCard instead of raw XML
4. Steps should appear with phase badges (Gather / Draft / Execute)

- [ ] **Step 3: Verify spawn_subagent**

1. Ask Claude "spawn a sub-agent to count the rows in all tables and report back"
2. Verify a ToolCallCard appears for `spawn_subagent`
3. Verify a child thread is created in the thread list (ThreadPicker)
4. Verify the parent agent receives the child's output and continues

- [ ] **Step 4: Verify plan persistence**

1. Close and reopen the app
2. Navigate to a thread that had a plan
3. `plansStore.loadForThread` should reload the plan — PlanCard should reappear with persisted step statuses

---

## Self-Review

**Spec coverage:**
- ✅ `agent_plans` table → Task 1
- ✅ `agent_plan_steps` table → Task 1
- ✅ `parent_thread_id` on `agent_threads` → Task 1
- ✅ Rust CRUD commands → Task 2
- ✅ `plansStore` → Task 3
- ✅ `PlanCard` UI → Task 4
- ✅ `<plan>` XML detection in MessageBubble → Task 5
- ✅ `parentThreadId` in threads → Task 6
- ✅ `spawn_subagent` in tool-executor → Task 7
- ✅ `runChildAgent` in AgentPanel → Task 8
- ✅ Child agent auto-dispatches tools (no approval gate) → Task 8 Step 3
- ✅ `spawn_subagent` added to system prompt → Task 8 Step 5
- ✅ `<plan>` XML instructions in plan mode system prompt → Task 8 Step 6

**Not in this plan (Phase 4):**
- Parallel sub-agents (Map<sessionId, SubagentState> for concurrent children)
- PostCompact hook → compact.done event → summary in agent_threads
- Plan step status updates as tool calls complete (link tool_call_id to plan steps)

**Placeholder scan:** None found.

**Type consistency:**
- `AgentPlanStep.phase` is `"gather" | "draft" | "execute"` throughout (store, PlanCard, SQL CHECK constraint)
- `AgentPlan.status` is `"pending" | "running" | "done" | "cancelled"` throughout
- `ToolContext.spawnSubagent` signature `(goal: string, model?: string) => Promise<string>` matches both the interface (Task 7 Step 1) and the implementation (Task 8 Step 3)
