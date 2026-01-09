<script lang="ts">
    import { DropdownMenu as DropdownMenuPrimitive } from "bits-ui";
    import { cn } from "$lib/utils";

    let {
        ref = $bindable(null),
        class: className,
        children,
        ...restProps
    }: DropdownMenuPrimitive.ItemProps = $props();
</script>

<DropdownMenuPrimitive.Item
    bind:ref
    class={cn(
        "relative flex cursor-default select-none items-center rounded-sm px-2 py-1.5 text-sm outline-none transition-colors data-disabled:pointer-events-none data-disabled:opacity-50 data-highlighted:bg-accent/10",
        className,
    )}
    {...restProps}
>
    {@render children?.()}
</DropdownMenuPrimitive.Item>
