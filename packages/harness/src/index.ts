import { createSession, checkAvailability } from "./registry";
import type { Session } from "./types";
import { unlinkSync } from "fs";

const sessions = new Map<string, Session>();
const pendingToolResults = new Map<string, { sessionId: string; resolve: (v: unknown) => void; reject: (e: Error) => void }>();

const CORS = {
    "Access-Control-Allow-Origin": "*",
    "Access-Control-Allow-Methods": "GET, POST, OPTIONS",
    "Access-Control-Allow-Headers": "Content-Type",
};

// Unix socket path — used by sandboxed agents (e.g. Codex) that can't reach TCP localhost.
// Exported via HARNESS_SOCK so the system prompt can reference it.
export const SOCKET_PATH = "/tmp/tables-harness.sock";

async function handleRequest(req: Request): Promise<Response> {
        const url = new URL(req.url);

        if (req.method === "OPTIONS") {
            return new Response(null, { status: 204, headers: CORS });
        }

        if (req.method === "POST" && url.pathname === "/session/start") {
            const { sessionId, threadId, systemPrompt, model, effort, provider = "claude", providerConfig } = (await req.json()) as {
                sessionId: string;
                threadId: string;
                systemPrompt: string;
                model?: string;
                effort?: "auto" | "low" | "medium" | "high" | "max";
                provider?: string;
                providerConfig?: Record<string, unknown>;
            };
            sessions.get(sessionId)?.stop();
            sessions.set(sessionId, createSession({ sessionId, threadId, systemPrompt, provider, providerConfig, model, effort }));
            console.error(`[harness] session started: ${sessionId}`);
            return Response.json({ ok: true }, { headers: CORS });
        }

        if (req.method === "POST" && url.pathname === "/session/send") {
            const { sessionId, text } = (await req.json()) as {
                sessionId: string;
                text: string;
            };
            const session = sessions.get(sessionId);
            if (!session) {
                return Response.json({ error: "session not found" }, { status: 404, headers: CORS });
            }

            console.error(`[harness] session send: ${sessionId} — "${text.slice(0, 60)}"`);

            const encoder = new TextEncoder();
            let controller!: ReadableStreamDefaultController<Uint8Array>;

            const stream = new ReadableStream<Uint8Array>({
                start(c) { controller = c; },
                cancel() {
                    console.error(`[harness] SSE stream cancelled for ${sessionId}`);
                    // Silence the current turn — its remaining events should not
                    // bleed into the next /session/send SSE stream.
                    session.setEmit(() => {});
                    // Immediately reject any tool calls waiting for frontend results
                    // so the SDK turn unblocks quickly instead of hitting the 30s timeout.
                    for (const [reqId, pending] of pendingToolResults) {
                        if (pending.sessionId === sessionId) {
                            pendingToolResults.delete(reqId);
                            pending.reject(new Error("Turn stopped by user"));
                        }
                    }
                },
            });

            session.setEmit((e) => {
                console.error(`[harness] emit → ${e.type}${"content" in e ? ` "${(e as any).content?.slice?.(0, 20)}"` : ""}`);
                try {
                    controller.enqueue(encoder.encode(`data: ${JSON.stringify(e)}\n\n`));
                    if (e.type === "turn.done" || e.type === "error") {
                        controller.close();
                    }
                } catch {
                    // SSE stream already closed (client disconnected) — ignore
                }
            });

            console.error(`[harness] calling session.send for ${sessionId}`);
            session.send(text);

            return new Response(stream, {
                headers: {
                    ...CORS,
                    "Content-Type": "text/event-stream",
                    "Cache-Control": "no-cache",
                    "X-Accel-Buffering": "no",
                },
            });
        }

        if (req.method === "POST" && url.pathname === "/session/stop") {
            const { sessionId } = (await req.json()) as { sessionId: string };
            sessions.get(sessionId)?.stop();
            sessions.delete(sessionId);
            console.error(`[harness] session stopped: ${sessionId}`);
            return Response.json({ ok: true }, { headers: CORS });
        }

        if (req.method === "POST" && url.pathname === "/session/resume") {
            const { sessionId, threadId, sdkSessionId, systemPrompt, model, effort, provider = "claude", providerConfig } = (await req.json()) as {
                sessionId: string;
                threadId: string;
                sdkSessionId: string;
                systemPrompt: string;
                model?: string;
                effort?: "auto" | "low" | "medium" | "high" | "max";
                provider?: string;
                providerConfig?: Record<string, unknown>;
            };
            sessions.get(sessionId)?.stop();
            sessions.set(sessionId, createSession({ sessionId, threadId, systemPrompt, provider, providerConfig, model, effort, sdkSessionId }));
            console.error(`[harness] session resumed: ${sessionId} sdk: ${sdkSessionId}`);
            return Response.json({ ok: true }, { headers: CORS });
        }

        // POST /db/:sessionId/:toolName — agent calls this via curl/Bash tool
        if (req.method === "POST" && url.pathname.startsWith("/db/")) {
            const parts = url.pathname.split("/"); // ["", "db", sessionId, toolName]
            const pathSessionId = parts[2];
            const toolName = parts[3];

            if (!pathSessionId || !toolName) {
                return Response.json({ error: "invalid path" }, { status: 400, headers: CORS });
            }

            const session = sessions.get(pathSessionId);
            if (!session) {
                return Response.json({ error: "session not found" }, { status: 404, headers: CORS });
            }

            const input = await req.json().catch(() => ({}));
            const requestId = crypto.randomUUID();

            console.error(`[harness] /db/ received — tool="${toolName}" session="${pathSessionId}" requestId="${requestId}" input=${JSON.stringify(input)}`);

            // Emit tool.started to frontend via current SSE stream.
            // requiresResponse:true signals the frontend must POST /tool-result/:requestId
            // to unblock this pending curl call. Provider-internal tool events never set this.
            console.error(`[harness] emitting tool.started requiresResponse=true for requestId="${requestId}"`);
            session.emitToolEvent({ type: "tool.started", toolId: requestId, toolName, input, requiresResponse: true });
            console.error(`[harness] tool.started emitted — now waiting for /tool-result/${requestId}`);

            // Hold the request open until frontend POSTs the result
            const result = await new Promise<unknown>((resolve, reject) => {
                pendingToolResults.set(requestId, { sessionId: pathSessionId, resolve, reject });
                const timeoutMs = 30_000;
                setTimeout(() => {
                    if (pendingToolResults.has(requestId)) {
                        console.error(`[harness] tool "${toolName}" (${requestId}) TIMED OUT after ${timeoutMs / 1000}s`);
                        pendingToolResults.delete(requestId);
                        reject(new Error(`Tool "${toolName}" timed out after ${timeoutMs / 1000}s`));
                    }
                }, timeoutMs);
            }).catch((e) => ({ error: String(e) }));

            console.error(`[harness] /tool-result resolved for requestId="${requestId}" result=${JSON.stringify(result)?.slice(0, 200)}`);

            // Emit tool.completed to frontend
            session.emitToolEvent({
                type: "tool.completed",
                toolId: requestId,
                output: typeof result === "string" ? result : JSON.stringify(result),
            });

            return Response.json(result, { headers: CORS });
        }

        // POST /tool-result/:requestId — frontend submits tool execution result
        if (req.method === "POST" && url.pathname.startsWith("/tool-result/")) {
            const requestId = url.pathname.slice("/tool-result/".length);
            console.error(`[harness] /tool-result/${requestId} received — pending count=${pendingToolResults.size}`);
            const pending = pendingToolResults.get(requestId);
            if (!pending) {
                console.error(`[harness] /tool-result/${requestId} — NO MATCHING PENDING (already timed out or wrong id)`);
                return Response.json({ error: "no pending tool for this id" }, { status: 404, headers: CORS });
            }
            pendingToolResults.delete(requestId);
            const body = await req.json().catch(() => ({}));
            console.error(`[harness] /tool-result resolving requestId="${requestId}" body=${JSON.stringify(body)?.slice(0, 200)}`);
            pending.resolve(body);
            return Response.json({ ok: true }, { headers: CORS });
        }

        if (req.method === "GET" && url.pathname === "/providers") {
            const providers = await checkAvailability();
            return Response.json(providers, { headers: CORS });
        }

    return new Response("harness ok", { headers: CORS });
}

// TCP server — used by the frontend (SSE streams, session management, /providers).
const server = Bun.serve({
    port: 0,
    idleTimeout: 0,
    fetch: handleRequest,
});

// Unix socket server — used by sandboxed agents (e.g. Codex) that cannot reach TCP localhost.
// Silently remove any stale socket file from a previous run.
try { unlinkSync(SOCKET_PATH); } catch {}
Bun.serve({
    unix: SOCKET_PATH,
    fetch: handleRequest,
});

console.log(`HARNESS_PORT=${server.port}`);
