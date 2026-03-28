// packages/harness/src/adapters/async-queue.ts

export class AsyncQueue<T> {
    private buffer: T[] = [];
    private waiting: ((value: IteratorResult<T>) => void) | null = null;
    private done = false;

    push(item: T) {
        if (this.waiting) {
            const resolve = this.waiting;
            this.waiting = null;
            resolve({ value: item, done: false });
        } else {
            this.buffer.push(item);
        }
    }

    close() {
        this.done = true;
        if (this.waiting) {
            this.waiting({ value: undefined as any, done: true });
            this.waiting = null;
        }
    }

    [Symbol.asyncIterator](): AsyncIterator<T> {
        return {
            next: (): Promise<IteratorResult<T>> => {
                if (this.buffer.length > 0) return Promise.resolve({ value: this.buffer.shift()!, done: false });
                if (this.done) return Promise.resolve({ value: undefined as any, done: true });
                return new Promise((resolve) => { this.waiting = resolve; });
            },
        };
    }
}
