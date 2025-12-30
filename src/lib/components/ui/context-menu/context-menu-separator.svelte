<script lang="ts">
	import { cn } from "$lib/utils";

	interface Props {
		class?: string;
	}

	let { class: className }: Props = $props();
</script>

<div
	class={cn("my-1 h-px bg-(--theme-border-default) opacity-50", className)}
></div>
