<script lang="ts">
    import { flip } from "svelte/animate";
    import { notifications } from "$lib/utils/notification.svelte";
    import NotificationItem from "./NotificationItem.svelte";

    // Position can be customized if needed, but defaulting to bottom-right
    const position = "bottom-right";
</script>

<div
    class="fixed inset-x-0 top-0 pointer-events-none flex flex-col p-6 items-center justify-start overflow-hidden"
    style="z-index: 999999;"
>
    <div class="flex flex-col space-y-3 w-full max-w-sm pointer-events-none">
        {#each notifications.notifications as item (item.id)}
            <div animate:flip={{ duration: 300 }}>
                <NotificationItem {item} />
            </div>
        {/each}
    </div>
</div>
