<script lang="ts">
	import { getContext, onMount } from "svelte";
	import { scale } from "svelte/transition";
	import { cn } from "$lib/utils";

	interface Props {
		class?: string;
		children?: import("svelte").Snippet;
	}

	let { class: className, children }: Props = $props();

	const ctx = getContext<{
		open: boolean;
		setOpen: (val: boolean) => void;
		coords: { x: number; y: number };
	}>("context-menu");

	let menuElement = $state<HTMLElement | null>(null);
	let adjustedCoords = $state({ x: 0, y: 0 });

	$effect(() => {
		if (ctx?.open && ctx?.coords) {
			adjustedCoords = { ...ctx.coords };
			// Small delay to ensure menuElement is rendered and we can measure it
			setTimeout(() => {
				if (menuElement) {
					const rect = menuElement.getBoundingClientRect();
					let newX = ctx.coords.x;
					let newY = ctx.coords.y;

					if (newX + rect.width > window.innerWidth) {
						newX = window.innerWidth - rect.width - 10;
					}
					if (newY + rect.height > window.innerHeight) {
						newY = window.innerHeight - rect.height - 10;
					}
					adjustedCoords = { x: newX, y: newY };
				}
			}, 0);
		}
	});

	function close() {
		ctx?.setOpen(false);
	}

	function handleKeyDown(e: KeyboardEvent) {
		if (e.key === "Escape") close();
	}

	function handleClickOutside(e: MouseEvent) {
		if (menuElement && !menuElement.contains(e.target as Node)) {
			close();
		}
	}

	onMount(() => {
		window.addEventListener("keydown", handleKeyDown);
		window.addEventListener("mousedown", handleClickOutside);
		return () => {
			window.removeEventListener("keydown", handleKeyDown);
			window.removeEventListener("mousedown", handleClickOutside);
		};
	});
</script>

{#if ctx?.open}
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div
		bind:this={menuElement}
		transition:scale={{ duration: 100, start: 0.95 }}
		class={cn(
			"fixed z-9999 min-w-[160px] overflow-hidden rounded-md border border-(--theme-border-default) bg-(--theme-bg-secondary) p-1 shadow-xl",
			"backdrop-blur-md bg-opacity-95",
			className,
		)}
		style="left: {adjustedCoords.x}px; top: {adjustedCoords.y}px;"
		oncontextmenu={(e) => e.preventDefault()}
		onmousedown={(e) => e.stopPropagation()}
	>
		{@render children?.()}
	</div>
{/if}
