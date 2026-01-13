<script lang="ts">
    import { DropdownMenu as DropdownMenuPrimitive } from "bits-ui";
    import IconChevronRight from "@tabler/icons-svelte/icons/chevron-right";
    import { cn } from "$lib/utils";

    let {
        ref = $bindable(null),
        class: className,
        children,
        ...restProps
    }: DropdownMenuPrimitive.SubTriggerProps = $props();
</script>

<DropdownMenuPrimitive.SubTrigger
    bind:ref
    class={cn(
        "group flex cursor-default select-none items-center rounded-sm px-2 py-1.5 text-sm outline-none focus:bg-accent/10 data-state-open:bg-accent/10 data-highlighted:bg-accent/10 font-medium text-(--theme-fg-secondary)",
        className,
    )}
    {...restProps}
>
    {@render children?.()}
    <IconChevronRight
        class="ml-auto size-4 shrink-0 opacity-40 group-hover:opacity-100 transition-opacity"
    />
</DropdownMenuPrimitive.SubTrigger>
