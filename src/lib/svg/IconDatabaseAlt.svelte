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
    <path d="M4 6c0 1.66 4 3 8 3s8 -1.34 8 -3s-4 -3 -8 -3s-8 1.34 -8 3"></path>
    <path d="M4 6v6c0 1.66 4 3 8 3c.53 0 1.05 -.02 1.56 -.07"></path>
    <path d="M20 12v-6"></path>
    <path d="M4 12v6c0 1.66 4 3 8 3c.27 0 .53 0 .79 -.01"></path>
    <path
        d="M18 22l3.35 -3.284a2.143 2.143 0 0 0 .005 -3.071a2.242 2.242 0 0 0 -3.129 -.006l-.224 .22l-3.35 3.284a2.143 2.143 0 0 0 -.005 3.071a2.242 2.242 0 0 0 3.129 .006l.224 -.22z"
    ></path>
</svg>
