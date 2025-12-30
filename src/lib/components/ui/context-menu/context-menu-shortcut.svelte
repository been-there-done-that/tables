<script lang="ts">
	import { cn } from "$lib/utils";

	interface Props {
		class?: string;
		children?: import("svelte").Snippet;
	}

	let { class: className, children }: Props = $props();
</script>

<span
	class={cn(
		"ml-auto text-[10px] tracking-widest opacity-50 uppercase",
		className,
	)}
>
	{@render children?.()}
</span>
