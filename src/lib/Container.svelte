<script lang="ts">
  import {cn} from "$lib/utils";
  let { class: className = "", children } = $props();
</script>

<section
  class={cn("w-full mx-auto p-8 rounded-md border shadow-md shadow-[color-mix(in_srgb,var(--theme-bg-active)_45%,transparent)]", className)}
  style={`background: var(--theme-bg-secondary); border-color: var(--theme-border-default);`}
>
  {@render children?.()}
</section>
