import { spawn } from "child_process";
import { createInterface } from "readline";
import type { HarnessEvent, Session } from "../types";

interface PendingRequest {
    resolve: (value: unknown) => void;
    reject: (error: Error) => void;
    timeout: ReturnType<typeof setTimeout>;
}

export abstract class JsonRpcAdapter implements Session {
    protected emitFn: (e: HarnessEvent) => void = () => {};
    protected child: ReturnType<typeof spawn>;
    private pending = new Map<string, PendingRequest>();
    private nextId = 1;

    constructor(binaryPath: string, args: string[], env?: NodeJS.ProcessEnv) {
        this.child = spawn(binaryPath, args, {
            stdio: ["pipe", "pipe", "pipe"],
            env: env ?? process.env,
            shell: process.platform === "win32",
        });

        const rl = createInterface({ input: this.child.stdout! });
        rl.on("line", (line) => this.handleLine(line));

        this.child.stderr?.on("data", (chunk: Buffer) => {
            console.error(`[jsonrpc] stderr: ${chunk.toString().trim()}`);
        });

        this.child.on("exit", (code) => {
            if (code !== 0) {
                this.emitFn({ type: "error", message: `Provider process exited with code ${code}` });
            }
        });
    }

    setEmit(fn: (e: HarnessEvent) => void) {
        this.emitFn = fn;
    }

    emitToolEvent(e: HarnessEvent) {
        this.emitFn(e);
    }

    private handleLine(line: string) {
        if (!line.trim()) return;
        let msg: unknown;
        try { msg = JSON.parse(line); } catch { return; }
        const m = msg as Record<string, unknown>;
        const hasMethod = typeof m.method === "string";
        const hasId = "id" in m;
        if (hasMethod && !hasId) this.handleNotification(m.method as string, m.params);
        else if (hasMethod && hasId) this.handleServerRequest(m.id, m.method as string, m.params);
        else if (!hasMethod && hasId) this.handleResponse(String(m.id), m.result, m.error);
    }

    private handleResponse(id: string, result: unknown, error: unknown) {
        const pending = this.pending.get(id);
        if (!pending) return;
        clearTimeout(pending.timeout);
        this.pending.delete(id);
        if (error) pending.reject(new Error(JSON.stringify(error)));
        else pending.resolve(result);
    }

    protected sendRequest<T = unknown>(method: string, params: unknown, timeoutMs = 20_000): Promise<T> {
        const id = this.nextId++;
        return new Promise<T>((resolve, reject) => {
            const timeout = setTimeout(() => {
                this.pending.delete(String(id));
                reject(new Error(`Timeout waiting for response to "${method}"`));
            }, timeoutMs);
            this.pending.set(String(id), { resolve: resolve as (v: unknown) => void, reject, timeout });
            this.write({ jsonrpc: "2.0", method, id, params });
        });
    }

    protected sendNotification(method: string, params?: unknown) {
        this.write({ jsonrpc: "2.0", method, params });
    }

    protected respondToRequest(id: unknown, result: unknown) {
        this.write({ jsonrpc: "2.0", id, result });
    }

    private write(msg: unknown) {
        if (this.child.stdin?.writable) {
            this.child.stdin.write(JSON.stringify(msg) + "\n");
        }
    }

    protected abstract handleNotification(method: string, params: unknown): void;
    protected abstract handleServerRequest(id: unknown, method: string, params: unknown): void;
    abstract send(text: string): void;

    stop() {
        this.child.kill();
    }
}
