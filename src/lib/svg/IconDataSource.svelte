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
    <rect x="2" y="3" width="20" height="8" rx="2" ry="2"></rect>
    <rect x="2" y="13" width="20" height="8" rx="2" ry="2"></rect>
    <line x1="6" y1="7" x2="6.01" y2="7"></line>
    <line x1="10" y1="7" x2="10.01" y2="7"></line>
    <line x1="6" y1="17" x2="6.01" y2="17"></line>
    <line x1="10" y1="17" x2="10.01" y2="17"></line>
</svg>
