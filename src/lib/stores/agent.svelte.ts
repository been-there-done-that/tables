import type { AgentSession } from "$lib/agent/claude";

export interface AgentMessage {
    id: string;
    role: "user" | "assistant";
    content: string;
    streaming: boolean;
    thinking?: string;
    thinkingStreaming?: boolean;
    timestamp: number;
}

export interface AgentToolCall {
    id: string;
    toolName: string;
    input: unknown;
    status: "running" | "done" | "error";
    output?: string;
    timestamp: number;
    startedAt: number;
}

class AgentStore {
    messages = $state<AgentMessage[]>([]);
    toolCalls = $state<AgentToolCall[]>([]);
    status = $state<"idle" | "running" | "error">("idle");
    session = $state<AgentSession | null>(null);
    errorMessage = $state<string | null>(null);

    addUserMessage(text: string) {
        this.messages.push({
            id: crypto.randomUUID(),
            role: "user",
            content: text,
            streaming: false,
            timestamp: Date.now(),
        });
    }

    /** Start a new streaming assistant message, return its id */
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
        }
    }

    addToolCall(toolId: string, toolName: string, input: unknown) {
        this.toolCalls.push({
            id: toolId,
            toolName,
            input,
            status: "running",
            timestamp: Date.now(),
            startedAt: Date.now(),
        });
    }

    completeToolCall(toolId: string, output: string) {
        const tc = this.toolCalls.find((t) => t.id === toolId);
        if (tc) {
            tc.status = "done";
            tc.output = output;
        }
    }

    failToolCall(toolId: string) {
        const tc = this.toolCalls.find((t) => t.id === toolId);
        if (tc) tc.status = "error";
    }

    setStatus(s: "idle" | "running" | "error") {
        this.status = s;
    }

    setError(msg: string) {
        this.errorMessage = msg;
        this.status = "error";
    }

    clear() {
        this.messages = [];
        this.toolCalls = [];
        this.status = "idle";
        this.errorMessage = null;
    }
}

export const agentStore = new AgentStore();
