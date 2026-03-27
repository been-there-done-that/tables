import { harnessStore } from "$lib/stores/harness.svelte";

export type AgentEventType =
    | { type: "text.delta"; content: string }
    | { type: "thinking.delta"; content: string }
    | { type: "tool.started"; toolId: string; toolName: string; input: unknown }
    | { type: "tool.completed"; toolId: string; output: string }
    | { type: "tool.input_delta"; toolId: string; toolName: string; partialContent: string }
    | { type: "turn.done"; isError: boolean }
    | { type: "error"; message: string };

export interface AgentSession {
    send: (text: string) => void;
    abort: () => void;
    sessionId: string;
    port: number;
}

function waitForPort(timeoutMs: number): Promise<number> {
    return new Promise((resolve, reject) => {
        if (harnessStore.port !== null) {
            resolve(harnessStore.port);
            return;
        }
        const deadline = setTimeout(() => {
            reject(new Error("Harness not ready — is the sidecar running?"));
        }, timeoutMs);
        const poll = setInterval(() => {
            if (harnessStore.port !== null) {
                clearTimeout(deadline);
                clearInterval(poll);
                resolve(harnessStore.port!);
            }
        }, 500);
    });
}

export async function startAgentSession(opts: {
    systemPrompt: string;
    sessionId?: string;
    onEvent: (event: AgentEventType) => void;
    abortController: AbortController;
}): Promise<AgentSession> {
    const port = await waitForPort(10_000);
    const base = `http://127.0.0.1:${port}`;
    const sessionId = opts.sessionId ?? crypto.randomUUID();

    console.log(`[claude] starting session ${sessionId} on port ${port}`);
    const startRes = await fetch(`${base}/session/start`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ sessionId, systemPrompt: opts.systemPrompt }),
    });
    if (!startRes.ok) {
        throw new Error(`Failed to start harness session: ${startRes.status}`);
    }
    console.log(`[claude] session started ok`);

    let currentReader: ReadableStreamDefaultReader<Uint8Array> | null = null;

    const stop = () => {
        currentReader?.cancel();
        currentReader = null;
        fetch(`${base}/session/stop`, {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({ sessionId }),
        }).catch(() => {});
    };

    opts.abortController.signal.addEventListener("abort", stop);

    async function consumeSSE(response: Response) {
        console.log("[claude] consumeSSE started");
        const reader = response.body!.getReader();
        currentReader = reader;
        const decoder = new TextDecoder();
        let buffer = "";
        let eventCount = 0;
        try {
            while (true) {
                const { done, value } = await reader.read();
                if (done) { console.log(`[claude] SSE stream done, total events: ${eventCount}`); break; }
                buffer += decoder.decode(value, { stream: true });
                const parts = buffer.split("\n\n");
                buffer = parts.pop() ?? "";
                for (const part of parts) {
                    if (part.startsWith("data: ")) {
                        const event = JSON.parse(part.slice(6)) as AgentEventType;
                        eventCount++;
                        console.log(`[claude] event #${eventCount}: ${event.type}${"content" in event ? ` "${(event as any).content?.slice?.(0, 20)}"` : ""}`);
                        opts.onEvent(event);
                        await Promise.resolve();
                    }
                }
            }
        } catch (e) {
            if (!opts.abortController.signal.aborted) {
                opts.onEvent({ type: "error", message: String(e) });
            }
        } finally {
            currentReader = null;
        }
    }

    return {
        sessionId,
        port,
        send(text: string) {
            console.log(`[claude] send → "${text.slice(0, 40)}"`);
            fetch(`${base}/session/send`, {
                method: "POST",
                headers: { "Content-Type": "application/json" },
                body: JSON.stringify({ sessionId, text }),
            })
                .then((res) => {
                    if (!res.ok) throw new Error(`/session/send returned ${res.status}`);
                    return consumeSSE(res);
                })
                .catch((e) => {
                    if (!opts.abortController.signal.aborted) {
                        opts.onEvent({ type: "error", message: String(e) });
                    }
                });
        },
        abort: stop,
    };
}
