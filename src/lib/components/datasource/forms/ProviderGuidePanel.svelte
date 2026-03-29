<!-- src/lib/components/datasource/forms/ProviderGuidePanel.svelte -->
<script lang="ts">
	import type { ProviderConfig } from "$lib/providers/registry";

	interface Props {
		provider: ProviderConfig;
	}

	let { provider }: Props = $props();
</script>

<div class="flex flex-col h-full bg-muted/30 border-l border-border p-5 min-w-[220px] max-w-[260px]">
	<!-- Header -->
	<div class="text-xs font-semibold mb-4" style="color: {provider.color}">
		Connect to {provider.label}
	</div>

	<!-- Steps -->
	<div class="flex flex-col gap-3 mb-5">
		{#each provider.guide as step, i}
			<div class="flex gap-3 items-start">
				<div
					class="flex-shrink-0 w-5 h-5 rounded-full flex items-center justify-center text-[10px] font-bold border"
					style="color: {provider.color}; border-color: {provider.color}33; background-color: {provider.color}11;"
				>
					{i + 1}
				</div>
				<div class="text-xs text-muted-foreground leading-relaxed">{step}</div>
			</div>
		{/each}
	</div>

	<!-- Notes (warnings) -->
	{#if provider.notes.length > 0}
		<div class="flex flex-col gap-2 mb-4">
			{#each provider.notes as note}
				<div class="bg-orange-500/8 border border-orange-500/20 rounded px-3 py-2">
					<div class="text-[10px] text-orange-400 font-semibold mb-0.5">⚠ Note</div>
					<div class="text-[10px] text-orange-400/70 leading-relaxed">{note}</div>
				</div>
			{/each}
		</div>
	{/if}

	<!-- TLS badge -->
	{#if provider.defaults.sslRequired}
		<div class="flex items-center gap-1.5 mt-auto">
			<div class="w-1.5 h-1.5 rounded-full bg-green-500"></div>
			<div class="text-[10px] text-muted-foreground">TLS enforced by default</div>
		</div>
	{/if}
</div>
