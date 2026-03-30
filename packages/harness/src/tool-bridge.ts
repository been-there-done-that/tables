import type { HarnessEvent } from "./types";
import { hLog } from "./logger";

type PendingTool = {
    sessionId: string;
    resolve: (v: unknown) => void;
    reject: (e: Error) => void;
};

const pending = new Map<string, PendingTool>();

/**
 * Called by AiSdkSession (and the /db/ HTTP handler) to dispatch a tool call.
 * Emits tool.started to SSE, waits for the frontend to POST /tool-result/:requestId,
 * then emits tool.completed and returns the result.
 */
export async function callTool(
    sessionId: string,
    toolName: string,
    input: unknown,
    emitFn: (e: HarnessEvent) => void,
): Promise<unknown> {
    const requestId = crypto.randomUUID();
    hLog("info", "bridge", `callTool tool="${toolName}" requestId="${requestId}" session="${sessionId}"`);

    emitFn({ type: "tool.started", toolId: requestId, toolName, input, requiresResponse: true });

    const result = await new Promise<unknown>((resolve, reject) => {
        pending.set(requestId, { sessionId, resolve, reject });
        setTimeout(() => {
            if (pending.has(requestId)) {
                pending.delete(requestId);
                hLog("error", "bridge", `tool "${toolName}" (${requestId}) timed out after 30s`);
                reject(new Error(`Tool "${toolName}" timed out after 30s`));
            }
        }, 30_000);
    }).catch((e) => ({ error: String(e) }));

    emitFn({
        type: "tool.completed",
        toolId: requestId,
        output: typeof result === "string" ? result : JSON.stringify(result),
    });

    return result;
}

/**
 * Called by the /tool-result/:requestId HTTP handler.
 * Returns true if a matching pending tool was found and resolved.
 */
export function resolveToolResult(requestId: string, result: unknown): boolean {
    const p = pending.get(requestId);
    if (!p) return false;
    pending.delete(requestId);
    hLog("debug", "bridge", `resolveToolResult requestId="${requestId}"`);
    p.resolve(result);
    return true;
}

/**
 * Called when a session's SSE stream is cancelled (user stops the turn).
 * Rejects all pending tool calls for the given session.
 */
export function cancelSessionTools(sessionId: string): void {
    let cancelled = 0;
    for (const [reqId, p] of pending) {
        if (p.sessionId === sessionId) {
            pending.delete(reqId);
            p.reject(new Error("Turn stopped by user"));
            cancelled++;
        }
    }
    if (cancelled > 0) hLog("info", "bridge", `cancelled ${cancelled} pending tools for session="${sessionId}"`);
}
