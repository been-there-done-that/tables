<script lang="ts">
    import { schemaStore } from "$lib/stores/schema.svelte";
    import PostgresExplorer from "./engines/PostgresExplorer.svelte";
    import SqliteExplorer from "./engines/SqliteExplorer.svelte";

    const engine = $derived(schemaStore.activeConnection?.engine);
    const isLoading = $derived(schemaStore.status === "connecting" || schemaStore.status === "refreshing");

    // Tree-skeleton rows: [indentLevel, widthPercent]
    const skeletonRows: [number, number][] = [
        [0, 55], [1, 70], [1, 60], [1, 75], [2, 50], [2, 65],
        [1, 68], [0, 52], [1, 72], [1, 58], [1, 80], [2, 45],
    ];
</script>

{#if isLoading}
    <div class="flex flex-col gap-1.5 p-2 pt-3">
        {#each skeletonRows as [indent, width]}
            <div
                class="shimmer h-5 rounded"
                style="width: {width}%; margin-left: {indent * 14}px;"
            ></div>
        {/each}
    </div>
{:else if engine === "postgres" || engine === "mysql" || engine === "mariadb"}
    <PostgresExplorer />
{:else if engine === "sqlite"}
    <SqliteExplorer />
{:else}
    <!-- Fallback to Postgres style for now as generic -->
    <PostgresExplorer />
{/if}
