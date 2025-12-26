<script lang="ts">
  import { cn } from "$lib/utils";
  import type { Snippet } from "svelte";

  let {
    label = "",
    id = "",
    class: className = "",
    labelWidth = "w-28", // Fixed label width for alignment
    children,
  }: {
    label: string;
    id?: string;
    className?: string;
    labelWidth?: string;
    children: Snippet;
  } = $props();
</script>

<div class={cn("flex items-center gap-3 py-1", className)}>
  <label
    for={id}
    class={cn(
      "text-xs font-medium text-(--theme-fg-secondary) shrink-0 text-right",
      labelWidth
    )}
  >
    {label}
  </label>
  <div class="flex-1">
    {@render children()}
  </div>
</div>
