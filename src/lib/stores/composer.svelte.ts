// src/lib/stores/composer.svelte.ts

export interface TaggedResult {
    toolId: string;
    toolName: string;
    output: string;       // already truncated to ≤50 rows
    label: string;        // e.g. "run_query result"
    truncated: boolean;
    totalRows?: number;
    timestamp: number;
}

class ComposerStore {
    taggedResults = $state<Map<string, TaggedResult>>(new Map());
    pendingChip = $state<{ path: string; lineStart: number; lineEnd: number } | null>(null);

    tagResult(toolId: string, toolName: string, rawOutput: string): void {
        let output = rawOutput;
        let truncated = false;
        let totalRows: number | undefined;
        try {
            const parsed = JSON.parse(rawOutput);
            if (Array.isArray(parsed) && parsed.length > 50) {
                totalRows = parsed.length;
                output = JSON.stringify(parsed.slice(0, 50));
                truncated = true;
            }
        } catch {
            // non-JSON output — store as-is
        }
        const newMap = new Map(this.taggedResults);
        newMap.set(toolId, {
            toolId, toolName, output,
            label: `${toolName} result`,
            truncated, totalRows,
            timestamp: Date.now(),
        });
        // keep at most 10
        if (newMap.size > 10) {
            const oldest = newMap.keys().next().value!;
            newMap.delete(oldest);
        }
        this.taggedResults = newMap;
    }

    untagResult(toolId: string): void {
        const newMap = new Map(this.taggedResults);
        newMap.delete(toolId);
        this.taggedResults = newMap;
    }

    isTagged(toolId: string): boolean {
        return this.taggedResults.has(toolId);
    }

    recentResults(): TaggedResult[] {
        return [...this.taggedResults.values()]
            .sort((a, b) => b.timestamp - a.timestamp)
            .slice(0, 10);
    }
}

export const composerStore = new ComposerStore();
