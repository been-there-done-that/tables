<script lang="ts">
  let {
    value = $bindable(""),
    placeholder = "",
    type = "text",
    label = "",
    inputId = "",
    class: className = "",
    disabled = false,
    required = false,
    oninput = undefined,
  } = $props<{
    value?: string;
    placeholder?: string;
    type?: "text" | "password" | "number" | "email" | "url";
    label?: string;
    inputId?: string;
    class?: string;
    disabled?: boolean;
    required?: boolean;
    oninput?: ((e: Event) => void) | undefined;
  }>();
</script>

<div class="space-y-1">
  {#if label}
    <label class="text-xs font-medium text-(--theme-fg-secondary)" for={inputId}>{label}</label>
  {/if}
  <input
    id={inputId}
    type={type}
    class={`h-8 w-full rounded-md border border-(--theme-border-default) bg-(--theme-bg-primary) px-3 py-1.5 text-sm focus:outline-none focus-visible:ring-0 ${className}`}
    placeholder={placeholder}
    autocomplete="off"
    spellcheck="false"
    bind:value={value}
    disabled={disabled}
    required={required}
    oninput={oninput}
  />
</div>
