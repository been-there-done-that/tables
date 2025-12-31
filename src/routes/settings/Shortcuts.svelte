<script lang="ts">
    import { windowState } from "$lib/stores/window.svelte";

    let commands = $derived(Array.from(windowState.commands.values()));

    function getKeybinding(actionId: string): string {
        return windowState.formatKeybinding(actionId);
    }
</script>

<div class="p-6 w-full max-w-2xl mx-auto">
    <div class="mb-6">
        <h2 class="text-lg font-medium text-foreground">
            Keyboard Shortcuts
        </h2>
        <p class="text-sm text-foreground-muted mt-1">
            View and manage keyboard shortcuts for this window.
        </p>
    </div>

    <div class="rounded-md border border-border overflow-hidden">
        <table class="w-full text-sm text-left">
            <thead
                class="bg-surface text-foreground-muted"
            >
                <tr>
                    <th class="px-4 py-3 font-medium">Command</th>
                    <th class="px-4 py-3 font-medium text-right">Keybinding</th>
                </tr>
            </thead>
            <tbody class="divide-y divide-border">
                {#each commands as command}
                    <tr class="group hover:bg-muted">
                        <td
                            class="px-4 py-3 font-medium text-foreground"
                        >
                            {command.label}
                        </td>
                        <td class="px-4 py-3 text-right">
                            <span
                                class="inline-flex items-center px-2 py-1 rounded border border-border bg-muted font-mono text-xs text-foreground-muted"
                            >
                                {getKeybinding(command.id)}
                            </span>
                        </td>
                    </tr>
                {/each}
            </tbody>
        </table>
    </div>
</div>
