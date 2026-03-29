import { tmpdir } from "os";
import { JsonRpcAdapter } from "../adapters/jsonrpc-adapter";
import type { HarnessEvent, SessionConfig } from "../types";

// Only these item types represent actual tool calls — all others
// (agentMessage, userMessage, reasoning, plan, etc.) are content, not tools.
const TOOL_ITEM_TYPES = new Set([
    "commandExecution",
    "fileChange",
    "fileRead",
    "tool",
    "mcp_tool",
]);

// Codex app-server JSON-RPC notification method → HarnessEvent mapping
const NOTIFICATION_MAP: Record<string, (params: any) => HarnessEvent | null> = {
    // ── Text content ──────────────────────────────────────────────────────────
    "item/agentMessage/delta": (p) => ({
        type: "text.delta",
        content: p?.delta ?? "",
    }),

    // ── Reasoning / thinking ──────────────────────────────────────────────────
    "item/reasoning/textDelta": (p) => ({
        type: "thinking.delta",
        content: p?.delta ?? "",
    }),
    "item/reasoning/summaryTextDelta": (p) => ({
        type: "thinking.delta",
        content: p?.delta ?? "",
    }),

    // ── Live command output (stream while the command runs) ───────────────────
    "item/commandExecution/outputDelta": (p) => {
        const delta = p?.delta ?? "";
        const itemId = p?.itemId ?? p?.item?.id ?? "";
        if (!delta) return null;
        return {
            type: "tool.input_delta",
            toolId: itemId,
            toolName: "commandExecution",
            partialContent: delta,
        };
    },

    // ── Tool lifecycle ────────────────────────────────────────────────────────
    "item/started": (p) => {
        const item = p?.item;
        const itemType: string = item?.type ?? "";
        if (!itemType || !TOOL_ITEM_TYPES.has(itemType)) return null;

        // itemId: prefer params.itemId (top-level), fall back to item.id
        const toolId = p?.itemId ?? item?.id ?? "";

        // Extract meaningful input based on item type
        let input: unknown = {};
        if (itemType === "commandExecution") {
            // command field can be top-level in params or nested in item.params
            const command = p?.command ?? item?.params?.command ?? item?.command ?? "";
            if (command) input = { command };
        } else if (itemType === "fileChange" || itemType === "fileRead") {
            const path = p?.path ?? item?.params?.path ?? item?.path ?? "";
            if (path) input = { path };
        } else {
            input = item?.params ?? {};
        }

        return { type: "tool.started", toolId, toolName: itemType, input };
    },

    "item/completed": (p) => {
        const item = p?.item;
        const itemType: string = item?.type ?? "";
        if (!itemType || !TOOL_ITEM_TYPES.has(itemType)) return null;

        const toolId = p?.itemId ?? item?.id ?? "";
        const rawOutput = item?.output ?? p?.output ?? "";
        const output = typeof rawOutput === "string" ? rawOutput : JSON.stringify(rawOutput);

        return { type: "tool.completed", toolId, output };
    },

    // ── Turn / session ────────────────────────────────────────────────────────
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
        await this.sendRequest("initialize", {
            clientInfo: { name: "tables-harness", title: null, version: "1.0.0" },
            capabilities: { experimentalApi: false },
        });

        this.sendNotification("initialized");

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
        this.sendRequest("turn/start", {
            threadId: this.threadId,
            input: [{ type: "text", text, text_elements: [] }],
        }).catch((e) =>
            this.emitFn({ type: "error", message: `Codex send failed: ${String(e)}` })
        );
    }
}
