import { tmpdir } from "os";
import { spawn } from "child_process";
import { createInterface } from "readline";
import type { Session, HarnessEvent, SessionConfig } from "../types";

// stream-json event shapes we care about
interface GeminiMessageEvent {
    type: "message";
    role: "assistant" | "user";
    content: string;
    delta?: boolean;
}
interface GeminiThinkingEvent {
    type: "thinking";
    content: string;
    delta?: boolean;
}
interface GeminiResultEvent {
    type: "result";
    status: "success" | "error";
    error?: string;
}
type GeminiStreamEvent = GeminiMessageEvent | GeminiThinkingEvent | GeminiResultEvent | { type: string };

export class GeminiProvider implements Session {
    private history: Array<{ role: "user" | "model"; text: string }> = [];
    private emitFn: (e: HarnessEvent) => void = () => {};
    private aborted = false;
    private currentChild: ReturnType<typeof spawn> | null = null;

    constructor(private config: SessionConfig) {}

    setEmit(fn: (e: HarnessEvent) => void) {
        this.emitFn = fn;
    }

    emitToolEvent(e: HarnessEvent) {
        this.emitFn(e);
    }

    stop() {
        this.aborted = true;
        this.currentChild?.kill();
    }

    async send(text: string): Promise<void> {
        if (this.aborted) return;

        if (this.history.length === 0) {
            this.emitFn({ type: "session.init", sdkSessionId: crypto.randomUUID() });
        }

        const fullPrompt = this.buildPrompt(text);

        const args = ["--yolo", "--output-format", "stream-json", "-p", fullPrompt];
        if (this.config.model) args.unshift("--model", this.config.model);

        const child = spawn("gemini", args, {
            stdio: ["ignore", "pipe", "pipe"],
            cwd: tmpdir(),
            env: process.env,
        });
        this.currentChild = child;

        let responseText = "";

        child.stderr?.on("data", (chunk: Buffer) => {
            console.error(`[gemini] stderr: ${chunk.toString().trim()}`);
        });

        return new Promise<void>((resolve) => {
            const rl = createInterface({ input: child.stdout! });

            rl.on("line", (line) => {
                if (!line.trim() || this.aborted) return;
                let event: GeminiStreamEvent;
                try { event = JSON.parse(line) as GeminiStreamEvent; } catch { return; }

                if (event.type === "message") {
                    const msg = event as GeminiMessageEvent;
                    if (msg.role === "assistant" && msg.content) {
                        responseText += msg.content;
                        this.emitFn({ type: "text.delta", content: msg.content });
                    }
                } else if (event.type === "thinking") {
                    const t = event as GeminiThinkingEvent;
                    if (t.content) {
                        this.emitFn({ type: "thinking.delta", content: t.content });
                    }
                } else if (event.type === "result") {
                    const result = event as GeminiResultEvent;
                    if (result.status === "error" && result.error) {
                        if (!this.aborted) this.emitFn({ type: "error", message: result.error });
                    }
                }
            });

            child.on("close", (code) => {
                this.currentChild = null;
                if (this.aborted) { resolve(); return; }

                if (code !== 0 && responseText === "") {
                    this.emitFn({ type: "error", message: `gemini exited with code ${code}` });
                } else {
                    this.history.push({ role: "user", text });
                    if (responseText.trim()) {
                        this.history.push({ role: "model", text: responseText.trim() });
                    }
                    this.emitFn({ type: "turn.done" });
                }
                resolve();
            });
        });
    }

    async isAvailable(): Promise<boolean> {
        try {
            const result = await Bun.$`which gemini`.quiet();
            return result.exitCode === 0;
        } catch {
            return false;
        }
    }

    private buildPrompt(newText: string): string {
        const parts: string[] = [];

        if (this.history.length === 0 && this.config.systemPrompt) {
            parts.push(this.config.systemPrompt);
            parts.push("---");
        }

        for (const turn of this.history) {
            parts.push(turn.role === "user" ? `User: ${turn.text}` : `Assistant: ${turn.text}`);
        }

        parts.push(newText);
        return parts.join("\n\n");
    }
}
