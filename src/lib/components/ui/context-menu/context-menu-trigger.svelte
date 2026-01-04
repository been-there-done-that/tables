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

	function handleContextMenu(e: MouseEvent) {
		if (!ctx) return;
		e.preventDefault();
		e.stopPropagation(); // Prevent parent context menus from interfering
		console.log(
			`[ContextMenu Debug] Clicked at (${e.clientX}, ${e.clientY})`,
		);
		ctx.setCoords({ x: e.clientX, y: e.clientY });
		ctx.setOpen(true);
	}
</script>

<div class="contents" oncontextmenu={handleContextMenu}>
	{@render children?.()}
</div>
