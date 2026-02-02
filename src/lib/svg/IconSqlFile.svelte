<script lang="ts">
    let { class: className = undefined, ...rest } = $props();
</script>

<svg
    xmlns="http://www.w3.org/2000/svg"
    width="16"
    height="16"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="2"
    stroke-linecap="round"
    stroke-linejoin="round"
    class={className}
    {...rest}
>
    <!-- File shape -->
    <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"></path>
    <polyline points="14 2 14 8 20 8"></polyline>

    <!-- SQL-like text or symbol inside -->
    <text
        x="6"
        y="18"
        font-size="6"
        font-weight="bold"
        fill="currentColor"
        stroke="none">SQL</text
    >
</svg>
