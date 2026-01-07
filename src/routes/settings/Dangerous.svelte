<script lang="ts">
    import { invoke } from "@tauri-apps/api/core";
    import IconAlertTriangle from "@tabler/icons-svelte/icons/alert-triangle";
    import IconDatabase from "@tabler/icons-svelte/icons/database";
    import IconTrash from "@tabler/icons-svelte/icons/trash";
    import { toast } from "svelte-sonner";

    let isConfirmingReset = $state(false);

    async function handleReset() {
        if (!isConfirmingReset) {
            isConfirmingReset = true;
            return;
        }

        try {
            await invoke("reset_app_state");
            toast.success(
                "Application data reset successfully. Please restart the app.",
            );
            isConfirmingReset = false;
        } catch (e) {
            toast.error(`Reset failed: ${e}`);
        }
    }

    async function handleOpenInternalDb() {
        try {
            const connectionConfig = await invoke<any>("open_internal_db");
            // Open a new window that will connect to this internal DB
            await invoke("create_new_window", {
                label: "internal-db",
                title: "Internal Database",
                url: "/",
                // The new window can use this connectionConfig to connect on load
            });
            toast.info("Opening internal database in new window...");
        } catch (e) {
            toast.error(`Failed to open internal database: ${e}`);
        }
    }
</script>

<div
    class="relative h-full w-full overflow-hidden"
    style="background-image: url('/danger_bg.png'); background-size: cover; background-position: center;"
>
    <!-- Overlay for readability -->
    <div class="absolute inset-0 bg-background/90"></div>

    <div
        class="relative z-10 flex flex-col items-center justify-center h-full p-8"
    >
        <div class="max-w-md w-full space-y-8 text-center">
            <!-- Warning Header -->
            <div class="flex flex-col items-center gap-4">
                <div
                    class="p-4 bg-destructive/20 rounded-full border-2 border-destructive/50"
                >
                    <IconAlertTriangle class="size-12 text-destructive" />
                </div>
                <h1 class="text-2xl font-bold text-destructive">Danger Zone</h1>
                <p class="text-sm text-muted-foreground">
                    Actions in this section are irreversible and can result in
                    permanent data loss.
                </p>
            </div>

            <!-- Actions -->
            <div class="space-y-4">
                <!-- Reset Database -->
                <div
                    class="p-4 rounded-lg border border-destructive/30 bg-destructive/5"
                >
                    <div class="flex items-start gap-3">
                        <IconTrash class="size-5 text-destructive mt-0.5" />
                        <div class="flex-1 text-left">
                            <h3 class="font-medium text-foreground">
                                Reset All Data
                            </h3>
                            <p class="text-xs text-muted-foreground mt-1">
                                Clears all connections, settings, history, and
                                cached data. This cannot be undone.
                            </p>
                        </div>
                    </div>
                    <button
                        class="mt-4 w-full px-4 py-2 rounded-md text-sm font-medium transition-colors
                   {isConfirmingReset
                            ? 'bg-destructive text-destructive-foreground hover:bg-destructive/90'
                            : 'bg-destructive/20 text-destructive hover:bg-destructive/30 border border-destructive/30'}"
                        onclick={handleReset}
                    >
                        {isConfirmingReset
                            ? "Click Again to Confirm Reset"
                            : "Reset All Data"}
                    </button>
                    {#if isConfirmingReset}
                        <button
                            class="mt-2 w-full px-4 py-1.5 rounded-md text-xs text-muted-foreground hover:text-foreground"
                            onclick={() => (isConfirmingReset = false)}
                        >
                            Cancel
                        </button>
                    {/if}
                </div>

                <!-- Open Internal DB -->
                <div
                    class="p-4 rounded-lg border border-amber-500/30 bg-amber-500/5"
                >
                    <div class="flex items-start gap-3">
                        <IconDatabase class="size-5 text-amber-500 mt-0.5" />
                        <div class="flex-1 text-left">
                            <h3 class="font-medium text-foreground">
                                Open Internal Database
                            </h3>
                            <p class="text-xs text-muted-foreground mt-1">
                                Opens the app's internal SQLite database in a
                                new window for inspection.
                            </p>
                        </div>
                    </div>
                    <button
                        class="mt-4 w-full px-4 py-2 rounded-md text-sm font-medium transition-colors
                   bg-amber-500/20 text-amber-500 hover:bg-amber-500/30 border border-amber-500/30"
                        onclick={handleOpenInternalDb}
                    >
                        Open Internal Database
                    </button>
                </div>
            </div>
        </div>
    </div>
</div>
