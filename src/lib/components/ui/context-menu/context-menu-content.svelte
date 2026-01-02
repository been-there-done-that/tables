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
						newY = window.innerHeight - rect.height - 2;
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
		// Only handle keys when THIS menu is open
		if (!ctx?.open) return;

		if (e.key === "Escape") {
			close();
			return;
		}

		if (!menuElement) return;

		const items = Array.from(
			menuElement.querySelectorAll("button:not([disabled])"),
		) as HTMLElement[];

		if (items.length === 0) return;

		const currentIndex = items.indexOf(
			document.activeElement as HTMLElement,
		);

		if (e.key === "ArrowDown") {
			e.preventDefault();
			const nextIndex = (currentIndex + 1) % items.length;
			items[nextIndex].focus();
		} else if (e.key === "ArrowUp") {
			e.preventDefault();
			const prevIndex = (currentIndex - 1 + items.length) % items.length;
			items[prevIndex].focus();
		} else if (e.key === "Home") {
			e.preventDefault();
			items[0].focus();
		} else if (e.key === "End") {
			e.preventDefault();
			items[items.length - 1].focus();
		} else if (e.key === "Enter") {
			// Button click will handle close via ContextMenuItem
		}
	}

	function handleClickOutside(e: MouseEvent) {
		if (menuElement && !menuElement.contains(e.target as Node)) {
			close();
		}
	}

	$effect(() => {
		if (ctx?.open) {
			// Auto-focus the first item when opened
			setTimeout(() => {
				if (
					menuElement &&
					!menuElement.contains(document.activeElement)
				) {
					const firstItem = menuElement.querySelector(
						"button:not([disabled])",
					) as HTMLElement;
					firstItem?.focus();
				}
			}, 50);
		}
	});

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
			"fixed z-50 min-w-[160px] overflow-hidden rounded-md border border-(--theme-border-default) bg-(--theme-bg-secondary) p-1 shadow-lg",
			className,
		)}
		style="left: {adjustedCoords.x}px; top: {adjustedCoords.y}px;"
		oncontextmenu={(e) => e.preventDefault()}
		onmousedown={(e) => e.stopPropagation()}
	>
		{@render children?.()}
	</div>
{/if}
