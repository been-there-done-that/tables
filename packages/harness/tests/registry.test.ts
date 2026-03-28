import { describe, it, expect } from "bun:test";
import { createSession } from "../src/registry";
import type { SessionConfig } from "../src/types";

const baseConfig: SessionConfig = {
    sessionId: "test-session",
    threadId: "test-thread",
    systemPrompt: "you are a test assistant",
    provider: "claude",
};

describe("registry", () => {
    it("throws on unknown provider", () => {
        expect(() => createSession({ ...baseConfig, provider: "unknown-provider" }))
            .toThrow('Unknown provider: "unknown-provider"');
    });

    it("returns a Session for provider 'claude'", () => {
        const session = createSession({ ...baseConfig, provider: "claude" });
        expect(typeof session.send).toBe("function");
        expect(typeof session.stop).toBe("function");
        expect(typeof session.setEmit).toBe("function");
        expect(typeof session.emitToolEvent).toBe("function");
    });
});
