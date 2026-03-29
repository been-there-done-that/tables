// Integration tests — require claude, codex, and gemini CLI tools to be installed.
// Run with: bun test packages/harness/tests/integration.test.ts

import { describe, it, expect } from "bun:test";
import { createSession } from "../src/registry";
import type { HarnessEvent, SessionConfig } from "../src/types";

const BASE_CONFIG: SessionConfig = {
    sessionId: crypto.randomUUID(),
    threadId: crypto.randomUUID(),
    systemPrompt: "",
    provider: "claude",
};

/**
 * Create a session, collect all events until turn.done or error,
 * then call stop(). Returns the collected events.
 */
async function runSession(
    provider: string,
    prompt: string,
    options: { timeoutMs?: number; waitForReadyMs?: number } = {},
): Promise<HarnessEvent[]> {
    const { timeoutMs = 60_000, waitForReadyMs = 0 } = options;

    const config: SessionConfig = {
        ...BASE_CONFIG,
        sessionId: crypto.randomUUID(),
        provider,
    };
    const session = createSession(config);

    return new Promise((resolve, reject) => {
        const events: HarnessEvent[] = [];

        const timer = setTimeout(() => {
            session.stop();
            reject(new Error(`[${provider}] Timeout after ${timeoutMs}ms. Events so far: ${JSON.stringify(events)}`));
        }, timeoutMs);

        session.setEmit((e) => {
            events.push(e);
            if (e.type === "turn.done" || e.type === "error") {
                clearTimeout(timer);
                session.stop();
                resolve(events);
            }
        });

        // Some providers need time to init before send()
        if (waitForReadyMs > 0) {
            setTimeout(() => session.send(prompt), waitForReadyMs);
        } else {
            session.send(prompt);
        }
    });
}

describe("integration", () => {
    it("claude: responds to a simple prompt", async () => {
        const events = await runSession("claude", "Reply with exactly the word PONG and nothing else.");
        const textEvents = events.filter(e => e.type === "text.delta");
        const fullText = textEvents.map(e => (e as any).content).join("");
        expect(events.some(e => e.type === "turn.done")).toBe(true);
        expect(fullText.toLowerCase()).toContain("pong");
    }, { timeout: 90_000 });

    it.skip("google: responds to a simple prompt (requires GOOGLE_API_KEY)", async () => {
        const events = await runSession("google", "Reply with exactly the word PONG and nothing else.");
        const textEvents = events.filter(e => e.type === "text.delta");
        const fullText = textEvents.map(e => (e as any).content).join("");
        expect(events.some(e => e.type === "turn.done")).toBe(true);
        expect(fullText.toLowerCase()).toContain("pong");
    }, { timeout: 60_000 });

    it.skip("google: maintains multi-turn context (requires GOOGLE_API_KEY)", async () => {
        const config: SessionConfig = { ...BASE_CONFIG, sessionId: crypto.randomUUID(), provider: "google" };
        const session = createSession(config);
        const allEvents: HarnessEvent[] = [];
        let turnResolve!: () => void;

        const awaitTurn = () => new Promise<void>((r) => (turnResolve = r));

        session.setEmit((e) => {
            allEvents.push(e);
            if (e.type === "turn.done" || e.type === "error") turnResolve();
        });

        // Turn 1: simple math question
        let turn = awaitTurn();
        session.send("What is 17 multiplied by 3? Reply with only the number.");
        await turn;

        // Turn 2: reference the previous answer
        turn = awaitTurn();
        session.send("Add 9 to the number you just gave me. Reply with only the number.");
        await turn;
        session.stop();

        const fullText = allEvents
            .filter(e => e.type === "text.delta")
            .map(e => (e as any).content)
            .join("");
        // 17 * 3 = 51, 51 + 9 = 60
        expect(fullText).toContain("60");
    }, { timeout: 120_000 });

    it("codex: responds to a simple prompt", async () => {
        const events = await runSession(
            "codex",
            "Reply with exactly the word PONG and nothing else.",
            { timeoutMs: 90_000, waitForReadyMs: 5_000 }
        );
        const textEvents = events.filter(e => e.type === "text.delta");
        const fullText = textEvents.map(e => (e as any).content).join("");
        expect(events.some(e => e.type === "turn.done")).toBe(true);
        expect(fullText.toLowerCase()).toContain("pong");
    }, { timeout: 100_000 });
});
