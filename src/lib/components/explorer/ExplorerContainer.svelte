<script lang="ts">
    import { schemaStore } from "$lib/stores/schema.svelte";
    import PostgresExplorer from "./engines/PostgresExplorer.svelte";
    import SqliteExplorer from "./engines/SqliteExplorer.svelte";

    const engine = $derived(schemaStore.activeConnection?.engine);
</script>

{#if engine === "postgres" || engine === "mysql" || engine === "mariadb"}
    <PostgresExplorer />
{:else if engine === "sqlite"}
    <SqliteExplorer />
{:else}
    <!-- Fallback to Postgres style for now as generic -->
    <PostgresExplorer />
{/if}
