import { describe, it, expect } from "bun:test";
import { CodexProvider } from "../src/providers/codex";
import type { HarnessEvent, SessionConfig } from "../src/types";

const baseConfig: SessionConfig = {
    sessionId: "test-session",
    threadId: "test-thread",
    systemPrompt: "you are a test assistant",
    provider: "codex",
};

/**
 * Create a CodexProvider and immediately wire up a recorder.
 * The constructor spawns a process (which may fail) and fires init() async —
 * both are irrelevant to handleNotification, which is a pure sync mapping.
 */
function makeProvider(): { provider: CodexProvider; events: HarnessEvent[] } {
    const events: HarnessEvent[] = [];
    const provider = new CodexProvider(baseConfig);
    provider.setEmit((e) => events.push(e));
    return { provider, events };
}

describe("CodexProvider.handleNotification", () => {
    it("item/agentMessage/delta maps to text.delta with correct content", () => {
        const { provider, events } = makeProvider();

        (provider as any).handleNotification("item/agentMessage/delta", { delta: "Hello world" });

        const textEvents = events.filter((e) => e.type === "text.delta");
        expect(textEvents.length).toBe(1);
        expect((textEvents[0] as Extract<HarnessEvent, { type: "text.delta" }>).content).toBe("Hello world");
    });

    it("item/thinkingMessage/delta maps to thinking.delta with correct content", () => {
        const { provider, events } = makeProvider();

        (provider as any).handleNotification("item/thinkingMessage/delta", { delta: "Deep thought" });

        const thinkEvents = events.filter((e) => e.type === "thinking.delta");
        expect(thinkEvents.length).toBe(1);
        expect((thinkEvents[0] as Extract<HarnessEvent, { type: "thinking.delta" }>).content).toBe("Deep thought");
    });

    it("thread/turn/complete maps to turn.done", () => {
        const { provider, events } = makeProvider();

        (provider as any).handleNotification("thread/turn/complete", {});

        const doneEvents = events.filter((e) => e.type === "turn.done");
        expect(doneEvents.length).toBe(1);
    });

    it("unknown method emits no event", () => {
        const { provider, events } = makeProvider();
        // Clear any events from the constructor's async init (error events from missing binary)
        events.length = 0;

        (provider as any).handleNotification("some/unknown/method", { data: "whatever" });

        expect(events.length).toBe(0);
    });
});
