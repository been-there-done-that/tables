import type { Session, HarnessEvent, SessionConfig } from "../types";

export class GeminiProvider implements Session {
    private history: Array<{ role: "user" | "model"; text: string }> = [];
    private emitFn: (e: HarnessEvent) => void = () => {};
    private aborted = false;
    private currentProcess: ReturnType<typeof Bun.spawn> | null = null;

    constructor(private config: SessionConfig) {}

    setEmit(fn: (e: HarnessEvent) => void) {
        this.emitFn = fn;
    }

    emitToolEvent(e: HarnessEvent) {
        this.emitFn(e);
    }

    stop() {
        this.aborted = true;
        this.currentProcess?.kill();
    }

    async send(text: string): Promise<void> {
        if (this.aborted) return;

        // Emit session.init on the first send call
        if (this.history.length === 0) {
            this.emitFn({ type: "session.init", sdkSessionId: crypto.randomUUID() });
        }

        // Build the full prompt: system prompt (first turn only) + conversation history + new message
        const fullPrompt = this.buildPrompt(text);

        const args = ["-p", fullPrompt];
        if (this.config.model) args.unshift("--model", this.config.model);

        const proc = Bun.spawn(["gemini", ...args], {
            stdout: "pipe",
            stderr: "pipe",
        });
        this.currentProcess = proc;

        let responseText = "";

        try {
            // Read stdout line by line, emit as text.delta
            const reader = proc.stdout.getReader();
            const decoder = new TextDecoder();
            while (true) {
                const { done, value } = await reader.read();
                if (done || this.aborted) break;
                const chunk = decoder.decode(value, { stream: true });
                responseText += chunk;
                this.emitFn({ type: "text.delta", content: chunk });
            }

            await proc.exited;

            if (this.aborted) return;

            // Record the exchange in history for multi-turn context
            this.history.push({ role: "user", text });
            if (responseText.trim()) {
                this.history.push({ role: "model", text: responseText.trim() });
            }

            if (proc.exitCode !== 0) {
                const errBytes = await new Response(proc.stderr).arrayBuffer();
                const errText = new TextDecoder().decode(errBytes).trim();
                this.emitFn({
                    type: "error",
                    message: errText || `gemini exited with code ${proc.exitCode}`,
                });
                return;
            }

            this.emitFn({ type: "turn.done" });
        } catch (e: unknown) {
            if (!this.aborted) {
                this.emitFn({ type: "error", message: String(e) });
            }
        } finally {
            this.currentProcess = null;
        }
    }

    private buildPrompt(newText: string): string {
        const parts: string[] = [];

        // Prepend system prompt before the first user message only
        if (this.history.length === 0 && this.config.systemPrompt) {
            parts.push(this.config.systemPrompt);
            parts.push("---");
        }

        // Include conversation history
        for (const turn of this.history) {
            parts.push(turn.role === "user" ? `User: ${turn.text}` : `Assistant: ${turn.text}`);
        }

        // Add the new user message
        parts.push(newText);

        return parts.join("\n\n");
    }
}
