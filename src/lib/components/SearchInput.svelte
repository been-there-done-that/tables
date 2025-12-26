<script lang="ts">
  let {
    value = $bindable(""),
    placeholder = "",
    class: className = "",
  } = $props<{
    value?: string;
    placeholder?: string;
    class?: string;
  }>();
</script>

<input
  type="search"
  class={`h-8 w-full rounded-md border border-(--theme-border-default) bg-(--theme-bg-primary) px-3 py-1.5 text-sm focus:outline-none focus-visible:ring-0 ${className}`}
  placeholder={placeholder}
  autocomplete="off"
  spellcheck="false"
  bind:value={value}
/>
