<script lang="ts">
    import IconFolder from "@tabler/icons-svelte/icons/folder";
    import ConfigInput from "./ConfigInput.svelte";
    import { cn } from "$lib/utils";

    let {
        value = $bindable(""),
        placeholder = "Select database file...",
        label = "File",
        id = "",
        onPick = () => {},
    } = $props();
</script>

<div class="relative flex items-center group">
    <ConfigInput bind:value {placeholder} class="pr-8" />
    <button
        type="button"
        onclick={onPick}
        class={cn(
            "absolute right-0.5 h-6 w-7 flex items-center justify-center",
            "text-(--theme-fg-tertiary) hover:text-(--theme-fg-primary)",
            "hover:bg-(--theme-bg-tertiary) rounded-sm transition-colors",
        )}
        title="Choose file"
    >
        <IconFolder class="size-3.5" />
    </button>
</div>
