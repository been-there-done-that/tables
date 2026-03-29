// packages/harness/src/types.ts

export type HarnessEvent =
    | { type: "text.delta"; content: string }
    | { type: "thinking.delta"; content: string }
    | { type: "tool.started"; toolId: string; toolName: string; input: unknown; requiresResponse?: boolean }
    | { type: "tool.completed"; toolId: string; output: string }
    | { type: "tool.input_delta"; toolId: string; toolName: string; partialContent: string }
    | { type: "session.init"; sdkSessionId: string }
    | { type: "turn.done" }
    | { type: "error"; message: string };

export interface Session {
    setEmit(fn: (e: HarnessEvent) => void): void;
    emitToolEvent(e: HarnessEvent): void;
    send(text: string): void | Promise<void>;
    isAvailable(): Promise<boolean>;
    stop(): void;
}

export interface SessionConfig {
    sessionId: string;
    threadId: string;
    systemPrompt: string;
    provider: string;
    providerConfig?: Record<string, unknown>;
    model?: string;
    effort?: "auto" | "low" | "medium" | "high" | "max";
    sdkSessionId?: string; // resume only
}
