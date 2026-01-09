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
    <path d="M12 3c-4.4 0 -8 3 -8 7"></path>
    <path d="M4 14c0 4 3.6 7 8 7"></path>
    <path d="M20 10l-8 4l-8 -4"></path>
    <path d="M4 14l8 4l8 -4"></path>
    <path d="M16 3c2.2 0 4 3 4 7v4"></path>
</svg>
