<script lang="ts">
    import { cn } from "$lib/utils";

    let {
        class: className,
        ...restProps
    }: import("svelte/elements").HTMLAttributes<HTMLSpanElement> = $props();
</script>

<!-- svelte-ignore a11y_missing_content -->
<span
    class={cn("ml-auto text-xs tracking-widest opacity-60", className)}
    {...restProps}
/>
