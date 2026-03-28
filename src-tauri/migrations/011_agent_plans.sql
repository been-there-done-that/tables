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
