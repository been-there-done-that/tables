// packages/harness/src/providers/claude.ts
import { query } from "@anthropic-ai/claude-agent-sdk";
import { AsyncQueue } from "../adapters/async-queue";
import { SdkAdapter } from "../adapters/sdk-adapter";
import type { SessionConfig } from "../types";

type SDKMsg = {
    type: "user";
    session_id: string;
    parent_tool_use_id: null;
    message: { role: "user"; content: Array<{ type: "text"; text: string }> };
};

export class ClaudeProvider extends SdkAdapter {
    private queue = new AsyncQueue<SDKMsg>();
    private firstMessage = true;
    private systemPrompt: string;

    constructor(config: SessionConfig) {
        super();
        this.systemPrompt = config.systemPrompt;

        const claudePath =
            Bun.which("claude") ??
            (Bun.env.HOME ? `${Bun.env.HOME}/.claude/local/claude` : null) ??
            "/usr/local/bin/claude";

        const cwd = `${Bun.env.HOME ?? ""}/.config/tables/sessions/${config.threadId}`;
        Bun.spawnSync(["mkdir", "-p", cwd]);

        const childEnv = { ...process.env };
        delete childEnv.CLAUDECODE;
        delete childEnv.CLAUDE_CODE_ENTRYPOINT;
        delete childEnv.CLAUDE_CODE_VERSION;

        const stream = query({
            prompt: this.queue as any,
            options: {
                permissionMode: "bypassPermissions",
                abortController: this.ac,
                env: childEnv,
                includePartialMessages: true,
                ...(claudePath ? { pathToClaudeCodeExecutable: claudePath } : {}),
                ...(config.model ? { model: config.model } : {}),
                ...(config.effort && config.effort !== "auto" ? { effort: config.effort } : {}),
                cwd,
                ...(config.sdkSessionId ? { resume: config.sdkSessionId } : {}),
                persistSession: true,
            } as any,
        });

        this.consume(stream);
    }

    send(text: string) {
        const content = this.firstMessage
            ? `${this.systemPrompt}\n\n---\n\n${text}`
            : text;
        this.firstMessage = false;
        this.queue.push({
            type: "user",
            session_id: "",
            parent_tool_use_id: null,
            message: { role: "user", content: [{ type: "text", text: content }] },
        });
    }

    override stop() {
        super.stop();
        this.queue.close();
    }
}
