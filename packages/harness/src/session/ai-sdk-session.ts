import { streamText, type LanguageModel, type CoreMessage } from "ai";
import { HttpAdapter } from "../adapters/http-adapter";
import type { SessionConfig } from "../types";
import { createDbTools } from "../tools/definitions";
import { cancelSessionTools } from "../tool-bridge";

export class AiSdkSession extends HttpAdapter {
    protected config: SessionConfig;
    private model: LanguageModel;
    private messages: CoreMessage[] = [];
    private ac = new AbortController();

    constructor(model: LanguageModel, config: SessionConfig) {
        super();
        this.model = model;
        this.config = config;
        this.messages = [{ role: "system", content: config.systemPrompt }];
    }

    protected onStop(): void {
        this.ac.abort();
        cancelSessionTools(this.config.sessionId);
    }

    async isAvailable(): Promise<boolean> {
        return true;
    }

    async send(text: string): Promise<void> {
        this.messages.push({ role: "user", content: text });

        try {
            const tools = createDbTools(this.config.sessionId, (e) => this.emitFn(e));
            let responseMessages: CoreMessage[] = [];

            const result = streamText({
                model: this.model,
                messages: this.messages,
                tools,
                maxSteps: 20,
                abortSignal: this.ac.signal,
                onFinish: ({ response }) => {
                    responseMessages = response.messages as CoreMessage[];
                },
            });

            for await (const chunk of result.fullStream) {
                if (this.isAborted()) break;
                if (chunk.type === "text-delta") {
                    this.emitFn({ type: "text.delta", content: chunk.textDelta });
                } else if (chunk.type === "reasoning") {
                    this.emitFn({ type: "thinking.delta", content: (chunk as any).textDelta });
                }
            }

            if (!this.isAborted()) {
                this.messages.push(...responseMessages);
                this.emitFn({ type: "turn.done" });
            }
        } catch (e: unknown) {
            if (!this.isAborted()) {
                this.emitFn({ type: "error", message: String(e) });
            }
        }
    }
}
