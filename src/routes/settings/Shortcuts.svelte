<script lang="ts">
    import { windowState } from "$lib/stores/window.svelte";

    let commands = $derived(Array.from(windowState.commands.values()));

    function getKeybinding(actionId: string): string {
        for (const [key, id] of windowState.keybindings.entries()) {
            if (id === actionId) {
                return key
                    .split("+")
                    .map((k) => k.charAt(0).toUpperCase() + k.slice(1))
                    .join(" + ");
            }
        }
        return "None";
    }
</script>

<div class="p-6 w-full max-w-2xl mx-auto">
    <div class="mb-6">
        <h2
            class="text-lg font-medium text-[--theme-fg-primary]"
            style="color: var(--theme-fg-primary);"
        >
            Keyboard Shortcuts
        </h2>
        <p class="text-sm text-[--theme-fg-secondary] mt-1">
            View and manage keyboard shortcuts for this window.
        </p>
    </div>

    <div
        class="rounded-md border border-[--theme-border-subtle] overflow-hidden"
        style="border-color: var(--theme-border-subtle);"
    >
        <table class="w-full text-sm text-left">
            <thead
                class="bg-[--theme-bg-secondary] text-[--theme-fg-secondary]"
                style="background: var(--theme-bg-secondary); color: var(--theme-fg-secondary);"
            >
                <tr>
                    <th class="px-4 py-3 font-medium">Command</th>
                    <th class="px-4 py-3 font-medium text-right">Keybinding</th>
                </tr>
            </thead>
            <tbody class="divide-y divide-[--theme-border-subtle]">
                {#each commands as command}
                    <tr class="group hover:bg-[--theme-bg-hover]">
                        <td
                            class="px-4 py-3 font-medium text-[--theme-fg-primary]"
                            style="color: var(--theme-fg-primary);"
                        >
                            {command.label}
                        </td>
                        <td class="px-4 py-3 text-right">
                            <span
                                class="inline-flex items-center px-2 py-1 rounded border border-[--theme-border-default] bg-[--theme-bg-tertiary] font-mono text-xs text-[--theme-fg-secondary]"
                                style="background: var(--theme-bg-tertiary); border-color: var(--theme-border-default); color: var(--theme-fg-secondary);"
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
