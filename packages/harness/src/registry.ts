import type { Session, SessionConfig } from "./types";
import { ClaudeProvider } from "./providers/claude";
import { CodexProvider } from "./providers/codex";
import { GeminiProvider } from "./providers/gemini";
import { OpenCodeProvider } from "./providers/opencode";
import { CursorProvider } from "./providers/cursor";

type ProviderFactory = (config: SessionConfig) => Session;

const PROVIDERS: Record<string, ProviderFactory> = {
    claude:   (c) => new ClaudeProvider(c),
    codex:    (c) => new CodexProvider(c),
    gemini:   (c) => new GeminiProvider(c),
    opencode: (c) => new OpenCodeProvider(c),
    cursor:   (c) => new CursorProvider(c),
};

export function createSession(config: SessionConfig): Session {
    const factory = PROVIDERS[config.provider];
    if (!factory) throw new Error(`Unknown provider: "${config.provider}"`);
    return factory(config);
}

export interface AvailableProvider {
    id: string;
    label: string;
    available: boolean;
}

export const PROVIDER_LABELS: Record<string, string> = {
    claude:   "Claude",
    gemini:   "Gemini",
    codex:    "Codex",
    opencode: "OpenCode",
    cursor:   "Cursor",
};

export async function checkAvailability(): Promise<AvailableProvider[]> {
    return Promise.all(
        Object.entries(PROVIDERS).map(async ([id, factory]) => {
            const instance = factory({ sessionId: "", threadId: "", systemPrompt: "", provider: id });
            const available = typeof (instance as any).isAvailable === "function"
                ? await (instance as any).isAvailable().catch(() => false)
                : false;
            instance.stop();
            return { id, label: PROVIDER_LABELS[id] ?? id, available };
        })
    );
}
