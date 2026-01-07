<script lang="ts">
    import { windowState } from "$lib/stores/window.svelte";
    import { schemaStore } from "$lib/stores/schema.svelte";
    import IconPlus from "@tabler/icons-svelte/icons/plus";
    import IconDatabase from "@tabler/icons-svelte/icons/database";
    import IconKeyboard from "@tabler/icons-svelte/icons/keyboard";
    import IconLayout from "@tabler/icons-svelte/icons/layout";
    import { cn } from "$lib/utils";

    const activeSession = $derived(windowState.activeSession);

    const shortcuts = [
        { label: "Quick Select", keys: ["Meta", "P"], icon: IconDatabase },
        {
            label: "Search Commands",
            keys: ["Meta", "Shift", "P"],
            icon: IconKeyboard,
        },
        { label: "Toggle Sidebar", keys: ["Meta", "J"], icon: IconLayout },
        {
            label: "New Connection",
            keys: ["Meta", "Shift", "D"],
            icon: IconPlus,
        },
    ];

    function handleNewQuery() {
        if (activeSession) {
            activeSession.openView("editor", "New Query");
        } else {
            windowState.executeCommand("workbench.action.openDatasource");
        }
    }
</script>

<div
    class="flex flex-col items-center justify-center h-full w-full bg-background overflow-y-auto p-8 animate-fade-in"
>
    <div class="max-w-2xl w-full space-y-12">
        <!-- Hero Section -->
        <div class="text-center space-y-4">
            <div
                class="inline-flex p-3 rounded-2xl bg-primary/10 text-primary mb-2"
            >
                <IconDatabase class="size-8" />
            </div>
            <h1 class="text-3xl font-bold tracking-tight text-foreground">
                {activeSession
                    ? `Connected to ${activeSession.connection?.name}`
                    : "Welcome to Tables"}
            </h1>
            <p class="text-muted-foreground text-lg max-w-md mx-auto">
                {activeSession
                    ? "Start a new query or select a table from the explorer to begin analyzing your data."
                    : "Connect to a database to explore your schemas, run queries, and manage your data."}
            </p>
        </div>

        <!-- Quick Actions -->
        <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
            <button
                onclick={handleNewQuery}
                class={cn(
                    "flex flex-col items-start gap-3 p-6 rounded-xl border border-border bg-muted/30 transition-all duration-300",
                    "hover:border-primary/50 hover:bg-primary/5 hover:shadow-lg group text-left",
                )}
            >
                <div
                    class="p-2 rounded-lg bg-background border border-border group-hover:border-primary/30 transition-colors"
                >
                    <IconPlus class="size-5 text-primary" />
                </div>
                <div>
                    <h3 class="font-semibold text-foreground">New SQL Query</h3>
                    <p class="text-sm text-muted-foreground">
                        Open a fresh editor to write and execute SQL.
                    </p>
                </div>
            </button>

            <button
                onclick={() =>
                    windowState.executeCommand(
                        "workbench.action.openDatasource",
                    )}
                class={cn(
                    "flex flex-col items-start gap-3 p-6 rounded-xl border border-border bg-muted/30 transition-all duration-300",
                    "hover:border-primary/50 hover:bg-primary/5 hover:shadow-lg group text-left",
                )}
            >
                <div
                    class="p-2 rounded-lg bg-background border border-border group-hover:border-primary/30 transition-colors"
                >
                    <IconDatabase class="size-5 text-primary" />
                </div>
                <div>
                    <h3 class="font-semibold text-foreground">
                        Add Connection
                    </h3>
                    <p class="text-sm text-muted-foreground">
                        Connect to another database or cloud resource.
                    </p>
                </div>
            </button>
        </div>

        <!-- Shortcuts Guide -->
        <div class="space-y-4">
            <h2
                class="text-sm font-semibold text-muted-foreground uppercase tracking-wider px-1"
            >
                Quick Shortcuts
            </h2>
            <div class="grid grid-cols-1 sm:grid-cols-2 gap-3">
                {#each shortcuts as shortcut}
                    <div
                        class="flex items-center gap-3 p-3 rounded-lg border border-border/50 bg-muted/20"
                    >
                        <shortcut.icon
                            class="size-4 text-muted-foreground/60"
                        />
                        <span class="text-sm font-medium text-foreground/80"
                            >{shortcut.label}</span
                        >
                        <div class="ml-auto flex gap-1">
                            {#each shortcut.keys as key}
                                <kbd
                                    class="px-1.5 py-0.5 rounded border border-border bg-background text-[10px] font-sans font-medium text-muted-foreground"
                                >
                                    {key === "Meta" ? "⌘" : key}
                                </kbd>
                            {/each}
                        </div>
                    </div>
                {/each}
            </div>
        </div>
    </div>
</div>

<style>
    /* Subtle fade-in animation for the whole container */
    .animate-fade-in {
        animation: fadeIn 0.5s ease-out forwards;
    }

    @keyframes fadeIn {
        from {
            opacity: 0;
            transform: translateY(10px);
        }
        to {
            opacity: 1;
            transform: translateY(0);
        }
    }
</style>
