// src/lib/stores/threads.svelte.ts
import { invoke } from "@tauri-apps/api/core";

export interface AgentThread {
    id: string;
    title: string;
    connectionId: string;
    databaseName: string | null;
    model: string;
    effort: string;
    sdkSessionId: string | null;
    summary: string | null;
    parentThreadId: string | null;
    createdAt: number;
    updatedAt: number;
}

function nowSecs(): number {
    return Math.floor(Date.now() / 1000);
}

class ThreadsStore {
    threads = $state<AgentThread[]>([]);
    activeThreadId = $state<string | null>(null);

    get activeThread(): AgentThread | null {
        return this.threads.find((t) => t.id === this.activeThreadId) ?? null;
    }

    async load(connectionId: string, databaseName: string | null) {
        try {
            const rows = await invoke<AgentThread[]>("list_agent_threads", {
                connectionId,
                databaseName,
            });
            this.threads = rows;
        } catch (e) {
            console.error("[threads] load failed:", e);
            this.threads = [];
        }
    }

    async createThread(opts: {
        connectionId: string;
        databaseName: string | null;
        model: string;
        effort: "auto" | "low" | "medium" | "high" | "max";
        parentThreadId?: string | null;
    }): Promise<AgentThread> {
        const id = crypto.randomUUID();
        const now = nowSecs();
        try {
            await invoke("create_agent_thread", {
                id,
                connectionId: opts.connectionId,
                databaseName: opts.databaseName,
                model: opts.model,
                effort: opts.effort,
                parentThreadId: opts.parentThreadId ?? null,
                now,
            });
        } catch (e) {
            console.error("[threads] createThread failed:", e);
            throw e;
        }
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

    async setSdkSessionId(threadId: string, sdkSessionId: string) {
        const now = nowSecs();
        try {
            await invoke("update_agent_thread_sdk_session", { id: threadId, sdkSessionId, now });
        } catch (e) {
            console.error("[threads] setSdkSessionId failed:", e);
            return;
        }
        const t = this.threads.find((x) => x.id === threadId);
        if (t) {
            t.sdkSessionId = sdkSessionId;
            t.updatedAt = now;
        }
    }

    async setTitle(threadId: string, title: string) {
        const now = nowSecs();
        try {
            await invoke("update_agent_thread_title", { id: threadId, title, now });
        } catch (e) {
            console.error("[threads] setTitle failed:", e);
            return;
        }
        const t = this.threads.find((x) => x.id === threadId);
        if (t) {
            t.title = title;
            t.updatedAt = now;
            // Re-sort by updated_at desc
            this.threads = [...this.threads].sort((a, b) => b.updatedAt - a.updatedAt);
        }
    }

    async deleteThread(threadId: string) {
        try {
            await invoke("delete_agent_thread", { id: threadId });
        } catch (e) {
            console.error("[threads] deleteThread failed:", e);
            return;
        }
        this.threads = this.threads.filter((t) => t.id !== threadId);
        if (this.activeThreadId === threadId) {
            this.activeThreadId = this.threads[0]?.id ?? null;
        }
    }

    setActive(threadId: string) {
        this.activeThreadId = threadId;
    }

    clear() {
        this.threads = [];
        this.activeThreadId = null;
    }
}

export const threadsStore = new ThreadsStore();
