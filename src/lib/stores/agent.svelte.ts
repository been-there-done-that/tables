import { invoke } from "@tauri-apps/api/core";
import type { AgentSession } from "$lib/agent/claude";

export interface AgentMessage {
    id: string;
    role: "user" | "assistant";
    content: string;
    streaming: boolean;
    thinking?: string;
    thinkingStreaming?: boolean;
    timestamp: number;
    docJson?: unknown; // TipTap doc JSON for user messages with @chips
    isError?: boolean; // inline error messages from failed turns
}

export interface AgentToolCall {
    id: string;
    toolName: string;
    input: unknown;
    status: "running" | "done" | "error";
    output?: string;
    timestamp: number;
    startedAt: number;
    completedAt?: number; // wall-clock ms when tool finished (for accurate duration)
}

export interface TurnSummary {
    id: string;
    totalMs: number;
    model: string;
    timestamp: number;
    cancelled?: boolean;
}

function nowSecs(): number {
    return Math.floor(Date.now() / 1000);
}

class AgentStore {
    messages = $state<AgentMessage[]>([]);
    toolCalls = $state<AgentToolCall[]>([]);
    turnSummaries = $state<TurnSummary[]>([]);
    status = $state<"idle" | "running" | "error">("idle");
    session = $state<AgentSession | null>(null);
    errorMessage = $state<string | null>(null);
    threadId = $state<string | null>(null);

    private persistMessage(msg: AgentMessage) {
        if (!this.threadId) return;
        invoke("append_agent_message", {
            id: msg.id,
            threadId: this.threadId,
            role: msg.role,
            content: msg.content,
            thinking: msg.thinking ?? null,
            timestamp: Math.floor(msg.timestamp / 1000),
            now: nowSecs(),
        }).catch((e) => console.error("[agentStore] persist message failed:", e));
    }

    private persistToolCall(tc: AgentToolCall) {
        if (!this.threadId) return;
        invoke("upsert_agent_tool_call", {
            id: tc.id,
            threadId: this.threadId,
            toolName: tc.toolName,
            input: JSON.stringify(tc.input),
            output: tc.output ?? null,
            status: tc.status,
            startedAt: Math.floor(tc.startedAt / 1000),
            completedAt: tc.status !== "running" ? nowSecs() : null,
        }).catch((e) => console.error("[agentStore] persist tool call failed:", e));
    }

    addUserMessage(text: string, docJson?: unknown) {
        const msg: AgentMessage = {
            id: crypto.randomUUID(),
            role: "user",
            content: text,
            streaming: false,
            timestamp: Date.now(),
            docJson,
        };
        this.messages.push(msg);
        this.persistMessage(msg);
    }

    startAssistantMessage(): string {
        const id = crypto.randomUUID();
        this.messages.push({
            id,
            role: "assistant",
            content: "",
            streaming: true,
            timestamp: Date.now(),
        });
        return id;
    }

    appendDelta(id: string, delta: string) {
        const msg = this.messages.find((m) => m.id === id);
        if (msg) msg.content += delta;
    }

    appendThinking(id: string, delta: string) {
        const msg = this.messages.find((m) => m.id === id);
        if (msg) {
            msg.thinking = (msg.thinking ?? "") + delta;
            msg.thinkingStreaming = true;
        }
    }

    finalizeMessage(id: string) {
        const msg = this.messages.find((m) => m.id === id);
        if (msg) {
            msg.streaming = false;
            msg.thinkingStreaming = false;
            this.persistMessage(msg); // persist final content after streaming ends
        }
    }

    addToolCall(toolId: string, toolName: string, input: unknown) {
        const tc: AgentToolCall = {
            id: toolId,
            toolName,
            input,
            status: "running",
            timestamp: Date.now(),
            startedAt: Date.now(),
        };
        this.toolCalls.push(tc);
        this.persistToolCall(tc);
    }

    completeToolCall(toolId: string, output: string) {
        const tc = this.toolCalls.find((t) => t.id === toolId);
        if (tc) {
            tc.status = "done";
            tc.output = output;
            tc.completedAt = Date.now();
            this.persistToolCall(tc);
        }
    }

    failToolCall(toolId: string) {
        const tc = this.toolCalls.find((t) => t.id === toolId);
        if (tc) {
            tc.status = "error";
            tc.completedAt = Date.now();
            this.persistToolCall(tc);
        }
    }

    addTurnSummary(totalMs: number, model: string, cancelled = false) {
        this.turnSummaries.push({
            id: crypto.randomUUID(),
            totalMs,
            model,
            timestamp: Date.now(),
            cancelled,
        });
    }

    addErrorMessage(text: string) {
        this.messages.push({
            id: crypto.randomUUID(),
            role: "assistant",
            content: text,
            streaming: false,
            isError: true,
            timestamp: Date.now(),
        });
        // Not persisted — errors are transient UI feedback
    }

    setStatus(s: "idle" | "running" | "error") {
        this.status = s;
    }

    setError(msg: string) {
        this.errorMessage = msg;
        this.status = "error";
    }

    async loadThread(threadId: string) {
        // Set threadId to null while loading so any in-flight persists from the
        // previous session don't accidentally write into the new thread's records.
        this.threadId = null;
        this.messages = [];
        this.toolCalls = [];
        this.status = "idle";
        this.errorMessage = null;

        try {
            const [msgs, tools] = await Promise.all([
                invoke<Array<{
                    id: string; threadId: string; role: string;
                    content: string; thinking: string | null; timestamp: number;
                }>>("list_agent_messages", { threadId }),
                invoke<Array<{
                    id: string; threadId: string; toolName: string;
                    input: string; output: string | null; status: string;
                    startedAt: number; completedAt: number | null;
                }>>("list_agent_tool_calls", { threadId }),
            ]);

            this.messages = msgs.map((m) => ({
                id: m.id,
                role: m.role as "user" | "assistant",
                content: m.content,
                streaming: false,
                thinking: m.thinking ?? undefined,
                timestamp: m.timestamp * 1000,
            }));

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

            // Only activate persists for this thread after all state is loaded.
            this.threadId = threadId;
        } catch (e) {
            console.error("[agentStore] loadThread failed:", e);
        }
    }

    clear() {
        this.messages = [];
        this.toolCalls = [];
        this.turnSummaries = [];
        this.status = "idle";
        this.errorMessage = null;
        this.threadId = null;
    }
}

export const agentStore = new AgentStore();
