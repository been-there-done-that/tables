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
