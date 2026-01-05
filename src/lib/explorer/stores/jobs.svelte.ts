// src/lib/explorer/stores/jobs.svelte.ts

export type JobScope =
    | { type: "global" }
    | { type: "database"; database: string }
    | { type: "schema"; database: string; schema: string }
    | { type: "table"; database: string; schema: string; table: string };

export type IntrospectionJob = {
    id: string; // unique job id
    provider: string; // e.g., 'postgres'
    connectionId: string;
    scope: JobScope;
    status: "pending" | "running" | "done" | "error";
    startTime: number;
    error?: string;
};

export class JobStore {
    private _jobs = $state(new Map<string, IntrospectionJob>());

    start(job: IntrospectionJob) {
        this._jobs.set(job.id, job);
    }

    complete(id: string) {
        const job = this._jobs.get(id);
        if (job) {
            job.status = "done";
            // Optional: remove after a delay to clean up memory
            // setTimeout(() => this._jobs.delete(id), 5000);
        }
    }

    fail(id: string, error: string) {
        const job = this._jobs.get(id);
        if (job) {
            job.status = "error";
            job.error = error;
        }
    }

    // Helper to find if a specific scope is loading
    isLoading(scopeMatcher: (scope: JobScope) => boolean): boolean {
        for (const job of this._jobs.values()) {
            if ((job.status === "running" || job.status === "pending") && scopeMatcher(job.scope)) {
                return true;
            }
        }
        return false;
    }
}

export const jobs = new JobStore();
