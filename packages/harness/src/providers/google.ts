import { createGoogleGenerativeAI } from "@ai-sdk/google";
import { AiSdkSession } from "../session/ai-sdk-session";
import type { SessionConfig } from "../types";

export class GoogleProvider extends AiSdkSession {
    constructor(config: SessionConfig) {
        const apiKey = (config.providerConfig?.apiKey as string) ?? "";
        const google = createGoogleGenerativeAI({ apiKey });
        const modelId = config.model ?? "gemini-2.5-flash";
        super(google(modelId), config);
    }

    override async isAvailable(): Promise<boolean> {
        return true; // Frontend checks for API key; harness always reports available
    }
}
