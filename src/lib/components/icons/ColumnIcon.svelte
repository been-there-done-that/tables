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
    class={cn("text-blue-500", className)}
>
    <rect x="3" y="3" width="18" height="18" rx="2" class="stroke-foreground/40"
    ></rect>
    <path d="M9 3v18" class="stroke-blue-500"></path>
    <path d="M3 3h6v18H3z" class="fill-blue-500/10 stroke-none"></path>
</svg>
