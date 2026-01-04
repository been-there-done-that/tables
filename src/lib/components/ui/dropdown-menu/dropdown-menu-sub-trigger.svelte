<script lang="ts">
    import { DropdownMenu as DropdownMenuPrimitive } from "bits-ui";
    import ChevronRight from "lucide-svelte/icons/chevron-right";
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
        "flex cursor-default select-none items-center rounded-sm px-2 py-1.5 text-sm outline-none focus:bg-accent data-state-open:bg-accent data-highlighted:bg-accent",
        className,
    )}
    {...restProps}
>
    {@render children?.()}
    <ChevronRight class="ml-auto h-4 w-4" />
</DropdownMenuPrimitive.SubTrigger>
