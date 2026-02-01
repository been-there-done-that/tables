<script lang="ts">
	let { children } = $props();
	import "../app.css";
	import { getCurrentWindow } from "@tauri-apps/api/window";
	import ThemeProvider from "$lib/providers/ThemeProvider.svelte";
	import Titlebar from "$lib/Titlebar.svelte";
	import { windowState } from "$lib/stores/window.svelte";
	import { schemaStore } from "$lib/stores/schema.svelte";
	import { settingsStore } from "$lib/stores/settings.svelte";
	import { onMount } from "svelte";
	import { listen } from "@tauri-apps/api/event";
	import LoadingOverlay from "$lib/LoadingOverlay.svelte";
	import CloseConfirmDialog from "$lib/components/CloseConfirmDialog.svelte";
	import { Toaster } from "svelte-sonner";
	import SuccessIcon from "$lib/svg/SuccessMark.svelte";
	import ErrorIcon from "$lib/svg/ErrorMark.svelte";
	import InfoIcon from "$lib/svg/InfoMark.svelte";
	import WarnIcon from "$lib/svg/WarnMark.svelte";

	import { resolveClipboardApi } from "$lib/components/table/clipboardUtils";

	import AgentChat from "$lib/components/agent/AgentChat.svelte";

	const appWindow = getCurrentWindow();

	let isFullScreen = $state(false);

	async function checkFullScreen() {
		isFullScreen = await appWindow.isFullscreen();
	}

	onMount(() => {
		let unlisten: () => void;

		const setup = async () => {
			// Pre-resolve/warm-up clipboard API globally so async checks don't break paste gestures
			resolveClipboardApi().catch((e) =>
				console.debug("[Layout] Clipboard warm-up failed", e),
			);

			await checkFullScreen();
			unlisten = await appWindow.onResized(checkFullScreen);
			await windowState.init();
			// Initialize schema store with the window label to restore sessions
			await schemaStore.initialize(windowState.label);
		};

		setup();

		return () => {
			if (unlisten) {
				unlisten();
			}
			windowState.cleanup();
		};
	});
</script>

<svelte:window onkeydown={(e) => windowState.handleKeydown(e)} />

<ThemeProvider>
	<LoadingOverlay />
	<CloseConfirmDialog />
	<div class="flex h-screen w-full flex-col overflow-hidden bg-background">
		<Titlebar {isFullScreen} />
		<div class="h-8 shrink-0" aria-hidden="true"></div>
		<div class="flex-1 w-full min-h-0 overflow-hidden">
			{@render children()}
		</div>
	</div>

	{#if windowState.agentConsoleOpen}
		<div
			class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm"
		>
			<div class="relative">
				<button
					onclick={() => (windowState.agentConsoleOpen = false)}
					class="absolute -right-4 -top-4 z-10 border-4 border-black bg-white p-2 text-black hover:bg-red-500 hover:text-white transition-colors shadow-[4px_4px_0px_0px_rgba(0,0,0,1)]"
				>
					<svg
						xmlns="http://www.w3.org/2000/svg"
						width="24"
						height="24"
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="3"
						stroke-linecap="round"
						stroke-linejoin="round"
						><line x1="18" y1="6" x2="6" y2="18"></line><line
							x1="6"
							y1="6"
							x2="18"
							y2="18"
						></line></svg
					>
				</button>
				<AgentChat />
			</div>
		</div>
	{/if}

	<Toaster position="bottom-right" visibleToasts={40} expand={true}>
		{#snippet successIcon()}
			<SuccessIcon />
		{/snippet}
		{#snippet errorIcon()}
			<ErrorIcon />
		{/snippet}
		{#snippet infoIcon()}
			<InfoIcon />
		{/snippet}
		{#snippet warningIcon()}
			<WarnIcon />
		{/snippet}
	</Toaster>
</ThemeProvider>
