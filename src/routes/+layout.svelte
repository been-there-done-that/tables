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
	import { Toaster, toast } from "svelte-sonner";
	import SuccessIcon from "$lib/svg/SuccessMark.svelte";
	import ErrorIcon from "$lib/svg/ErrorMark.svelte";
	import InfoIcon from "$lib/svg/InfoMark.svelte";
	import WarnIcon from "$lib/svg/WarnMark.svelte";

	import { resolveClipboardApi } from "$lib/components/table/clipboardUtils";
	import "$lib/stores/harness.svelte"; // eagerly register harness://ready listener

	const appWindow = getCurrentWindow();

	let isFullScreen = $state(false);

	async function checkFullScreen() {
		isFullScreen = await appWindow.isFullscreen();
	}

	onMount(() => {
		let unlisten: () => void;
		let unlistenFeedback: () => void;

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

			unlistenFeedback = await listen<{ issue_url: string }>(
				"feedback://submitted",
				(event) => {
					toast.success("Feedback submitted!", {
						description: "Thank you — your issue has been created.",
						action: {
							label: "View",
							onClick: () => {
								if (event.payload.issue_url) {
									window.open(event.payload.issue_url, "_blank");
								}
							},
						},
					});
				}
			);
		};

		setup();

		return () => {
			if (unlisten) {
				unlisten();
			}
			if (unlistenFeedback) {
				unlistenFeedback();
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
