<script lang="ts">
    import { getContext } from "svelte";

    interface Props {
        children?: import("svelte").Snippet;
    }

    let { children }: Props = $props();

    const ctx = getContext<{
        setOpen: (val: boolean) => void;
        setCoords: (coords: { x: number; y: number }) => void;
    }>("context-menu");

    function handleClick(e: MouseEvent) {
        if (!ctx) return;
        e.preventDefault();
        const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
        ctx.setCoords({ x: rect.left + rect.width / 2, y: rect.bottom + 4 });
        ctx.setOpen(true);
    }
</script>

<div
    class="w-fit h-fit"
    onclick={handleClick}
    onkeydown={(e) => e.key === "Enter" && handleClick(e as any)}
>
    {@render children?.()}
</div>
