import { tmpdir } from "os";
import { JsonRpcAdapter } from "../adapters/jsonrpc-adapter";
import type { HarnessEvent, SessionConfig } from "../types";

// Codex app-server JSON-RPC notification method → HarnessEvent mapping
// Method names sourced from `codex app-server generate-ts` (v0.117.0)
const NOTIFICATION_MAP: Record<string, (params: any) => HarnessEvent | null> = {
    "item/agentMessage/delta": (p) => ({
        type: "text.delta",
        content: p?.delta ?? "",
    }),
    "item/reasoning/textDelta": (p) => ({
        type: "thinking.delta",
        content: p?.delta ?? "",
    }),
    "item/reasoning/summaryTextDelta": (p) => ({
        type: "thinking.delta",
        content: p?.delta ?? "",
    }),
    "item/started": (p) => {
        // Only emit tool.started for non-message items (file changes, commands)
        if (p?.item?.type && p.item.type !== "message") {
            return {
                type: "tool.started",
                toolId: p.item.id ?? "",
                toolName: p.item.type ?? "",
                input: p.item.params ?? {},
            };
        }
        return null;
    },
    "item/completed": (p) => {
        if (p?.item?.type && p.item.type !== "message") {
            return {
                type: "tool.completed",
                toolId: p.item.id ?? "",
                output: typeof p.item.output === "string" ? p.item.output : JSON.stringify(p.item.output ?? ""),
            };
        }
        return null;
    },
    "turn/completed": () => ({ type: "turn.done" }),
    "thread/started": (p) => ({
        type: "session.init",
        sdkSessionId: p?.thread?.id ?? "",
    }),
};

export class CodexProvider extends JsonRpcAdapter {
    private threadId: string | null = null;
    private config: SessionConfig;
    private ready = false;

    constructor(config: SessionConfig) {
        const codexPath = Bun.which("codex") ?? "codex";
        super(codexPath, ["app-server"]);
        this.config = config;
        this.init().catch((e) =>
            this.emitFn({ type: "error", message: `Codex init failed: ${String(e)}` })
        );
    }

    private async init() {
        // 1. Send initialize with capabilities
        await this.sendRequest("initialize", {
            clientInfo: { name: "tables-harness", title: null, version: "1.0.0" },
            capabilities: { experimentalApi: false },
        });

        // 2. Confirm initialization
        this.sendNotification("initialized");

        // 3. Start a thread — system prompt goes in developerInstructions
        //    experimentalRawEvents and persistExtendedHistory are required fields
        const response = await this.sendRequest<{ thread: { id: string } }>("thread/start", {
            cwd: tmpdir(),
            developerInstructions: this.config.systemPrompt || undefined,
            approvalPolicy: "never",
            experimentalRawEvents: false,
            persistExtendedHistory: false,
            ephemeral: true,
            ...(this.config.model ? { model: this.config.model } : {}),
        });
        this.threadId = response.thread.id;
        this.ready = true;
    }

    protected handleNotification(method: string, params: unknown) {
        const mapper = NOTIFICATION_MAP[method];
        if (mapper) {
            const event = mapper(params);
            if (event) this.emitFn(event);
        }
    }

    protected handleServerRequest(id: unknown, _method: string, _params: unknown) {
        // Auto-approve all agent requests (command execution, file changes)
        this.respondToRequest(id, { approved: true });
    }

    async isAvailable(): Promise<boolean> {
        try {
            const result = await Bun.$`which codex`.quiet();
            return result.exitCode === 0;
        } catch {
            return false;
        }
    }

    send(text: string) {
        if (!this.ready || !this.threadId) {
            this.emitFn({ type: "error", message: "Codex session not ready" });
            return;
        }
        // turn/start sends a user message and starts a new agent turn
        this.sendRequest("turn/start", {
            threadId: this.threadId,
            input: [{ type: "text", text, text_elements: [] }],
        }).catch((e) =>
            this.emitFn({ type: "error", message: `Codex send failed: ${String(e)}` })
        );
    }
}
