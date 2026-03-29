<!-- src/lib/components/datasource/forms/ProviderForm.svelte -->
<script lang="ts">
	import { PROVIDERS } from "$lib/providers/registry";
	import type { PostgresFormData } from "$lib/providers/parseConnectionString";
	import { untrack } from "svelte";
	import ConnectionStringInput from "./ConnectionStringInput.svelte";
	import ProviderGuidePanel from "./ProviderGuidePanel.svelte";
	import PostgresForm from "./PostgresForm.svelte";

	interface Props {
		providerId: string;
		data: any;
		onChange: (field: string, value: any) => void;
		hideFooter?: boolean;
	}

	let { providerId, data, onChange, hideFooter = false }: Props = $props();

	const provider = $derived(PROVIDERS[providerId]);

	// Apply provider defaults when provider changes (e.g. user picks different cloud provider).
	// untrack data reads so user edits don't re-trigger this effect.
	$effect(() => {
		const p = provider; // tracked: re-runs when provider changes
		if (!p) return;
		untrack(() => {
			if (!data.db?.host) onChange("db.host", "");
			if (!data.db?.port) onChange("db.port", p.defaults.port);
			if (!data.db?.database) onChange("db.database", p.defaults.database);
			if (!data.db?.username) onChange("db.username", p.defaults.username);
		});
	});

	function handleParsed(result: Partial<PostgresFormData>) {
		if (result.host !== undefined) onChange("db.host", result.host);
		if (result.port !== undefined) onChange("db.port", result.port);
		if (result.username !== undefined) onChange("db.username", result.username);
		if (result.password !== undefined) onChange("db.password", result.password);
		if (result.database !== undefined) onChange("db.database", result.database);
	}
</script>

{#if provider}
	<div class="flex h-full">
		<!-- Left column: connection string + form fields -->
		<div class="flex-1 overflow-y-auto pr-4">
			<ConnectionStringInput onParse={handleParsed} />

			<!-- Divider -->
			<div class="flex items-center gap-3 mb-4">
				<div class="flex-1 h-px bg-border"></div>
				<div class="text-xs text-muted-foreground">or fill manually</div>
				<div class="flex-1 h-px bg-border"></div>
			</div>

			<PostgresForm {data} {onChange} {hideFooter} />
		</div>

		<!-- Right column: guide panel -->
		<ProviderGuidePanel {provider} />
	</div>
{:else}
	<!-- Fallback: unknown provider, render standard postgres form -->
	<PostgresForm {data} {onChange} />
{/if}
