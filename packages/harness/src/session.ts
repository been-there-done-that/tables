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
                return new Promise((resolve) => {
                    this.waiting = resolve;
                });
            },
        };
    }
}

export type HarnessEvent =
    | { type: "text.delta"; content: string }
    | { type: "thinking.delta"; content: string }
    | { type: "tool.started"; toolId: string; toolName: string; input: unknown }
    | { type: "tool.completed"; toolId: string; output: string }
    | { type: "turn.done" }
    | { type: "error"; message: string };

type SDKMsg = { type: "user"; message: { role: "user"; content: string }; parent_tool_use_id: null };

export class ClaudeSession {
    private queue = new AsyncQueue<SDKMsg>();
    private ac = new AbortController();
    private firstMessage = true;
    private emitFn: (e: HarnessEvent) => void = () => {};

    constructor(
        private systemPrompt: string,
        claudePath: string | null,
    ) {
        const stream = query({
            prompt: this.queue as any,
            options: {
                permissionMode: "bypassPermissions",
                abortController: this.ac,
                ...(claudePath ? { pathToClaudeCodeExecutable: claudePath } : {}),
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
        try {
            for await (const msg of stream as AsyncIterable<Record<string, unknown>>) {
                if (msg.type === "assistant") {
                    const message = msg.message as { content: Array<Record<string, unknown>> };
                    for (const block of message?.content ?? []) {
                        if (block.type === "text") {
                            this.emitFn({ type: "text.delta", content: block.text as string });
                        } else if (block.type === "thinking") {
                            this.emitFn({ type: "thinking.delta", content: block.thinking as string });
                        } else if (block.type === "tool_use") {
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
                            const content = Array.isArray(block.content)
                                ? (block.content as Array<Record<string, unknown>>)
                                      .filter((c) => c.type === "text")
                                      .map((c) => c.text)
                                      .join("")
                                : String(block.content ?? "");
                            this.emitFn({
                                type: "tool.completed",
                                toolId: block.tool_use_id as string,
                                output: content,
                            });
                        }
                    }
                } else if (msg.type === "result") {
                    this.emitFn({ type: "turn.done" });
                }
            }
        } catch (e: unknown) {
            if (!this.ac.signal.aborted) {
                this.emitFn({ type: "error", message: String(e) });
            }
        }
    }

    private msg(content: string): SDKMsg {
        return { type: "user", message: { role: "user", content }, parent_tool_use_id: null };
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
