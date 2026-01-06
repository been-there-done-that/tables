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
	import { Toaster } from "svelte-sonner";
	import SuccessIcon from "$lib/svg/SuccessMark.svelte";
	import ErrorIcon from "$lib/svg/ErrorMark.svelte";
	import InfoIcon from "$lib/svg/InfoMark.svelte";
	import WarnIcon from "$lib/svg/WarnMark.svelte";

	const appWindow = getCurrentWindow();

	let isFullScreen = $state(false);

	async function checkFullScreen() {
		isFullScreen = await appWindow.isFullscreen();
	}

	onMount(() => {
		let unlisten: () => void;

		const setup = async () => {
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

	// Apply selected font globally to the body
	$effect(() => {
		const family = settingsStore.editorFontFamily;
		const safeFamily = family.includes(" ") ? `"${family}"` : family;
		// Override body font to ensure it applies everywhere (EditorHome, Sidebar, etc.)
		document.body.style.fontFamily = safeFamily;
	});

	// Listen for font changes from other windows (e.g. settings window)
	onMount(() => {
		let unlisten: () => void;

		const setup = async () => {
			unlisten = await listen<string>("font-changed", (event) => {
				console.log("Font changed event received:", event.payload);
				// Update the store so the effect above runs and updates the body style
				settingsStore.editorFontFamily = event.payload;
			});
		};
		setup();

		return () => {
			if (unlisten) {
				unlisten();
			}
		};
	});
</script>

<svelte:window onkeydown={(e) => windowState.handleKeydown(e)} />

<ThemeProvider>
	<LoadingOverlay />
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
