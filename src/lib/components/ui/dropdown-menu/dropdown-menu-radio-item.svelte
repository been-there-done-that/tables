<script lang="ts">
    import { DropdownMenu as DropdownMenuPrimitive } from "bits-ui";
    import IconCircleFilled from "@tabler/icons-svelte/icons/circle-filled";
    import { cn } from "$lib/utils";
    import type { Snippet } from "svelte";

    let {
        ref = $bindable(null),
        class: className,
        value,
        children: childrenProp,
        ...restProps
    }: DropdownMenuPrimitive.RadioItemProps & {
        children?: Snippet;
    } = $props();
</script>

<DropdownMenuPrimitive.RadioItem
    bind:ref
    {value}
    class={cn(
        "relative flex cursor-default select-none items-center rounded-sm py-1.5 pl-8 pr-2 text-sm outline-none transition-colors data-disabled:pointer-events-none data-disabled:opacity-50 data-highlighted:bg-accent/10",
        className,
    )}
    {...restProps}
>
    {#snippet children({ checked })}
        <span
            class="absolute left-2 flex h-3.5 w-3.5 items-center justify-center"
        >
            {#if checked}
                <IconCircleFilled class="h-2 w-2" />
            {/if}
        </span>
        {@render childrenProp?.()}
    {/snippet}
</DropdownMenuPrimitive.RadioItem>
