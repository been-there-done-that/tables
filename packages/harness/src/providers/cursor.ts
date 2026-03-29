// packages/harness/src/providers/cursor.ts
//
// CursorProvider spawns "cursor agent acp" and communicates via the
// Agent Client Protocol (ACP) over stdio (JSON-RPC 2.0, newline-delimited).
//
// ACP SessionUpdate variants we care about:
//   agent_message_chunk  → text.delta
//   agent_thought_chunk  → thinking.delta
//   tool_call            → tool.started
//   tool_call_update     → tool.started (progress) or tool.completed (done/failed)

import { spawn } from "child_process";
import { createInterface } from "readline";
import { tmpdir } from "os";
import {
    ClientSideConnection,
    type SessionNotification,
    type RequestPermissionRequest,
    type RequestPermissionResponse,
    type Client,
    PROTOCOL_VERSION,
    type AnyMessage,
} from "@agentclientprotocol/sdk";
import type { HarnessEvent, Session, SessionConfig } from "../types";

class AcpClientImpl implements Pick<Client, "sessionUpdate" | "requestPermission"> {
    constructor(private readonly onUpdate: (params: SessionNotification) => void) {}

    async sessionUpdate(params: SessionNotification): Promise<void> {
        this.onUpdate(params);
    }

    async requestPermission(params: RequestPermissionRequest): Promise<RequestPermissionResponse> {
        return { outcome: { outcome: "selected", optionId: params.options[0]?.optionId ?? "" } };
    }
}

function makeStdioStream(
    child: ReturnType<typeof spawn>,
): {
    stream: { readable: ReadableStream<AnyMessage>; writable: WritableStream<AnyMessage> };
    closed: Promise<void>;
} {
    let closeResolve!: () => void;
    const closed = new Promise<void>((r) => (closeResolve = r));

    const readable = new ReadableStream<AnyMessage>({
        start(controller) {
            const rl = createInterface({ input: child.stdout! });
            rl.on("line", (line) => {
                if (!line.trim()) return;
                try { controller.enqueue(JSON.parse(line) as AnyMessage); } catch { /* skip non-JSON */ }
            });
            rl.on("close", () => {
                try { controller.close(); } catch { /* already closed */ }
                closeResolve();
            });
        },
    });

    const writable = new WritableStream<AnyMessage>({
        write(message) {
            return new Promise<void>((resolve, reject) => {
                child.stdin!.write(JSON.stringify(message) + "\n", (err) => {
                    if (err) reject(err);
                    else resolve();
                });
            });
        },
        close() {
            child.stdin!.end();
        },
    });

    return { stream: { readable, writable }, closed };
}

export class CursorProvider implements Session {
    private emitFn: (e: HarnessEvent) => void = () => {};
    private connection: ClientSideConnection | null = null;
    private sessionId: string | null = null;
    private aborted = false;
    private initPromise: Promise<void> | null = null;
    private child: ReturnType<typeof spawn> | null = null;

    constructor(private config: SessionConfig) {}

    setEmit(fn: (e: HarnessEvent) => void) { this.emitFn = fn; }
    emitToolEvent(e: HarnessEvent) { this.emitFn(e); }

    stop() {
        this.aborted = true;
        this.child?.kill();
    }

    async isAvailable(): Promise<boolean> {
        try {
            const result = await Bun.$`which cursor`.quiet();
            return result.exitCode === 0;
        } catch {
            return false;
        }
    }

    private async _initialize(): Promise<void> {
        const child = spawn("cursor", ["agent", "acp"], {
            stdio: ["pipe", "pipe", "pipe"],
            cwd: tmpdir(),
            env: process.env,
        });
        this.child = child;

        child.stderr?.on("data", (chunk: Buffer) => {
            console.error(`[cursor-acp] ${chunk.toString().trim()}`);
        });

        child.on("exit", (code) => {
            if (!this.aborted && code !== null && code !== 0) {
                this.emitFn({ type: "error", message: `cursor agent acp exited with code ${code}` });
            }
        });

        const { stream, closed } = makeStdioStream(child);

        closed.then(() => {
            if (!this.aborted) this.emitFn({ type: "error", message: "Cursor ACP process closed unexpectedly" });
        });

        const client = new AcpClientImpl((params) => this.handleUpdate(params));
        this.connection = new ClientSideConnection((_agent) => client as Client, stream);

        await this.connection.initialize({
            protocolVersion: PROTOCOL_VERSION,
            clientCapabilities: {},
        });
        if (this.aborted) return;

        const sessionResult = await this.connection.newSession({
            cwd: tmpdir(),
            mcpServers: [],
        });
        if (this.aborted) return;

        this.sessionId = sessionResult.sessionId;
        this.emitFn({ type: "session.init", sdkSessionId: sessionResult.sessionId });

        if (this.config.systemPrompt) {
            await this.connection.prompt({
                sessionId: this.sessionId,
                prompt: [{ type: "text", text: this.config.systemPrompt }],
            });
        }
    }

    async send(text: string): Promise<void> {
        if (this.aborted) return;
        try {
            if (!this.connection) {
                if (!this.initPromise) this.initPromise = this._initialize();
                await this.initPromise;
            }
            if (this.aborted || !this.connection || !this.sessionId) return;

            const result = await this.connection.prompt({
                sessionId: this.sessionId,
                prompt: [{ type: "text", text }],
            });

            if (!this.aborted && result.stopReason !== "cancelled") {
                this.emitFn({ type: "turn.done" });
            }
        } catch (e) {
            if (!this.aborted) this.emitFn({ type: "error", message: String(e) });
        }
    }

    private handleUpdate(params: SessionNotification): void {
        const update = params.update;

        switch (update.sessionUpdate) {
            case "agent_message_chunk": {
                if (update.content.type === "text") {
                    this.emitFn({ type: "text.delta", content: update.content.text });
                }
                break;
            }
            case "agent_thought_chunk": {
                if (update.content.type === "text") {
                    this.emitFn({ type: "thinking.delta", content: update.content.text });
                }
                break;
            }
            case "tool_call": {
                this.emitFn({
                    type: "tool.started",
                    toolId: update.toolCallId,
                    toolName: update.title,
                    input: update.rawInput ?? null,
                });
                break;
            }
            case "tool_call_update": {
                const status = update.status;
                if (status === "completed" || status === "failed") {
                    const outputParts = (update.content ?? [])
                        .map((c) => {
                            if (c.type === "content" && c.content?.type === "text" && c.content.text) {
                                return c.content.text;
                            }
                            return "";
                        })
                        .filter(Boolean);
                    const output =
                        outputParts.length > 0
                            ? outputParts.join("")
                            : status === "failed"
                              ? "tool failed"
                              : String(update.rawOutput ?? "");
                    this.emitFn({ type: "tool.completed", toolId: update.toolCallId, output });
                } else if (status === "in_progress" && update.title) {
                    this.emitFn({
                        type: "tool.started",
                        toolId: update.toolCallId,
                        toolName: update.title,
                        input: update.rawInput ?? null,
                    });
                }
                break;
            }
            default:
                break;
        }
    }
}
