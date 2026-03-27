import { query } from "@anthropic-ai/claude-agent-sdk";

class AsyncQueue<T> {
    private buffer: T[] = [];
    private waiting: ((value: IteratorResult<T>) => void) | null = null;
    private done = false;

    push(item: T) {
        if (this.waiting) {
            const resolve = this.waiting;
            this.waiting = null;
            resolve({ value: item, done: false });
        } else {
            this.buffer.push(item);
        }
    }

    close() {
        this.done = true;
        if (this.waiting) {
            this.waiting({ value: undefined as any, done: true });
            this.waiting = null;
        }
    }

    [Symbol.asyncIterator](): AsyncIterator<T> {
        return {
            next: (): Promise<IteratorResult<T>> => {
                if (this.buffer.length > 0) {
                    return Promise.resolve({ value: this.buffer.shift()!, done: false });
                }
                if (this.done) {
                    return Promise.resolve({ value: undefined as any, done: true });
                }
                return new Promise((resolve) => { this.waiting = resolve; });
            },
        };
    }
}

export type HarnessEvent =
    | { type: "text.delta"; content: string }
    | { type: "thinking.delta"; content: string }
    | { type: "tool.started"; toolId: string; toolName: string; input: unknown }
    | { type: "tool.completed"; toolId: string; output: string }
    | { type: "tool.input_delta"; toolId: string; toolName: string; partialContent: string }
    | { type: "session.init"; sdkSessionId: string }
    | { type: "turn.done" }
    | { type: "error"; message: string };

type SDKMsg = {
    type: "user";
    session_id: string;
    parent_tool_use_id: null;
    message: { role: "user"; content: Array<{ type: "text"; text: string }> };
};

export class ClaudeSession {
    private queue = new AsyncQueue<SDKMsg>();
    private ac = new AbortController();
    private firstMessage = true;
    private emitFn: (e: HarnessEvent) => void = () => {};
    // Track whether stream_event text tokens have arrived this turn
    private turnHasStreamEvents = false;
    // Track Bash tool IDs that are /db/ API calls so we can suppress them
    private suppressedBashIds = new Set<string>();
    // Track active tool_use block for write_file streaming
    private activeToolUseId: string | null = null;
    private activeToolName: string | null = null;
    private partialInput: string = "";

    constructor(
        private systemPrompt: string,
        claudePath: string | null,
        model?: string,
        effort?: "auto" | "low" | "medium" | "high" | "max",
        cwd?: string,
        resumeSessionId?: string,
    ) {
        if (cwd) {
            Bun.spawnSync(["mkdir", "-p", cwd]);
        }
        const stream = query({
            prompt: this.queue as any,
            options: {
                permissionMode: "bypassPermissions",
                abortController: this.ac,
                env: { ...process.env },
                includePartialMessages: true,
                ...(claudePath ? { pathToClaudeCodeExecutable: claudePath } : {}),
                ...(model ? { model } : {}),
                ...(effort ? { effort } : {}),
                ...(cwd ? { cwd } : {}),
                ...(resumeSessionId ? { resume: resumeSessionId } : {}),
                persistSession: true,
            } as any,
        });
        this.consume(stream);
    }

    setEmit(fn: (e: HarnessEvent) => void) {
        this.emitFn = fn;
    }

    emitToolEvent(e: HarnessEvent) {
        this.emitFn(e);
    }

    private async consume(stream: AsyncIterable<unknown>) {
        console.error("[session] consume() started");
        try {
            for await (const msg of stream as AsyncIterable<Record<string, unknown>>) {
                console.error(`[session] sdk msg type: ${msg.type}`);

                if (msg.type === "stream_event") {
                    // Per-token streaming — preferred path when SDK emits these
                    const streamEvent = (msg as any).event as Record<string, unknown>;
                    console.error(`[session] stream_event: ${streamEvent?.type}`);
                    if (streamEvent?.type === "content_block_start") {
                        const block = (streamEvent as Record<string, unknown>).content_block as Record<string, unknown>;
                        if (block?.type === "tool_use" && block?.name === "write_file") {
                            this.activeToolUseId = block.id as string;
                            this.activeToolName = block.name as string;
                            this.partialInput = "";
                        }
                    } else if (streamEvent?.type === "content_block_delta") {
                        const delta = streamEvent.delta as Record<string, unknown>;
                        if (delta?.type === "text_delta" && delta.text) {
                            this.turnHasStreamEvents = true;
                            this.emitFn({ type: "text.delta", content: delta.text as string });
                        } else if (delta?.type === "thinking_delta" && delta.thinking) {
                            this.turnHasStreamEvents = true;
                            this.emitFn({ type: "thinking.delta", content: delta.thinking as string });
                        } else if (delta?.type === "input_json_delta" && this.activeToolUseId) {
                            this.partialInput += (delta as Record<string, unknown>).partial_json ?? "";
                            // Extract partial "content" value from accumulated JSON string
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
                    } else if (streamEvent?.type === "content_block_stop") {
                        if (this.activeToolUseId) {
                            this.activeToolUseId = null;
                            this.activeToolName = null;
                            this.partialInput = "";
                        }
                    }

                } else if (msg.type === "assistant") {
                    const message = msg.message as { content: Array<Record<string, unknown>> };
                    for (const block of message?.content ?? []) {
                        if (block.type === "text") {
                            // Only emit from assistant block if stream_event didn't already stream it
                            if (!this.turnHasStreamEvents) {
                                console.error(`[session] assistant text fallback: "${String(block.text).slice(0, 40)}"`);
                                this.emitFn({ type: "text.delta", content: block.text as string });
                            }
                        } else if (block.type === "thinking") {
                            if (!this.turnHasStreamEvents) {
                                this.emitFn({ type: "thinking.delta", content: block.thinking as string });
                            }
                        } else if (block.type === "tool_use") {
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
                            this.emitFn({
                                type: "tool.completed",
                                toolId: toolUseId,
                                output: content,
                            });
                        }
                    }

                } else if (msg.type === "system") {
                    const sessionId = (msg as any).session_id as string | undefined;
                    if (sessionId) {
                        this.emitFn({ type: "session.init", sdkSessionId: sessionId });
                    }

                } else if (msg.type === "result") {
                    console.error(`[session] result:`, JSON.stringify(msg).slice(0, 200));
                    this.turnHasStreamEvents = false; // reset for next turn
                    this.emitFn({ type: "turn.done" });

                } else {
                    console.error(`[session] unhandled msg type: ${msg.type}`);
                }
            }
        } catch (e: unknown) {
            if (!this.ac.signal.aborted) {
                console.error(`[session] stream error:`, String(e));
                this.emitFn({ type: "error", message: String(e) });
            }
        }
    }

    private msg(content: string): SDKMsg {
        return {
            type: "user",
            session_id: "",
            parent_tool_use_id: null,
            message: { role: "user", content: [{ type: "text", text: content }] },
        };
    }

    send(text: string) {
        if (this.firstMessage) {
            this.firstMessage = false;
            this.queue.push(this.msg(`${this.systemPrompt}\n\n---\n\n${text}`));
        } else {
            this.queue.push(this.msg(text));
        }
    }

    stop() {
        this.ac.abort();
        this.queue.close();
    }
}
