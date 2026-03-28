import { describe, it, expect } from "bun:test";
import { GeminiProvider } from "../src/providers/gemini";
import type { SessionConfig } from "../src/types";

const baseConfig: SessionConfig = {
    sessionId: "test-session",
    threadId: "test-thread",
    systemPrompt: "You are a helpful assistant.",
    provider: "gemini",
};

class TestableGemini extends GeminiProvider {
    exposeBuildPrompt(text: string): string {
        return (this as any).buildPrompt(text);
    }
    exposeHistory(): Array<{ role: string; text: string }> {
        return (this as any).history;
    }
}

describe("GeminiProvider.buildPrompt", () => {
    it("first turn with no system prompt: result equals just the user text", () => {
        const config: SessionConfig = { ...baseConfig, systemPrompt: "" };
        const gemini = new TestableGemini(config);

        const result = gemini.exposeBuildPrompt("Hello there");

        expect(result).toBe("Hello there");
    });

    it("first turn with system prompt: result contains system prompt + '---' separator + user text", () => {
        const gemini = new TestableGemini(baseConfig);

        const result = gemini.exposeBuildPrompt("Hello there");

        expect(result).toContain(baseConfig.systemPrompt);
        expect(result).toContain("---");
        expect(result).toContain("Hello there");
        // System prompt appears before separator, separator before user text
        const sysIndex = result.indexOf(baseConfig.systemPrompt);
        const sepIndex = result.indexOf("---");
        const userIndex = result.indexOf("Hello there");
        expect(sysIndex).toBeLessThan(sepIndex);
        expect(sepIndex).toBeLessThan(userIndex);
    });

    it("multi-turn: after pushing to history, buildPrompt includes prior conversation", () => {
        const gemini = new TestableGemini(baseConfig);

        // Simulate a completed prior turn by pushing directly to history
        (gemini as any).history.push({ role: "user", text: "First question" });
        (gemini as any).history.push({ role: "model", text: "First answer" });

        const result = gemini.exposeBuildPrompt("Second question");

        expect(result).toContain("User: First question");
        expect(result).toContain("Assistant: First answer");
        expect(result).toContain("Second question");
    });

    it("system prompt only appears on turn 0, not on subsequent turns", () => {
        const gemini = new TestableGemini(baseConfig);

        // Push one completed turn into history (simulating turn 0 already happened)
        (gemini as any).history.push({ role: "user", text: "First question" });
        (gemini as any).history.push({ role: "model", text: "First answer" });

        const result = gemini.exposeBuildPrompt("Second question");

        // System prompt should NOT appear because history.length > 0
        expect(result).not.toContain(baseConfig.systemPrompt);
        expect(result).not.toContain("---");
    });
});
