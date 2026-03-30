// packages/harness/src/adapters/sdk-adapter.ts
import type { HarnessEvent, Session } from "../types";
import { hLog } from "../logger";

export abstract class SdkAdapter implements Session {
    protected emitFn: (e: HarnessEvent) => void = () => {};
    protected ac = new AbortController();
    private turnHasStreamEvents = false;
    private suppressedBashIds = new Set<string>();
    private activeToolUseId: string | null = null;
    private activeToolName: string | null = null;
    private partialInput = "";

    setEmit(fn: (e: HarnessEvent) => void) {
        this.emitFn = fn;
    }

    emitToolEvent(e: HarnessEvent) {
        this.emitFn(e);
    }

    stop() {
        this.ac.abort();
    }

    abstract isAvailable(): Promise<boolean>;
    abstract send(text: string): void;

    protected async consume(stream: AsyncIterable<unknown>) {
        hLog("debug", "sdk", "stream consume started");
        let msgCount = 0;
        try {
            for await (const msg of stream as AsyncIterable<Record<string, unknown>>) {
                msgCount++;
                if (msg.type === "stream_event") {
                    const ev = (msg as any).event as Record<string, unknown>;
                    if (ev?.type === "content_block_start") {
                        const block = (ev as any).content_block as Record<string, unknown>;
                        hLog("debug", "sdk", `content_block_start type=${block?.type} name=${block?.name ?? ""}`);
                        if (block?.type === "tool_use" && block?.name === "write_file") {
                            this.activeToolUseId = block.id as string;
                            this.activeToolName = block.name as string;
                            this.partialInput = "";
                        }
                    } else if (ev?.type === "content_block_delta") {
                        const delta = ev.delta as Record<string, unknown>;
                        if (delta?.type === "text_delta" && delta.text) {
                            this.turnHasStreamEvents = true;
                            this.emitFn({ type: "text.delta", content: delta.text as string });
                        } else if (delta?.type === "thinking_delta" && delta.thinking) {
                            this.turnHasStreamEvents = true;
                            this.emitFn({ type: "thinking.delta", content: delta.thinking as string });
                        } else if (delta?.type === "input_json_delta" && this.activeToolUseId) {
                            this.partialInput += (delta as any).partial_json ?? "";
                            const match = /"content"\s*:\s*"((?:[^"\\]|\\.)*)/.exec(this.partialInput);
                            if (match) {
                                const partial = match[1]
                                    .replace(/\\n/g, "\n")
                                    .replace(/\\"/g, '"')
                                    .replace(/\\\\/g, "\\");
                                this.emitFn({
                                    type: "tool.input_delta",
                                    toolId: this.activeToolUseId,
                                    toolName: this.activeToolName!,
                                    partialContent: partial,
                                });
                            }
                        }
                    } else if (ev?.type === "content_block_stop") {
                        hLog("debug", "sdk", "content_block_stop");
                        this.activeToolUseId = null;
                        this.activeToolName = null;
                        this.partialInput = "";
                    } else if (ev?.type === "message_stop") {
                        hLog("debug", "sdk", `message_stop stopReason=${(ev as any).stop_reason ?? "?"}`);
                    }

                } else if (msg.type === "assistant") {
                    const message = msg.message as { content: Array<Record<string, unknown>>; stop_reason?: string };
                    hLog("debug", "sdk", `assistant blocks=${message?.content?.length ?? 0} stop_reason=${message?.stop_reason ?? "?"}`);
                    for (const block of message?.content ?? []) {
                        if (block.type === "text" && !this.turnHasStreamEvents) {
                            this.emitFn({ type: "text.delta", content: block.text as string });
                        } else if (block.type === "thinking" && !this.turnHasStreamEvents) {
                            this.emitFn({ type: "thinking.delta", content: block.thinking as string });
                        } else if (block.type === "tool_use") {
                            hLog("debug", "sdk", `tool_use name=${block.name} id=${block.id}`);
                            if (block.name === "Bash") {
                                const cmd = ((block.input as any)?.command ?? "") as string;
                                if (cmd.includes("/db/")) {
                                    this.suppressedBashIds.add(block.id as string);
                                    continue;
                                }
                            }
                            this.emitFn({
                                type: "tool.started",
                                toolId: block.id as string,
                                toolName: block.name as string,
                                input: block.input,
                            });
                        }
                    }

                } else if (msg.type === "user") {
                    const message = msg.message as { content: Array<Record<string, unknown>> };
                    hLog("debug", "sdk", `user (tool results) blocks=${message?.content?.length ?? 0}`);
                    for (const block of message?.content ?? []) {
                        if (block.type === "tool_result") {
                            const toolUseId = block.tool_use_id as string;
                            if (this.suppressedBashIds.has(toolUseId)) {
                                this.suppressedBashIds.delete(toolUseId);
                                continue;
                            }
                            const content = Array.isArray(block.content)
                                ? (block.content as Array<Record<string, unknown>>)
                                      .filter((c) => c.type === "text")
                                      .map((c) => c.text)
                                      .join("")
                                : String(block.content ?? "");
                            hLog("debug", "sdk", `tool_result toolId=${toolUseId} outputLen=${content.length}`);
                            this.emitFn({ type: "tool.completed", toolId: toolUseId, output: content });
                        }
                    }

                } else if (msg.type === "system") {
                    const sessionId = (msg as any).session_id as string | undefined;
                    hLog("debug", "sdk", `system sessionId=${sessionId ?? "none"}`);
                    if (sessionId) this.emitFn({ type: "session.init", sdkSessionId: sessionId });

                } else if (msg.type === "result") {
                    const subtype = (msg as any).subtype;
                    const numTurns = (msg as any).num_turns;
                    hLog("info", "sdk", `result subtype=${subtype ?? "?"} turns=${numTurns ?? "?"} totalMsgs=${msgCount}`);
                    this.turnHasStreamEvents = false;
                    this.emitFn({ type: "turn.done" });

                } else {
                    hLog("debug", "sdk", `unknown msg type="${msg.type}"`);
                }
            }
            hLog("info", "sdk", `stream exhausted — totalMsgs=${msgCount}`);
        } catch (e: unknown) {
            if (this.ac.signal.aborted) {
                hLog("info", "sdk", `stream aborted after ${msgCount} messages`);
            } else {
                hLog("error", "sdk", `stream error after ${msgCount} messages: ${String(e)}`);
                this.emitFn({ type: "error", message: String(e) });
            }
        }
    }
}
