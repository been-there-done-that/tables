<script>
    let { class: className, ...rest } = $props();
</script>

<svg
    xmlns="http://www.w3.org/2000/svg"
    width="24"
    height="24"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="1.5"
    stroke-linecap="round"
    stroke-linejoin="round"
    class={className}
    {...rest}
>
    <path d="M12 3v18"></path>
    <path d="M6 8l6 -5l6 5"></path>
    <path d="M6 16l6 5l6 -5"></path>
    <path d="M6 12h12"></path>
    <rect x="5" y="8" width="14" height="8" rx="1"></rect>
</svg>
