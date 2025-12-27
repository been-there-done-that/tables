<script lang="ts">
	let { children } = $props();
	import "../app.css";
	import { getCurrentWindow } from "@tauri-apps/api/window";
	import ThemeProvider from "$lib/providers/ThemeProvider.svelte";
	import Titlebar from "$lib/Titlebar.svelte";
	import { onMount } from "svelte";
	import LoadingOverlay from "$lib/LoadingOverlay.svelte";
	import NotificationContainer from "$lib/components/notifications/NotificationContainer.svelte";

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

			// Do you have permission to send a notification?
			// let permissionGranted = await isPermissionGranted();

			// If not we need to request it
			// if (!permissionGranted) {
			// 	const permission = await requestPermission();
			// 	permissionGranted = permission === "granted";
			// }

			// Once permission has been granted we can send the notification
			// if (permissionGranted) {
			// se?ndNotification({ title: "Tauri", body: "Tauri is awesome!" });
			// }
		};

		setup();

		return () => {
			if (unlisten) {
				unlisten();
			}
		};
	});
</script>

<ThemeProvider>
	<LoadingOverlay />
	<NotificationContainer />
	<div class="flex h-screen w-full flex-col overflow-hidden bg-background">
		<Titlebar {isFullScreen} />
		<div class="h-8 shrink-0" aria-hidden="true"></div>
		<div class="flex-1 w-full min-h-0 overflow-hidden">
			{@render children()}
		</div>
	</div>
</ThemeProvider>
