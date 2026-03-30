import { streamText, type LanguageModel, type CoreMessage } from "ai";
import { HttpAdapter } from "../adapters/http-adapter";
import type { SessionConfig } from "../types";
import { createDbTools } from "../tools/definitions";
import { cancelSessionTools } from "../tool-bridge";
import { hLog } from "../logger";

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
        hLog("info", "ai-sdk", `send session=${this.config.sessionId} msgCount=${this.messages.length} textLen=${text.length}`);

        try {
            const tools = createDbTools(this.config.sessionId, (e) => this.emitFn(e));
            let responseMessages: CoreMessage[] = [];
            let chunkCount = 0;

            const result = streamText({
                model: this.model,
                messages: this.messages,
                tools,
                maxSteps: 20,
                abortSignal: this.ac.signal,
                onChunk: ({ chunk }) => {
                    if (chunk.type === "tool-call") {
                        hLog("debug", "ai-sdk", `tool-call name=${chunk.toolName}`);
                    }
                },
                onFinish: ({ response, finishReason, usage }) => {
                    hLog("info", "ai-sdk", `onFinish reason=${finishReason} chunks=${chunkCount} msgs=${response.messages.length} inTokens=${usage?.promptTokens ?? "?"} outTokens=${usage?.completionTokens ?? "?"}`);
                    responseMessages = response.messages as CoreMessage[];
                },
                onError: ({ error }) => {
                    hLog("error", "ai-sdk", `streamText onError: ${String(error)}`);
                },
            });

            for await (const chunk of result.fullStream) {
                if (this.isAborted()) {
                    hLog("info", "ai-sdk", `stream aborted after ${chunkCount} chunks`);
                    break;
                }
                chunkCount++;
                if (chunk.type === "text-delta") {
                    this.emitFn({ type: "text.delta", content: chunk.textDelta });
                } else if (chunk.type === "reasoning") {
                    this.emitFn({ type: "thinking.delta", content: (chunk as any).textDelta });
                } else if (chunk.type === "error") {
                    hLog("error", "ai-sdk", `stream chunk error: ${String((chunk as any).error)}`);
                }
            }

            if (!this.isAborted()) {
                this.messages.push(...responseMessages);
                hLog("info", "ai-sdk", `turn done — history now ${this.messages.length} messages`);
                this.emitFn({ type: "turn.done" });
            }
        } catch (e: unknown) {
            if (this.isAborted()) {
                hLog("info", "ai-sdk", "send aborted (expected)");
            } else {
                hLog("error", "ai-sdk", `send error: ${String(e)}`);
                this.emitFn({ type: "error", message: String(e) });
            }
        }
    }
}
