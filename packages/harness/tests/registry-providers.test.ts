import { describe, it, expect } from "bun:test";
import { createSession } from "../src/registry";
import type { SessionConfig } from "../src/types";

const baseConfig: SessionConfig = {
    sessionId: "test-session",
    threadId: "test-thread",
    systemPrompt: "you are a test assistant",
    provider: "claude",
};

const ALL_PROVIDERS = ["claude", "codex", "gemini", "opencode", "cursor"] as const;

describe("registry — all providers", () => {
    for (const provider of ALL_PROVIDERS) {
        it(`createSession with provider '${provider}' returns a Session-shaped object`, () => {
            const session = createSession({ ...baseConfig, provider });
            expect(typeof session.send).toBe("function");
            expect(typeof session.stop).toBe("function");
            expect(typeof session.setEmit).toBe("function");
            expect(typeof session.emitToolEvent).toBe("function");
        });
    }
});
