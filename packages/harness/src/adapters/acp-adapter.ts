// packages/harness/src/adapters/acp-adapter.ts
//
// Base adapter for ACP (Agent Client Protocol) providers.
//
// The SDK's ClientSideConnection works over a Stream (writable/readable of
// AnyMessage objects). For WebSocket endpoints we build a custom Stream by
// bridging the WS message events into ReadableStream/WritableStream.
//
// Session lifecycle:
//   1. connect()   – opens WS, creates ClientSideConnection, negotiates init
//   2. send(text)  – creates/reuses a session, calls connection.prompt()
//   3. stop()      – aborts in-flight prompt, closes WS

import {
    ClientSideConnection,
    type SessionNotification,
    type RequestPermissionRequest,
    type RequestPermissionResponse,
    type Client,
    PROTOCOL_VERSION,
} from "@agentclientprotocol/sdk";
import type { AnyMessage } from "@agentclientprotocol/sdk";
import type { Stream } from "@agentclientprotocol/sdk";
import type { HarnessEvent, Session, SessionConfig } from "../types";

/**
 * Minimal Client implementation passed to ClientSideConnection.
 * sessionUpdate() is the only required method — it is where the agent
 * streams all output back to us.
 */
class AcpClientImpl implements Pick<Client, "sessionUpdate" | "requestPermission"> {
    constructor(private readonly onUpdate: (params: SessionNotification) => void) {}

    async sessionUpdate(params: SessionNotification): Promise<void> {
        this.onUpdate(params);
    }

    // Permission requests: auto-allow so the harness is non-interactive
    async requestPermission(params: RequestPermissionRequest): Promise<RequestPermissionResponse> {
        const first = params.options[0];
        return {
            outcome: {
                outcome: "selected",
                optionId: first?.optionId ?? "",
            },
        };
    }
}

/**
 * Build a WebSocket-backed ACP Stream.
 *
 * The ACP SDK's Stream is `{ writable: WritableStream<AnyMessage>, readable: ReadableStream<AnyMessage> }`.
 * We bridge a native WebSocket into those streams:
 *   - readable: enqueue JSON-parsed messages from ws.onmessage
 *   - writable: JSON.stringify + ws.send on each write
 *
 * Bun's global WebSocket follows the browser API so no extra packages needed.
 */
function makeWebSocketStream(ws: WebSocket): {
    stream: Stream;
    opened: Promise<void>;
    closed: Promise<void>;
} {
    let openResolve!: () => void;
    let closeResolve!: () => void;
    let readableController!: ReadableStreamDefaultController<AnyMessage>;

    const opened = new Promise<void>((r) => (openResolve = r));
    const closed = new Promise<void>((r) => (closeResolve = r));

    const readable = new ReadableStream<AnyMessage>({
        start(controller) {
            readableController = controller;
        },
        cancel() {
            ws.close();
        },
    });

    ws.onopen = () => openResolve();

    ws.onmessage = (event) => {
        try {
            const msg = JSON.parse(
                typeof event.data === "string"
                    ? event.data
                    : new TextDecoder().decode(event.data as ArrayBuffer),
            ) as AnyMessage;
            readableController.enqueue(msg);
        } catch {
            // ignore unparseable frames
        }
    };

    ws.onerror = () => {
        // Connection errors surface via onclose
    };

    ws.onclose = () => {
        try { readableController.close(); } catch { /* already closed */ }
        closeResolve();
    };

    const writable = new WritableStream<AnyMessage>({
        write(message) {
            if (ws.readyState === WebSocket.OPEN) {
                ws.send(JSON.stringify(message));
            }
        },
        close() {
            ws.close();
        },
    });

    return { stream: { readable, writable }, opened, closed };
}

export abstract class AcpAdapter implements Session {
    protected emitFn: (e: HarnessEvent) => void = () => {};
    protected config: SessionConfig;

    private ws: WebSocket | null = null;
    private connection: ClientSideConnection | null = null;
    private sessionId: string | null = null;
    private aborted = false;
    private closeWs: (() => void) | null = null;

    constructor(config: SessionConfig) {
        this.config = config;
    }

    setEmit(fn: (e: HarnessEvent) => void) {
        this.emitFn = fn;
    }

    emitToolEvent(e: HarnessEvent) {
        this.emitFn(e);
    }

    stop() {
        this.aborted = true;
        this.closeWs?.();
        this.ws?.close();
    }

    /**
     * Called for each sessionUpdate notification from the agent.
     * Subclasses map ACP updates to HarnessEvents.
     */
    protected abstract handleUpdate(params: SessionNotification): void;

    /**
     * The ACP endpoint WebSocket URL, e.g. "ws://localhost:4747"
     */
    protected abstract getEndpoint(): string;

    /**
     * Connect, initialize the ACP session, and send the first prompt.
     * Subsequent send() calls reuse the same session.
     */
    async send(text: string): Promise<void> {
        if (this.aborted) return;

        try {
            // Establish WS + ClientSideConnection on first message
            if (!this.connection) {
                const endpoint = this.getEndpoint();
                const ws = new WebSocket(endpoint);
                this.ws = ws;

                const { stream, opened, closed } = makeWebSocketStream(ws);
                this.closeWs = () => ws.close();

                // Wait for WS handshake before talking ACP
                await opened;
                if (this.aborted) return;

                const client = new AcpClientImpl((params) => this.handleUpdate(params));
                this.connection = new ClientSideConnection((_agent) => client as Client, stream);

                // Close the connection when WS drops
                closed.then(() => {
                    if (!this.aborted) {
                        this.emitFn({ type: "error", message: "ACP WebSocket closed unexpectedly" });
                    }
                });

                // Protocol negotiation
                await this.connection.initialize({
                    protocolVersion: PROTOCOL_VERSION,
                    clientCapabilities: {},
                });
                if (this.aborted) return;

                // Send system prompt as the first user message if we have one,
                // then the actual user text. ACP doesn't have a dedicated system
                // prompt method on the session — we create the session first.
                const sessionResult = await this.connection.newSession({
                    cwd: process.cwd(),
                    mcpServers: [],
                });
                this.sessionId = sessionResult.sessionId;
                this.emitFn({ type: "session.init", sdkSessionId: sessionResult.sessionId });

                // Send system prompt as context before the real message, if present
                if (this.config.systemPrompt) {
                    await this.connection.prompt({
                        sessionId: this.sessionId,
                        prompt: [{ type: "text", text: this.config.systemPrompt }],
                    });
                    if (this.aborted) return;
                }
            }

            if (!this.connection || !this.sessionId) return;

            // Send the user message and wait for the turn to complete
            const result = await this.connection.prompt({
                sessionId: this.sessionId,
                prompt: [{ type: "text", text }],
            });

            if (!this.aborted) {
                // Map stop reasons to turn.done
                if (
                    result.stopReason === "end_turn" ||
                    result.stopReason === "max_tokens" ||
                    result.stopReason === "max_turn_requests"
                ) {
                    this.emitFn({ type: "turn.done" });
                } else if (result.stopReason === "cancelled") {
                    // aborted by caller — no event needed
                } else {
                    this.emitFn({ type: "turn.done" });
                }
            }
        } catch (e: unknown) {
            if (!this.aborted) {
                this.emitFn({ type: "error", message: String(e) });
            }
        }
    }
}
