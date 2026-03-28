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
