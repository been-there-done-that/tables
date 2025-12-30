<script lang="ts">
	import { cn } from "$lib/utils";
	import { getContext } from "svelte";

	interface Props {
		onclick?: (e: MouseEvent) => void;
		class?: string;
		children?: import("svelte").Snippet;
		variant?: "default" | "danger";
	}

	let {
		onclick,
		class: className,
		children,
		variant = "default",
	}: Props = $props();

	const ctx = getContext<{ setOpen: (val: boolean) => void }>("context-menu");

	function handleClick(e: MouseEvent) {
		onclick?.(e);
		ctx?.setOpen(false);
	}
</script>

<button
	type="button"
	class={cn(
		"flex w-full items-center gap-2 rounded-sm px-2 py-1.5 text-sm outline-none transition-colors",
		"hover:bg-(--theme-bg-hover) active:bg-(--theme-bg-active)",
		variant === "danger"
			? "text-red-500 hover:text-red-600"
			: "text-(--theme-fg-default)",
		className,
	)}
	onclick={handleClick}
>
	{@render children?.()}
</button>
