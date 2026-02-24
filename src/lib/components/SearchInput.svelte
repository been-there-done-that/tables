<script lang="ts">
  let {
    value = $bindable(""),
    placeholder = "",
    class: className = "",
    inputRef = $bindable<HTMLInputElement | null>(null),
  } = $props<{
    value?: string;
    placeholder?: string;
    class?: string;
    inputRef?: HTMLInputElement | null;
  }>();
</script>

<input
  type="search"
  name="search_{Math.random().toString(36).substring(7)}"
  class={`h-8 w-full rounded-md border border-(--theme-border-default) bg-(--theme-bg-primary) px-3 py-1.5 text-sm focus:outline-none focus-visible:ring-0 ${className}`}
  {placeholder}
  autocomplete="new-password"
  spellcheck="false"
  data-lpignore="true"
  data-form-type="other"
  data-1p-ignore
  bind:value
  bind:this={inputRef}
/>
