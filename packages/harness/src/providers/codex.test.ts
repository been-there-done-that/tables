import { describe, it, expect } from "bun:test";

// ── Inline the notification map logic so we can test it without spawning Codex ──

const TOOL_ITEM_TYPES = new Set([
    "commandExecution",
    "fileChange",
    "fileRead",
    "tool",
    "mcp_tool",
]);

type HarnessEvent =
    | { type: "text.delta"; content: string }
    | { type: "thinking.delta"; content: string }
    | { type: "tool.started"; toolId: string; toolName: string; input: unknown }
    | { type: "tool.completed"; toolId: string; output: string }
    | { type: "tool.input_delta"; toolId: string; toolName: string; partialContent: string }
    | { type: "session.init"; sdkSessionId: string }
    | { type: "turn.done" };

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
    "item/commandExecution/outputDelta": (p) => {
        const delta = p?.delta ?? "";
        const itemId = p?.itemId ?? p?.item?.id ?? "";
        if (!delta) return null;
        return { type: "tool.input_delta", toolId: itemId, toolName: "commandExecution", partialContent: delta };
    },
    "item/started": (p) => {
        const item = p?.item;
        const itemType: string = item?.type ?? "";
        if (!itemType || !TOOL_ITEM_TYPES.has(itemType)) return null;
        const toolId = p?.itemId ?? item?.id ?? "";
        let input: unknown = {};
        if (itemType === "commandExecution") {
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
    "turn/completed": () => ({ type: "turn.done" }),
    "thread/started": (p) => ({
        type: "session.init",
        sdkSessionId: p?.thread?.id ?? "",
    }),
};

function notify(method: string, params: unknown): HarnessEvent | null {
    return NOTIFICATION_MAP[method]?.(params) ?? null;
}

// ────────────────────────────────────────────────────────────────────────────

describe("Codex NOTIFICATION_MAP — text content", () => {
    it("maps item/agentMessage/delta to text.delta", () => {
        const event = notify("item/agentMessage/delta", { delta: "Hello " });
        expect(event).toEqual({ type: "text.delta", content: "Hello " });
    });

    it("maps item/reasoning/textDelta to thinking.delta", () => {
        const event = notify("item/reasoning/textDelta", { delta: "thinking..." });
        expect(event).toEqual({ type: "thinking.delta", content: "thinking..." });
    });

    it("maps item/reasoning/summaryTextDelta to thinking.delta", () => {
        const event = notify("item/reasoning/summaryTextDelta", { delta: "summary" });
        expect(event).toEqual({ type: "thinking.delta", content: "summary" });
    });
});

describe("Codex NOTIFICATION_MAP — non-tool items are filtered out", () => {
    const contentTypes = ["agentMessage", "userMessage", "reasoning", "plan", "message"];

    for (const type of contentTypes) {
        it(`filters out item/started for type "${type}"`, () => {
            const event = notify("item/started", {
                itemId: "id-1",
                item: { id: "id-1", type },
            });
            expect(event).toBeNull();
        });

        it(`filters out item/completed for type "${type}"`, () => {
            const event = notify("item/completed", {
                itemId: "id-1",
                item: { id: "id-1", type, output: "some output" },
            });
            expect(event).toBeNull();
        });
    }
});

describe("Codex NOTIFICATION_MAP — commandExecution tool", () => {
    it("emits tool.started with command from top-level params.command", () => {
        const event = notify("item/started", {
            itemId: "cmd-1",
            command: "curl http://127.0.0.1:9000/db/sess/run_query",
            item: { id: "cmd-1", type: "commandExecution" },
        });
        expect(event).toEqual({
            type: "tool.started",
            toolId: "cmd-1",
            toolName: "commandExecution",
            input: { command: "curl http://127.0.0.1:9000/db/sess/run_query" },
        });
    });

    it("emits tool.started with command from item.params.command", () => {
        const event = notify("item/started", {
            itemId: "cmd-2",
            item: { id: "cmd-2", type: "commandExecution", params: { command: "ls -la" } },
        });
        expect(event).toEqual({
            type: "tool.started",
            toolId: "cmd-2",
            toolName: "commandExecution",
            input: { command: "ls -la" },
        });
    });

    it("emits tool.started with command from item.command", () => {
        const event = notify("item/started", {
            item: { id: "cmd-3", type: "commandExecution", command: "psql -c 'SELECT 1'" },
        });
        expect(event).toEqual({
            type: "tool.started",
            toolId: "cmd-3",
            toolName: "commandExecution",
            input: { command: "psql -c 'SELECT 1'" },
        });
    });

    it("emits tool.started with empty input when no command field exists", () => {
        const event = notify("item/started", {
            item: { id: "cmd-4", type: "commandExecution" },
        });
        expect(event).toEqual({
            type: "tool.started",
            toolId: "cmd-4",
            toolName: "commandExecution",
            input: {},
        });
    });

    it("emits tool.completed with output from item.output", () => {
        const event = notify("item/completed", {
            itemId: "cmd-1",
            item: { id: "cmd-1", type: "commandExecution", output: "id | name\n1  | Alice" },
        });
        expect(event).toEqual({
            type: "tool.completed",
            toolId: "cmd-1",
            output: "id | name\n1  | Alice",
        });
    });

    it("emits tool.completed with output from top-level params.output", () => {
        const event = notify("item/completed", {
            itemId: "cmd-2",
            output: "exit code 0",
            item: { id: "cmd-2", type: "commandExecution" },
        });
        expect(event).toEqual({
            type: "tool.completed",
            toolId: "cmd-2",
            output: "exit code 0",
        });
    });

    it("emits tool.completed with empty string when no output", () => {
        const event = notify("item/completed", {
            item: { id: "cmd-3", type: "commandExecution" },
        });
        expect(event).toEqual({ type: "tool.completed", toolId: "cmd-3", output: "" });
    });

    it("streams live output via item/commandExecution/outputDelta", () => {
        const event = notify("item/commandExecution/outputDelta", {
            itemId: "cmd-1",
            delta: "row 1\n",
        });
        expect(event).toEqual({
            type: "tool.input_delta",
            toolId: "cmd-1",
            toolName: "commandExecution",
            partialContent: "row 1\n",
        });
    });

    it("returns null from outputDelta when delta is empty", () => {
        const event = notify("item/commandExecution/outputDelta", { itemId: "cmd-1", delta: "" });
        expect(event).toBeNull();
    });
});

describe("Codex NOTIFICATION_MAP — fileChange / fileRead tools", () => {
    it("emits tool.started for fileChange with path from top-level", () => {
        const event = notify("item/started", {
            itemId: "fc-1",
            path: "/tmp/out.sql",
            item: { id: "fc-1", type: "fileChange" },
        });
        expect(event).toEqual({
            type: "tool.started",
            toolId: "fc-1",
            toolName: "fileChange",
            input: { path: "/tmp/out.sql" },
        });
    });

    it("emits tool.started for fileRead with path from item.params.path", () => {
        const event = notify("item/started", {
            item: { id: "fr-1", type: "fileRead", params: { path: "/etc/hosts" } },
        });
        expect(event).toEqual({
            type: "tool.started",
            toolId: "fr-1",
            toolName: "fileRead",
            input: { path: "/etc/hosts" },
        });
    });
});

describe("Codex NOTIFICATION_MAP — toolId resolution", () => {
    it("prefers params.itemId over item.id", () => {
        const event = notify("item/started", {
            itemId: "top-level-id",
            item: { id: "nested-id", type: "commandExecution", command: "echo hi" },
        });
        expect((event as any).toolId).toBe("top-level-id");
    });

    it("falls back to item.id when params.itemId absent", () => {
        const event = notify("item/started", {
            item: { id: "nested-id", type: "commandExecution" },
        });
        expect((event as any).toolId).toBe("nested-id");
    });
});

describe("Codex NOTIFICATION_MAP — session / turn", () => {
    it("maps turn/completed to turn.done", () => {
        expect(notify("turn/completed", {})).toEqual({ type: "turn.done" });
    });

    it("maps thread/started to session.init", () => {
        const event = notify("thread/started", { thread: { id: "thread-abc" } });
        expect(event).toEqual({ type: "session.init", sdkSessionId: "thread-abc" });
    });

    it("returns null for unknown methods", () => {
        expect(notify("some/unknown/method", {})).toBeNull();
    });
});
