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
	<div class="text-xs text-[#909090] uppercase tracking-wide mb-1.5">
		Connection string
	</div>
	<input
		type="text"
		placeholder="postgresql://user:password@host:5432/database"
		value={raw}
		oninput={handleInput}
		class="w-full bg-[#2b2d30] border border-dashed rounded px-3 py-2 text-sm text-[#a9b7c6] placeholder-[#555] outline-none font-mono
			focus:ring-1 focus:outline-none
			{status === 'ok' ? 'border-[#22c55e] focus:border-[#22c55e] focus:ring-[#22c55e]' :
			 status === 'error' ? 'border-[#ef4444] focus:border-[#ef4444] focus:ring-[#ef4444]' :
			 'border-[#5e6060] focus:border-[#3574f0] focus:ring-[#3574f0]'}"
	/>
	{#if status === "ok"}
		<div class="text-xs text-[#22c55e] mt-1">↳ Fields filled from connection string</div>
	{:else if status === "error"}
		<div class="text-xs text-[#ef4444] mt-1">Invalid connection string — use postgresql://user:pass@host:port/db format</div>
	{/if}
</div>
