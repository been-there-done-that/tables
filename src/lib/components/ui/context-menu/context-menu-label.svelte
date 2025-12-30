<script lang="ts">
	import { cn } from "$lib/utils";

	interface Props {
		class?: string;
		children?: import("svelte").Snippet;
	}

	let { class: className, children }: Props = $props();
</script>

<div
	class={cn(
		"px-2 py-1.5 text-xs font-semibold text-[var(--theme-fg-secondary)] opacity-70 uppercase tracking-wider",
		className,
	)}
>
	{@render children?.()}
</div>
