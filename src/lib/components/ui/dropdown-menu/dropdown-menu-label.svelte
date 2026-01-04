<script lang="ts">
    import { DropdownMenu as DropdownMenuPrimitive } from "bits-ui";
    import { cn } from "$lib/utils";

    let {
        ref = $bindable(null),
        class: className,
        children,
        ...restProps
    }: any = $props();
</script>

<DropdownMenuPrimitive.Label
    bind:ref
    class={cn("px-2 py-1.5 text-sm font-semibold", className)}
    {...restProps}
>
    {@render children?.()}
</DropdownMenuPrimitive.Label>
