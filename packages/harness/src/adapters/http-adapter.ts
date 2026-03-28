import type { HarnessEvent, Session } from "../types";

export abstract class HttpAdapter implements Session {
    protected emitFn: (e: HarnessEvent) => void = () => {};
    private aborted = false;

    setEmit(fn: (e: HarnessEvent) => void) {
        this.emitFn = fn;
    }

    emitToolEvent(e: HarnessEvent) {
        this.emitFn(e);
    }

    stop() {
        this.aborted = true;
        this.onStop();
    }

    protected isAborted() { return this.aborted; }
    protected abstract onStop(): void;
    abstract send(text: string): void;
}
