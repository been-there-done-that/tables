// packages/harness/src/providers/cursor.ts
//
// CursorProvider connects to Cursor's ACP endpoint (default ws://localhost:4747)
// and maps ACP SessionUpdate notifications to HarnessEvents.
//
// ACP SessionUpdate variants we care about:
//   agent_message_chunk  → text.delta
//   tool_call            → tool.started (status "in_progress" or "pending")
//   tool_call_update     → tool.started (progress) or tool.completed (done/failed)
//
// The turn.done event is emitted by AcpAdapter after connection.prompt() resolves.

import type { SessionNotification } from "@agentclientprotocol/sdk";
import { AcpAdapter } from "../adapters/acp-adapter";
import type { SessionConfig } from "../types";

export class CursorProvider extends AcpAdapter {
    constructor(config: SessionConfig) {
        super(config);
    }

    async isAvailable(): Promise<boolean> {
        // Cursor must be running with its ACP server active on port 4747.
        // A binary check isn't enough — the server may not be started.
        return new Promise((resolve) => {
            let done = false;
            const finish = (v: boolean) => { if (!done) { done = true; resolve(v); } };
            const timeout = setTimeout(() => finish(false), 500);
            Bun.connect({
                hostname: "127.0.0.1",
                port: 4747,
                socket: {
                    open(s) { clearTimeout(timeout); s.end(); finish(true); },
                    error() { clearTimeout(timeout); finish(false); },
                    close() {},
                    data() {},
                },
            }).catch(() => { clearTimeout(timeout); finish(false); });
        });
    }

    protected getEndpoint(): string {
        const ep = this.config.providerConfig?.acpEndpoint;
        return typeof ep === "string" ? ep : "ws://localhost:4747";
    }

    protected handleUpdate(params: SessionNotification): void {
        const update = params.update;

        switch (update.sessionUpdate) {
            case "agent_message_chunk": {
                // Content is a ContentBlock union; we only handle text
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
                // A new tool call is being created by the agent
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
                    // Collect text from tool call content if present
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

                    this.emitFn({
                        type: "tool.completed",
                        toolId: update.toolCallId,
                        output,
                    });
                } else if (status === "in_progress" && update.title) {
                    // Progress update for an existing tool — emit as started again
                    // so the UI can refresh the title/status
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
                // plan, user_message_chunk, available_commands_update, etc. — ignore
                break;
        }
    }
}
