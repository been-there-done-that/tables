<script lang="ts">
    import { DropdownMenu as DropdownMenuPrimitive } from "bits-ui";

    let {
        value = $bindable(),
        children,
        ...restProps
    }: DropdownMenuPrimitive.RadioGroupProps = $props();
</script>

<DropdownMenuPrimitive.RadioGroup bind:value {...restProps}>
    {@render children?.()}
</DropdownMenuPrimitive.RadioGroup>
