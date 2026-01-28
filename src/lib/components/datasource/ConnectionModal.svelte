<script lang="ts">
    import DraggableWindow from "$lib/components/DraggableWindow.svelte";
    import DataSource from "./DataSource.svelte";

    let { open = $bindable(false) } = $props();
</script>

{#if open}
    <DraggableWindow
        bind:open
        title="New Connection"
        modal={true}
        class="w-[900px] h-[650px] flex flex-col"
        contentClass="flex-1 overflow-hidden"
        initialPosition="center"
        openShortcut={null}
        closeShortcut={null}
        headerActions={null}
        onClose={() => (open = false)}
    >
        <div class="h-full w-full">
            <DataSource onConnectSuccess={() => (open = false)} />
        </div>
    </DraggableWindow>
{/if}
