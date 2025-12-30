<script lang="ts">
    import { setContext } from "svelte";

    interface Props {
        open?: boolean;
        onOpenChange?: (open: boolean) => void;
        children?: import("svelte").Snippet;
    }

    let { open = $bindable(false), onOpenChange, children }: Props = $props();

    let coords = $state({ x: 0, y: 0 });

    function setOpen(val: boolean) {
        open = val;
        onOpenChange?.(val);
    }

    function setCoords(newCoords: { x: number; y: number }) {
        coords = newCoords;
    }

    setContext("context-menu", {
        get open() {
            return open;
        },
        setOpen,
        get coords() {
            return coords;
        },
        setCoords,
    });
</script>

{@render children?.()}
