import { createSession, checkAvailability } from "./registry";
import type { Session } from "./types";
import { unlinkSync } from "fs";
import { callTool, resolveToolResult, cancelSessionTools } from "./tool-bridge";

const sessions = new Map<string, Session>();

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
                    session.setEmit(() => {});
                    cancelSessionTools(sessionId);
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
            console.error(`[harness] /db/ received — tool="${toolName}" session="${pathSessionId}" input=${JSON.stringify(input)}`);

            const result = await callTool(
                pathSessionId,
                toolName,
                input,
                (e) => session.emitToolEvent(e),
            );

            return Response.json(result, { headers: CORS });
        }

        // POST /tool-result/:requestId — frontend submits tool execution result
        if (req.method === "POST" && url.pathname.startsWith("/tool-result/")) {
            const requestId = url.pathname.slice("/tool-result/".length);
            console.error(`[harness] /tool-result/${requestId} received`);
            const body = await req.json().catch(() => ({}));
            const resolved = resolveToolResult(requestId, body);
            if (!resolved) {
                console.error(`[harness] /tool-result/${requestId} — no matching pending tool`);
                return Response.json({ error: "no pending tool for this id" }, { status: 404, headers: CORS });
            }
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
