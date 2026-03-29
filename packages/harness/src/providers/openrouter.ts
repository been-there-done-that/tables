import { createOpenRouter } from "@openrouter/ai-sdk-provider";
import { AiSdkSession } from "../session/ai-sdk-session";
import type { SessionConfig } from "../types";

export class OpenRouterProvider extends AiSdkSession {
    constructor(config: SessionConfig) {
        const apiKey = (config.providerConfig?.apiKey as string) ?? "";
        const openrouter = createOpenRouter({ apiKey });
        const modelId = config.model ?? "openai/gpt-4o";
        super(openrouter.chat(modelId), config);
    }

    override async isAvailable(): Promise<boolean> {
        return true; // Frontend checks for API key; harness always reports available
    }
}
