import type { Session, HarnessEvent, SessionConfig } from "../types";
export class GeminiProvider implements Session {
    constructor(_config: SessionConfig) {}
    setEmit(_fn: (e: HarnessEvent) => void) {}
    emitToolEvent(_e: HarnessEvent) {}
    send(_text: string) {}
    stop() {}
}
