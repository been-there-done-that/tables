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

    it("item/reasoning/textDelta maps to thinking.delta with correct content", () => {
        const { provider, events } = makeProvider();

        (provider as any).handleNotification("item/reasoning/textDelta", { delta: "Deep thought" });

        const thinkEvents = events.filter((e) => e.type === "thinking.delta");
        expect(thinkEvents.length).toBe(1);
        expect((thinkEvents[0] as Extract<HarnessEvent, { type: "thinking.delta" }>).content).toBe("Deep thought");
    });

    it("turn/completed maps to turn.done", () => {
        const { provider, events } = makeProvider();

        (provider as any).handleNotification("turn/completed", {});

        const doneEvents = events.filter((e) => e.type === "turn.done");
        expect(doneEvents.length).toBe(1);
    });

    it("thread/started maps to session.init with thread id", () => {
        const { provider, events } = makeProvider();

        (provider as any).handleNotification("thread/started", { thread: { id: "thread-abc-123" } });

        const initEvents = events.filter((e) => e.type === "session.init");
        expect(initEvents.length).toBe(1);
        expect((initEvents[0] as Extract<HarnessEvent, { type: "session.init" }>).sdkSessionId).toBe("thread-abc-123");
    });

    it("unknown method emits no event", () => {
        const { provider, events } = makeProvider();
        // Clear any events from the constructor's async init (error events from missing binary)
        events.length = 0;

        (provider as any).handleNotification("some/unknown/method", { data: "whatever" });

        expect(events.length).toBe(0);
    });
});
