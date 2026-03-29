<!-- src/lib/components/datasource/forms/ConnectionStringInput.svelte -->
<script lang="ts">
	import { parseConnectionString, type PostgresFormData } from "$lib/providers/parseConnectionString";

	interface Props {
		onParse: (result: Partial<PostgresFormData>) => void;
	}

	let { onParse }: Props = $props();

	let raw = $state("");
	let status: "idle" | "ok" | "error" = $state("idle");

	function handleInput(e: Event) {
		const value = (e.currentTarget as HTMLInputElement).value;
		raw = value;

		if (!value.trim()) {
			status = "idle";
			return;
		}

		const result = parseConnectionString(value);
		if (result) {
			status = "ok";
			onParse(result);
		} else {
			status = "error";
		}
	}
</script>

<div class="mb-4">
	<div class="text-xs text-(--theme-fg-secondary) uppercase tracking-wide mb-1.5">
		Connection string
	</div>
	<input
		type="text"
		placeholder="postgresql://user:password@host:5432/database"
		value={raw}
		oninput={handleInput}
		class="h-8 w-full rounded-md border border-dashed bg-(--theme-bg-primary) px-3 py-1.5 text-sm font-mono focus:outline-none focus-visible:ring-0
			{status === 'ok' ? 'border-green-500' :
			 status === 'error' ? 'border-red-500' :
			 'border-(--theme-border-default)'}"
	/>
	{#if status === "ok"}
		<div class="text-xs text-green-500 mt-1">↳ Fields filled from connection string</div>
	{:else if status === "error"}
		<div class="text-xs text-red-500 mt-1">Invalid connection string — use postgresql://user:pass@host:port/db format</div>
	{/if}
</div>
