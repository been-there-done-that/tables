<script lang="ts">
  import { cn } from "$lib/utils";

  let {
    value = $bindable(""),
    placeholder = "",
    type = "text",
    id = "",
    class: className = "",
    ...rest
  } = $props();
</script>

<input
  {id}
  {type}
  bind:value
  {placeholder}
  class={cn(
    "w-full h-7 px-2 text-xs transition outline-none",
    "bg-(--theme-bg-secondary) text-(--theme-fg-primary)",
    "border border-(--theme-border-default) rounded-sm",
    "placeholder:text-(--theme-fg-tertiary) placeholder:opacity-50",
    "hover:border-(--theme-border-focus)",
    "focus:border-(--theme-accent-primary) focus:ring-1 focus:ring-(--theme-accent-primary)/30",
    className
  )}
  {...rest}
/>
