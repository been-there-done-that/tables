import { createOpencodeClient } from "@opencode-ai/sdk";
import type {
    Event,
    EventMessagePartUpdated,
    TextPart,
    ToolPart,
} from "@opencode-ai/sdk";
import { HttpAdapter } from "../adapters/http-adapter";
import type { SessionConfig } from "../types";

export class OpenCodeProvider extends HttpAdapter {
    private client: ReturnType<typeof createOpencodeClient>;
    private ocSessionId: string | null = null;
    private abortController = new AbortController();
    private serverUrl: string;

    constructor(config: SessionConfig) {
        super();
        this.serverUrl =
            (config.providerConfig?.serverUrl as string) ?? "http://localhost:4096";
        this.client = createOpencodeClient({ baseUrl: this.serverUrl });
    }

    async isAvailable(): Promise<boolean> {
        try {
            const res = await fetch(this.serverUrl, { method: "HEAD", signal: AbortSignal.timeout(500) });
            return res.ok || res.status < 500;
        } catch {
            return false;
        }
    }

    protected onStop() {
        this.abortController.abort();
        if (this.ocSessionId) {
            // best-effort cleanup
            this.client.session
                .delete({ path: { id: this.ocSessionId } })
                .catch(() => {});
        }
    }

    async send(text: string) {
        try {
            // Create session on first send
            if (!this.ocSessionId) {
                const result = await this.client.session.create({ body: {} });
                const session = result.data;
                if (!session) throw new Error("Failed to create OpenCode session");
                this.ocSessionId = session.id;
                this.emitFn({ type: "session.init", sdkSessionId: session.id });
            }

            // Post message
            await this.client.session.prompt({
                path: { id: this.ocSessionId },
                body: {
                    parts: [{ type: "text", text }],
                },
            });

            // Stream events via global SSE stream
            const { stream } = await this.client.event.subscribe({
                signal: this.abortController.signal,
            });

            for await (const event of stream) {
                if (this.isAborted()) break;
                this.handleEvent(event as Event, this.ocSessionId);
                // Stop streaming when session goes idle
                if (
                    event.type === "session.idle" &&
                    (event as { type: string; properties: { sessionID: string } })
                        .properties.sessionID === this.ocSessionId
                ) {
                    break;
                }
            }
        } catch (e: unknown) {
            if (!this.isAborted()) {
                this.emitFn({ type: "error", message: String(e) });
            }
        }
    }

    private handleEvent(event: Event, sessionId: string) {
        const type = event.type;

        if (type === "message.part.updated") {
            const e = event as EventMessagePartUpdated;
            const part = e.properties.part;
            const delta = e.properties.delta;

            if (part.type === "text") {
                const textPart = part as TextPart;
                // Use delta if provided; otherwise fall back to full text
                const content = delta ?? textPart.text;
                if (content) {
                    this.emitFn({ type: "text.delta", content });
                }
            } else if (part.type === "reasoning") {
                const content = delta ?? (part as { text?: string }).text;
                if (content) {
                    this.emitFn({ type: "thinking.delta", content });
                }
            } else if (part.type === "tool") {
                const toolPart = part as ToolPart;
                const state = toolPart.state;
                if (state.status === "running") {
                    this.emitFn({
                        type: "tool.started",
                        toolId: toolPart.callID,
                        toolName: toolPart.tool,
                        input: state.input,
                    });
                } else if (state.status === "completed") {
                    this.emitFn({
                        type: "tool.completed",
                        toolId: toolPart.callID,
                        output: state.output,
                    });
                } else if (state.status === "error") {
                    this.emitFn({
                        type: "tool.completed",
                        toolId: toolPart.callID,
                        output: `error: ${state.error}`,
                    });
                }
            }
        } else if (type === "session.idle") {
            const e = event as { type: string; properties: { sessionID: string } };
            if (e.properties.sessionID === sessionId) {
                this.emitFn({ type: "turn.done" });
            }
        }
    }
}
