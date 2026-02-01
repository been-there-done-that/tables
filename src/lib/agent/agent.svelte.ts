import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export interface Message {
    id: string;
    session_id: string;
    role: "user" | "assistant" | "system" | "tool";
    content: string | null;
    tool_calls?: any;
    tool_call_id?: string;
    created_at: number;
}

export interface Session {
    id: string;
    title: string;
    created_at: number;
    updated_at: number;
}

export class AgentStore {
    sessions = $state<Session[]>([]);
    currentSession = $state<Session | null>(null);
    messages = $state<Message[]>([]);
    isStreaming = $state(false);
    streamingContent = $state("");
    streamingToolCalls = $state<any[]>([]);

    constructor() {
        this.loadSessions();
        this.setupEventListeners();
    }

    async loadSessions() {
        try {
            this.sessions = await invoke("list_agent_sessions");
        } catch (e) {
            console.error("Failed to load agent sessions", e);
        }
    }

    async selectSession(session: Session) {
        this.currentSession = session;
        try {
            this.messages = await invoke("get_agent_messages", { sessionId: session.id });
        } catch (e) {
            console.error("Failed to load messages", e);
        }
    }

    async createSession(title: string = "New Chat") {
        try {
            const session = await invoke<Session>("create_agent_session", { title });
            this.sessions = [session, ...this.sessions];
            await this.selectSession(session);
            return session;
        } catch (e) {
            console.error("Failed to create session", e);
        }
    }

    async sendMessage(content: string, provider: string, apiKey: string, model: string, apiUrl?: string) {
        if (!this.currentSession) {
            await this.createSession(content.slice(0, 30) + "...");
        }

        const userMessage: Message = {
            id: crypto.randomUUID(),
            session_id: this.currentSession!.id,
            role: "user",
            content,
            created_at: Math.floor(Date.now() / 1000),
        };

        this.messages = [...this.messages, userMessage];
        await invoke("add_agent_message", { message: userMessage });

        this.isStreaming = true;
        this.streamingContent = "";

        try {
            await invoke("llm_stream", {
                sessionId: this.currentSession!.id,
                request: {
                    provider,
                    api_key: apiKey,
                    api_url: apiUrl,
                    model,
                    messages: this.messages.map(m => ({
                        role: m.role,
                        content: m.content
                    }))
                }
            });
        } catch (e) {
            console.error("LLM Stream failed", e);
            this.isStreaming = false;
        }
    }

    private setupEventListeners() {
        listen("llm-chunk", (event: any) => {
            const { session_id, chunk, done, error } = event.payload;
            if (this.currentSession?.id !== session_id) return;

            if (done) {
                this.finalizeStreamingMessage();
                return;
            }

            if (error) {
                console.error("Stream error", error);
                this.isStreaming = false;
                return;
            }

            if (chunk) {
                try {
                    // Simple heuristic to distinguish between text and tool call JSON
                    if (chunk.startsWith("[{") || chunk.startsWith("{")) {
                        const parsed = JSON.parse(chunk);
                        this.streamingToolCalls = [...this.streamingToolCalls, ...(Array.isArray(parsed) ? parsed : [parsed])];
                    } else {
                        this.streamingContent += chunk;
                    }
                } catch (e) {
                    this.streamingContent += chunk;
                }
            }
        });
    }

    private async finalizeStreamingMessage() {
        if (!this.currentSession) return;

        const assistantMessage: Message = {
            id: crypto.randomUUID(),
            session_id: this.currentSession.id,
            role: "assistant",
            content: this.streamingContent || null,
            tool_calls: this.streamingToolCalls.length > 0 ? JSON.stringify(this.streamingToolCalls) : undefined,
            created_at: Math.floor(Date.now() / 1000),
        };

        this.messages = [...this.messages, assistantMessage];
        await invoke("add_agent_message", { message: assistantMessage });

        const toolCallsToExecute = [...this.streamingToolCalls];
        this.isStreaming = false;
        this.streamingContent = "";
        this.streamingToolCalls = [];

        if (toolCallsToExecute.length > 0) {
            await this.handleToolCalls(toolCallsToExecute);
        }

        // Refresh sessions to update the 'updated_at' timestamp order
        await this.loadSessions();
    }

    private async handleToolCalls(toolCalls: any[]) {
        const { toolRunner } = await import("./tool_runner");

        for (const toolCall of toolCalls) {
            try {
                const result = await toolRunner.execute(toolCall);
                const toolMsg: Message = {
                    id: crypto.randomUUID(),
                    session_id: this.currentSession!.id,
                    role: "tool",
                    content: JSON.stringify(result),
                    tool_call_id: toolCall.id,
                    created_at: Math.floor(Date.now() / 1000),
                };
                this.messages = [...this.messages, toolMsg];
                await invoke("add_agent_message", { message: toolMsg });
            } catch (e) {
                console.error("Tool execution failed", e);
            }
        }
    }

    async deleteSession(sessionId: string) {
        try {
            await invoke("delete_agent_session", { sessionId });
            this.sessions = this.sessions.filter(s => s.id !== sessionId);
            if (this.currentSession?.id === sessionId) {
                this.currentSession = null;
                this.messages = [];
            }
        } catch (e) {
            console.error("Failed to delete session", e);
        }
    }
}

export const agentStore = new AgentStore();
