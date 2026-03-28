export interface ProviderModel {
    id: string;
    label: string;
}

export interface ProviderConfig {
    label: string;
    models: ProviderModel[];
    supportsModel: boolean;
    supportsEffort: boolean;
}

export const PROVIDER_CONFIGS: Record<string, ProviderConfig> = {
    claude: {
        label: "Claude",
        models: [
            { id: "claude-haiku-4-5-20251001", label: "Haiku 4.5" },
            { id: "claude-sonnet-4-6",         label: "Sonnet 4.6" },
            { id: "claude-opus-4-6",           label: "Opus 4.6" },
        ],
        supportsModel: true,
        supportsEffort: true,
    },
    gemini: {
        label: "Gemini",
        models: [
            { id: "gemini-2.5-pro",   label: "2.5 Pro" },
            { id: "gemini-2.0-flash", label: "2.0 Flash" },
        ],
        supportsModel: true,
        supportsEffort: false,
    },
    codex: {
        label: "Codex",
        models: [],
        supportsModel: false,
        supportsEffort: false,
    },
    opencode: {
        label: "OpenCode",
        models: [],
        supportsModel: false,
        supportsEffort: false,
    },
    cursor: {
        label: "Cursor",
        models: [],
        supportsModel: false,
        supportsEffort: false,
    },
};

/** Returns the default model ID for a provider, or empty string if none. */
export function defaultModel(provider: string): string {
    return PROVIDER_CONFIGS[provider]?.models[0]?.id ?? "";
}
