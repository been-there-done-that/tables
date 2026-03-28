import { describe, it, expect } from "bun:test";

// Test the message routing logic in isolation without spawning a process.
// The classify function mirrors what JsonRpcAdapter.handleLine() does internally.
function classifyMessage(raw: string): { kind: "notification" | "request" | "response" | "invalid"; parsed?: Record<string, unknown> } {
    let parsed: unknown;
    try { parsed = JSON.parse(raw); } catch { return { kind: "invalid" }; }
    const m = parsed as Record<string, unknown>;
    const hasMethod = typeof m.method === "string";
    const hasId = "id" in m;
    if (hasMethod && !hasId) return { kind: "notification", parsed: m };
    if (hasMethod && hasId) return { kind: "request", parsed: m };
    if (!hasMethod && hasId) return { kind: "response", parsed: m };
    return { kind: "invalid" };
}

describe("JsonRpcAdapter message classification", () => {
    it("classifies notification (method, no id)", () => {
        const result = classifyMessage('{"method":"item/agentMessage/delta","params":{"delta":"hello"}}');
        expect(result.kind).toBe("notification");
        expect((result.parsed as any).method).toBe("item/agentMessage/delta");
    });

    it("classifies server request (method + id)", () => {
        const result = classifyMessage('{"method":"approval/request","id":1,"params":{}}');
        expect(result.kind).toBe("request");
    });

    it("classifies response (id, no method)", () => {
        const result = classifyMessage('{"id":1,"result":{"ok":true}}');
        expect(result.kind).toBe("response");
    });

    it("returns invalid for bad JSON", () => {
        const result = classifyMessage("not json");
        expect(result.kind).toBe("invalid");
    });

    it("returns invalid for JSON with no method and no id", () => {
        const result = classifyMessage('{"data":"orphan"}');
        expect(result.kind).toBe("invalid");
    });
});
