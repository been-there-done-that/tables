import { describe, it, expect } from "bun:test";
import { spawn } from "child_process";
import { createInterface } from "readline";

// ── Inline the stream-json parsing logic from GeminiProvider ──

type HarnessEvent =
    | { type: "text.delta"; content: string }
    | { type: "thinking.delta"; content: string }
    | { type: "session.init"; sdkSessionId: string }
    | { type: "turn.done" }
    | { type: "error"; message: string };

interface GeminiStreamLine {
    type: string;
    role?: string;
    content?: string;
    delta?: boolean;
    status?: string;
    error?: string;
}

function parseStreamLine(line: string): HarnessEvent | null {
    let event: GeminiStreamLine;
    try { event = JSON.parse(line); } catch { return null; }

    if (event.type === "message" && event.role === "assistant" && event.content) {
        return { type: "text.delta", content: event.content };
    }
    if (event.type === "thinking" && event.content) {
        return { type: "thinking.delta", content: event.content };
    }
    if (event.type === "result") {
        if (event.status === "error" && event.error) {
            return { type: "error", message: event.error };
        }
        if (event.status === "success") {
            return { type: "turn.done" };
        }
    }
    return null;
}

// ────────────────────────────────────────────────────────────────────────────

describe("Gemini stream-json parser", () => {
    it("maps assistant message delta to text.delta", () => {
        const line = JSON.stringify({ type: "message", role: "assistant", content: "Hello!", delta: true });
        expect(parseStreamLine(line)).toEqual({ type: "text.delta", content: "Hello!" });
    });

    it("ignores user message lines", () => {
        const line = JSON.stringify({ type: "message", role: "user", content: "Hi", delta: true });
        expect(parseStreamLine(line)).toBeNull();
    });

    it("maps thinking delta to thinking.delta", () => {
        const line = JSON.stringify({ type: "thinking", content: "Let me reason..." });
        expect(parseStreamLine(line)).toEqual({ type: "thinking.delta", content: "Let me reason..." });
    });

    it("maps result success to turn.done", () => {
        const line = JSON.stringify({ type: "result", status: "success", stats: { total_tokens: 100 } });
        expect(parseStreamLine(line)).toEqual({ type: "turn.done" });
    });

    it("maps result error to error event", () => {
        const line = JSON.stringify({ type: "result", status: "error", error: "rate limit exceeded" });
        expect(parseStreamLine(line)).toEqual({ type: "error", message: "rate limit exceeded" });
    });

    it("ignores init events", () => {
        const line = JSON.stringify({ type: "init", session_id: "abc", model: "gemini-2.5-flash" });
        expect(parseStreamLine(line)).toBeNull();
    });

    it("ignores tool_call events", () => {
        const line = JSON.stringify({ type: "tool_call", name: "list_directory", input: {} });
        expect(parseStreamLine(line)).toBeNull();
    });

    it("returns null for non-JSON lines", () => {
        expect(parseStreamLine("not json")).toBeNull();
        expect(parseStreamLine("")).toBeNull();
        expect(parseStreamLine("  ")).toBeNull();
    });

    it("returns null for assistant message with empty content", () => {
        const line = JSON.stringify({ type: "message", role: "assistant", content: "" });
        expect(parseStreamLine(line)).toBeNull();
    });

    it("returns null for thinking with no content", () => {
        const line = JSON.stringify({ type: "thinking" });
        expect(parseStreamLine(line)).toBeNull();
    });
});

describe("Gemini CLI availability", () => {
    it("gemini binary is in PATH", async () => {
        const result = await Bun.$`which gemini`.quiet().nothrow();
        expect(result.exitCode).toBe(0);
    });

    it("gemini -p with stream-json produces valid events for a simple prompt", async () => {
        const events: HarnessEvent[] = [];

        await new Promise<void>((resolve, reject) => {
            const child = spawn(
                "gemini",
                ["--model", "gemini-2.5-flash", "--yolo", "--output-format", "stream-json", "-p", "Reply with only the word: pong"],
                { stdio: ["ignore", "pipe", "pipe"] }
            );

            const rl = createInterface({ input: child.stdout! });
            rl.on("line", (line) => {
                const ev = parseStreamLine(line);
                if (ev) events.push(ev);
            });

            const timeout = setTimeout(() => {
                child.kill();
                reject(new Error("gemini CLI timed out after 30s"));
            }, 30_000);

            child.on("close", () => {
                clearTimeout(timeout);
                resolve();
            });
        });

        const textDeltas = events.filter((e) => e.type === "text.delta");
        const turnDone = events.find((e) => e.type === "turn.done");

        expect(textDeltas.length).toBeGreaterThan(0);
        expect(turnDone).toBeDefined();

        const fullText = textDeltas.map((e) => (e as any).content).join("").toLowerCase();
        expect(fullText).toContain("pong");
    }, 35_000);
});
