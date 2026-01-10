<script lang="ts">
    import type { Column } from "./types";

    interface Props {
        columns?: Column[];
        rowCount?: number;
    }

    let { columns = [], rowCount = 8 }: Props = $props();

    // Use reasonable defaults if no columns provided
    const skeletonColumns = $derived(
        columns.length > 0
            ? columns.slice(0, 6) // Limit to 6 columns for cleaner skeleton
            : [
                  { id: "1", width: 100 },
                  { id: "2", width: 180 },
                  { id: "3", width: 150 },
                  { id: "4", width: 120 },
                  { id: "5", width: 200 },
              ],
    );

    const totalWidth = $derived(
        skeletonColumns.reduce(
            (acc, col) => acc + ((col as any).width || 150),
            60,
        ),
    );
</script>

<div class="h-full w-full overflow-hidden">
    <div style="width: {totalWidth}px; min-width: 100%;">
        <!-- Skeleton Rows -->
        {#each Array(rowCount) as _, rowIndex}
            <div
                class="flex items-center border-b border-border/30 h-9 animate-pulse"
                style="animation-delay: {rowIndex * 50}ms"
            >
                <!-- Row number cell -->
                <div
                    class="shrink-0 w-[60px] h-full flex items-center justify-center"
                >
                    <div class="h-3 w-6 rounded bg-muted-foreground/10"></div>
                </div>

                <!-- Data cells -->
                {#each skeletonColumns as col, colIndex}
                    <div
                        class="shrink-0 h-full flex items-center px-3"
                        style="width: {(col as any).width || 150}px"
                    >
                        <div
                            class="h-3 rounded bg-muted-foreground/10 skeleton-shimmer"
                            style="width: {40 +
                                Math.random() *
                                    40}%; animation-delay: {(rowIndex *
                                skeletonColumns.length +
                                colIndex) *
                                30}ms"
                        ></div>
                    </div>
                {/each}
            </div>
        {/each}
    </div>
</div>

<style>
    .skeleton-shimmer {
        background: linear-gradient(
            90deg,
            var(--muted-foreground) 0%,
            var(--muted-foreground) 40%,
            color-mix(in srgb, var(--muted-foreground), transparent 60%) 50%,
            var(--muted-foreground) 60%,
            var(--muted-foreground) 100%
        );
        background-size: 200% 100%;
        animation: shimmer 1.5s ease-in-out infinite;
        opacity: 0.1;
    }

    @keyframes shimmer {
        0% {
            background-position: 200% 0;
        }
        100% {
            background-position: -200% 0;
        }
    }

    .animate-pulse {
        animation: pulse 2s cubic-bezier(0.4, 0, 0.6, 1) infinite;
    }

    @keyframes pulse {
        0%,
        100% {
            opacity: 1;
        }
        50% {
            opacity: 0.6;
        }
    }
</style>
