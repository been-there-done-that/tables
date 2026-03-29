import { describe, it, expect } from "bun:test";
import { callTool, resolveToolResult, cancelSessionTools } from "./tool-bridge";
import type { HarnessEvent } from "./types";

describe("ToolBridge", () => {
    it("resolves callTool when resolveToolResult is called", async () => {
        const emitted: HarnessEvent[] = [];
        const emit = (e: HarnessEvent) => emitted.push(e);

        const callPromise = callTool("session-1", "list_tables", { schema: "public" }, emit);

        // Simulate frontend posting result
        const started = emitted.find((e) => e.type === "tool.started");
        expect(started).toBeDefined();
        expect((started as any).toolName).toBe("list_tables");
        expect((started as any).requiresResponse).toBe(true);

        const requestId = (started as any).toolId;
        const resolved = resolveToolResult(requestId, [{ table_name: "users" }]);
        expect(resolved).toBe(true);

        const result = await callPromise;
        expect(result).toEqual([{ table_name: "users" }]);

        const completed = emitted.find((e) => e.type === "tool.completed");
        expect(completed).toBeDefined();
        expect((completed as any).toolId).toBe(requestId);
    });

    it("returns false from resolveToolResult for unknown requestId", () => {
        expect(resolveToolResult("nonexistent-id", {})).toBe(false);
    });

    it("cancelSessionTools rejects all pending tools for that session", async () => {
        const emit = (_e: HarnessEvent) => {};

        const p1 = callTool("session-cancel", "run_query", { sql: "SELECT 1" }, emit);
        const p2 = callTool("session-cancel", "list_tables", {}, emit);

        cancelSessionTools("session-cancel");

        const r1 = await p1;
        const r2 = await p2;
        expect((r1 as any).error).toContain("stopped");
        expect((r2 as any).error).toContain("stopped");
    });

    it("cancelSessionTools only affects the target session", async () => {
        const emittedA: HarnessEvent[] = [];
        const pA = callTool("session-A", "list_tables", {}, (e) => emittedA.push(e));

        cancelSessionTools("session-B");  // different session — should not cancel pA

        const startedA = emittedA.find((e) => e.type === "tool.started");
        const idA = (startedA as any).toolId;
        resolveToolResult(idA, { result: "ok" });

        const result = await pA;
        expect((result as any).result).toBe("ok");
    });
});
