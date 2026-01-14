<script lang="ts">
    import { onMount } from "svelte";
    import { getCurrentWindow } from "@tauri-apps/api/window";
    import ConfirmationModal from "./ConfirmationModal.svelte";

    let showDialog = $state(false);

    const appWindow = getCurrentWindow();
    const isExcluded = ["datasource-window", "appearance-window"].includes(
        appWindow.label,
    );

    onMount(() => {
        if (isExcluded) return;

        const unlisten = appWindow.onCloseRequested((event) => {
            event.preventDefault();
            showDialog = true;
        });

        return () => {
            unlisten.then((u) => u());
        };
    });

    const handleConfirm = async () => {
        showDialog = false;
        try {
            await appWindow.destroy();
        } catch (e) {
            console.error("Failed to destroy window:", e);
            await appWindow.close();
        }
    };
</script>

<ConfirmationModal
    bind:open={showDialog}
    title="Close Window"
    message="Are you sure you want to close this window?"
    confirmText="Close"
    variant="danger"
    onConfirm={handleConfirm}
/>
