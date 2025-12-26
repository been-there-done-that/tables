<script lang="ts">
  import IconChevronRight from "@tabler/icons-svelte/icons/chevron-right";
  import { cn } from "$lib/utils";
  import type { Snippet } from "svelte";

  let {
    label = "Advanced Options",
    open = $bindable(false),
    children,
  }: {
    label?: string;
    open?: boolean;
    children: Snippet;
  } = $props();

  function toggle() {
    open = !open;
  }
</script>

<div class="mt-4">
  <button
    type="button"
    onclick={toggle}
    class="flex items-center gap-1.5 text-[11px] font-semibold uppercase tracking-wider text-(--theme-fg-tertiary) hover:text-(--theme-fg-secondary) transition-colors"
  >
    <IconChevronRight
      class={cn("size-3 transition-transform duration-200", open && "rotate-90")}
    />
    {label}
  </button>

  {#if open}
    <div class="mt-2 pl-4 border-l border-(--theme-border-default) space-y-1">
      {@render children()}
    </div>
  {/if}
</div>
