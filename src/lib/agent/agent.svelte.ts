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
            if (this.sessions.length > 0 && !this.currentSession) {
                this.selectSession(this.sessions[0]);
            }
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

    async createSession(title: string = "New Session") {
        try {
            const session = await invoke<Session>("create_agent_session", { title });
            this.sessions = [session, ...this.sessions];
            await this.selectSession(session);
            return session;
        } catch (e) {
            console.error("Failed to create session", e);
        }
    }

    async renameSession(id: string, title: string) {
        try {
            const updated = await invoke<Session>("update_agent_session", { id, title });
            // Update local store
            this.sessions = this.sessions.map(s => s.id === id ? updated : s);
            if (this.currentSession?.id === id) {
                this.currentSession = updated;
            }
        } catch (e) {
            console.error("Failed to rename session", e);
        }
    }

    private async autoNameSession() {
        if (!this.currentSession || this.messages.length < 2) return;

        const sessionId = this.currentSession.id;
        const tempId = `auto-name-${crypto.randomUUID()}`;
        const firstUserMsg = this.messages.find(m => m.role === 'user')?.content || "";
        const firstAssistantMsg = this.messages.find(m => m.role === 'assistant')?.content || "";

        const prompt = `Generate a concise title (3-5 words) for this conversation based on the first iteration.\nUser: ${firstUserMsg.slice(0, 200)}\nAssistant: ${firstAssistantMsg.slice(0, 200)}\nTitle:`;

        let generatedTitle = "";

        // Setup temporary listener
        const unlisten = await listen("llm-chunk", (event: any) => {
            const { session_id, chunk, done } = event.payload;
            if (session_id !== tempId) return;
            if (chunk) generatedTitle += chunk;
        });

        const { settingsStore } = await import("$lib/stores/settings.svelte"); // Dynamic import to avoid cycles if any

        try {
            await invoke("llm_stream", {
                sessionId: tempId,
                request: {
                    provider: settingsStore.aiAgentModel.startsWith("claude") ? "anthropic" : "openai", // Crude heuristic, improve later
                    api_key: settingsStore.aiAgentApiKey,
                    api_url: settingsStore.aiAgentUrl,
                    model: settingsStore.aiAgentModel,
                    messages: [{ role: "user", content: prompt }],
                    persist: false
                }
            });

            // Wait a bit for stream to finish implies we need to track 'done' in the listener or wait for promise?
            // invoke("llm_stream") returns when stream is established or finished? 
            // In my rust code it awaits stream completion! 
            // So by the time await invoke returns, the stream is done.

            if (generatedTitle.trim()) {
                let cleanTitle = generatedTitle.trim().replace(/^["']|["']$/g, '');
                await this.renameSession(sessionId, cleanTitle);
            }
        } catch (e) {
            console.warn("Auto-naming failed", e);
        } finally {
            unlisten();
        }
    }

    async sendMessage(content: string, provider: string, apiKey: string, model: string, apiUrl?: string) {
        if (!this.currentSession) {
            await this.createSession(); // Default New Session
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
                    })),
                    persist: true
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

        // Backend persistence handles the INSERT. We just update local state.
        const assistantMessage: Message = {
            id: crypto.randomUUID(), // Local ID, backend creates its own but that's fine for display
            session_id: this.currentSession.id,
            role: "assistant",
            content: this.streamingContent || null,
            tool_calls: this.streamingToolCalls.length > 0 ? JSON.stringify(this.streamingToolCalls) : undefined,
            created_at: Math.floor(Date.now() / 1000),
        };

        this.messages = [...this.messages, assistantMessage];
        // REMOVED: await invoke("add_agent_message", ...); 

        const toolCallsToExecute = [...this.streamingToolCalls];
        this.isStreaming = false;
        this.streamingContent = "";
        this.streamingToolCalls = [];

        if (toolCallsToExecute.length > 0) {
            await this.handleToolCalls(toolCallsToExecute);
        }

        // Refresh sessions to update 'updated_at'
        await this.loadSessions();

        // Check for auto-naming
        // If message count is exactly 2 (User + Assistant), it's the first turn.
        if (this.messages.length === 2 && assistantMessage.role === "assistant") {
            this.autoNameSession();
        }
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
            await invoke("delete_agent_session", { id: sessionId }); // Note: arg name changed to 'id' in backend command? 
            // Checked agent_commands.rs: delete_agent_session(id: String, ...) -> so arg is 'id'.
            // wait, in frontend invoke("delete_agent_session", { id: sessionId }) is correct if backend arg is 'id'.
            // Previous code had { sessionId }.
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
