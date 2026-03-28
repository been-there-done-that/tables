import { JsonRpcAdapter } from "../adapters/jsonrpc-adapter";
import type { HarnessEvent, SessionConfig } from "../types";

// Codex JSON-RPC notification method → HarnessEvent mapping
const NOTIFICATION_MAP: Record<string, (params: any) => HarnessEvent | null> = {
    "item/agentMessage/delta": (p) => ({
        type: "text.delta",
        content: p?.delta ?? "",
    }),
    "item/thinkingMessage/delta": (p) => ({
        type: "thinking.delta",
        content: p?.delta ?? "",
    }),
    "item/toolCall/start": (p) => ({
        type: "tool.started",
        toolId: p?.id ?? "",
        toolName: p?.name ?? "",
        input: p?.input ?? {},
    }),
    "item/toolCall/complete": (p) => ({
        type: "tool.completed",
        toolId: p?.id ?? "",
        output: typeof p?.output === "string" ? p.output : JSON.stringify(p?.output ?? ""),
    }),
    "thread/turn/complete": () => ({ type: "turn.done" }),
    "session/ready": (p) => ({
        type: "session.init",
        sdkSessionId: p?.sessionId ?? "",
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
        // 1. Send initialize request
        await this.sendRequest("initialize", {
            clientInfo: { name: "tables-harness", version: "1.0.0" },
        });

        // 2. Send initialized notification
        this.sendNotification("initialized");

        // 3. Verify account access
        await this.sendRequest("account/read", {});

        // 4. Open a thread for this session
        const response = await this.sendRequest<{ threadId: string }>("thread/start", {
            cwd: `${Bun.env.HOME ?? ""}/.config/tables/sessions/${this.config.threadId}`,
            ...(this.config.model ? { model: this.config.model } : {}),
        });
        this.threadId = response.threadId;
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

    send(text: string) {
        if (!this.ready || !this.threadId) {
            this.emitFn({ type: "error", message: "Codex session not ready" });
            return;
        }
        this.sendRequest("thread/message", {
            threadId: this.threadId,
            content: text,
            systemPrompt: this.config.systemPrompt,
        }).catch((e) =>
            this.emitFn({ type: "error", message: `Codex send failed: ${String(e)}` })
        );
    }
}
