<script lang="ts">
	let { children } = $props();
	import "../app.css";
	import { getCurrentWindow } from "@tauri-apps/api/window";
	import ThemeProvider from "$lib/providers/ThemeProvider.svelte";
	import Titlebar from "$lib/Titlebar.svelte";
	import { windowState } from "$lib/stores/window.svelte";
	import { schemaStore } from "$lib/stores/schema.svelte";
	import { metrics } from "$lib/stores/metrics.svelte";
	import { onMount } from "svelte";
	import LoadingOverlay from "$lib/LoadingOverlay.svelte";
	import { Toaster } from "svelte-sonner";
	import SuccessIcon from "$lib/svg/SuccessMark.svelte";
	import ErrorIcon from "$lib/svg/ErrorMark.svelte";
	import InfoIcon from "$lib/svg/InfoMark.svelte";
	import WarnIcon from "$lib/svg/WarnMark.svelte";
	import { getAllWindows } from "@tauri-apps/api/window";
	import { page } from "$app/stores";

	const appWindow = getCurrentWindow();
	console.log(" [Layout] Script initialized. Window Label:", appWindow.label);

	let isFullScreen = $state(false);

	async function checkFullScreen() {
		isFullScreen = await appWindow.isFullscreen();
	}

	onMount(() => {
		console.log(" [Layout] onMount triggered for:", appWindow.label);
		let unlisten: () => void;

		const setup = async () => {
			console.log("[Layout] Setup running for window:", appWindow.label);

			// Skip initialization for splash window
			if (appWindow.label === "splash") {
				console.log(
					"[Layout] Skipping initialization for splash window",
				);
				return;
			}

			try {
				await checkFullScreen();
				unlisten = await appWindow.onResized(checkFullScreen);
				await windowState.init();
				// Initialize schema store with the window label to restore sessions
				await schemaStore.initialize(windowState.label);
				await metrics.init();
			} catch (e) {
				console.error("[Layout] Initialization failed:", e);
			}

			// Multi-window splash logic:
			// If this is the main window (which starts hidden), show it and close splash
			if (appWindow.label === "main") {
				console.log(
					"[Layout] Main window detected. Initiating splash close sequence.",
				);
				// Optional: Small delay if you want to ensure DOM is painted,
				// but usually safe to show now or after a minimal timeout.
				setTimeout(async () => {
					console.log("[Layout] Showing main window now.");
					await appWindow.show();
					await appWindow.setFocus();

					// Close splash window
					try {
						const windows = await getAllWindows();
						const splash = windows.find(
							(w) => w.label === "splash",
						);
						if (splash) {
							console.log(
								"[Layout] Found splash window, closing it.",
							);
							await splash.close();
						} else {
							console.warn(
								"[Layout] Splash window not found via getAllWindows.",
							);
						}
					} catch (e) {
						console.error(
							"[Layout] Failed to close splash window",
							e,
						);
					}
				}, 500); // 500ms delay to let Svelte hydrate fully
			}
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

{#if $page.url.pathname === "/splash"}
	<ThemeProvider>
		{@render children()}
	</ThemeProvider>
{:else}
	<ThemeProvider>
		<LoadingOverlay />
		<div
			class="flex h-screen w-full flex-col overflow-hidden bg-background"
		>
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
{/if}
