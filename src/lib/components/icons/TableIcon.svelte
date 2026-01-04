<script>
    import { cn } from "$lib/utils";
    let { class: className } = $props();
</script>

<svg
    xmlns="http://www.w3.org/2000/svg"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="2"
    stroke-linecap="round"
    stroke-linejoin="round"
    class={cn(className)}
>
    <rect x="3" y="3" width="18" height="18" rx="2" />
    <path d="M3 10h18" />
    <path d="M10 3v18" />
</svg>
