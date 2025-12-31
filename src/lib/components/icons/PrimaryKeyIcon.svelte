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
    <!-- Base Column Icon (faded slightly to let key pop) -->
    <rect x="3" y="3" width="18" height="18" rx="2" class="stroke-foreground/40"
    ></rect>
    <path d="M9 3v18" class="stroke-blue-500"></path>
    <path d="M3 3h6v18H3z" class="fill-blue-500/10 stroke-none"></path>

    <!-- Key Overlay (Bottom Right, Amber/Orange) -->
    <!-- Creating a small key icon positioned at bottom right -->
    <g transform="translate(10, 10) scale(0.6)">
        <circle
            cx="16.5"
            cy="7.5"
            r="4.5"
            class="stroke-orange-500 fill-background"
            stroke-width="3"
        />
        <path d="M12 11L7 16" class="stroke-orange-500" stroke-width="3" />
        <path d="M7 16L9 18" class="stroke-orange-500" stroke-width="3" />
        <path d="M6 17L5 18" class="stroke-orange-500" stroke-width="3" />
    </g>
</svg>
