<script lang="ts">
  let { children } = $props();
  import "../app.css";
  	import { getCurrentWindow } from "@tauri-apps/api/window";
  import ThemeProvider from "$lib/providers/ThemeProvider.svelte";
  import Titlebar from "$lib/Titlebar.svelte";
  import { onMount } from "svelte";
    import LoadingOverlay from "$lib/LoadingOverlay.svelte";

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
  <Titlebar isFullScreen={isFullScreen} />
  
  {@render children()}
</ThemeProvider>
