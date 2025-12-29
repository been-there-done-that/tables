import { getCurrentWindow, getAllWindows } from "@tauri-apps/api/window";
import { listen } from "@tauri-apps/api/event";

class WindowStateStore {
    label = $state("main");
    settingsWindowOpen = $state(false);
    datasourceWindowOpen = $state(false);
    layout = $state({
        left: true,
        right: true,
        bottom: true
    });
    private unlistenFunctions: (() => void)[] = [];

    constructor() {
        // We initialize the label immediately as it's synchronous
        try {
            this.label = getCurrentWindow().label;
        } catch (e) {
            console.error("[WindowStateStore] Failed to get current window label:", e);
        }
    }

    async init() {
        try {
            console.log(`[WindowStateStore] Initializing for window: ${this.label}`);

            // Initial check for existing windows
            const windows = await getAllWindows();
            this.settingsWindowOpen = windows.some(w => w.label === "appearance-window");
            this.datasourceWindowOpen = windows.some(w => w.label === "datasource-window");

            // Listen for new windows being created
            const unlistenCreated = await listen("window-created", (event) => {
                const label = event.payload as string;
                console.log(`[WindowStateStore] Window created: ${label}`);
                if (label === "appearance-window") this.settingsWindowOpen = true;
                if (label === "datasource-window") this.datasourceWindowOpen = true;
            });
            this.unlistenFunctions.push(unlistenCreated);

            // Listen for windows being destroyed
            const unlistenDestroyed = await listen("window-destroyed", (event) => {
                const label = event.payload as string;
                console.log(`[WindowStateStore] Window destroyed: ${label}`);
                if (label === "appearance-window") this.settingsWindowOpen = false;
                if (label === "datasource-window") this.datasourceWindowOpen = false;
            });
            this.unlistenFunctions.push(unlistenDestroyed);

        } catch (e) {
            console.error("[WindowStateStore] Failed to setup window listeners:", e);
        }
    }

    cleanup() {
        this.unlistenFunctions.forEach(unlisten => unlisten());
        this.unlistenFunctions = [];
    }
}

export const windowState = new WindowStateStore();
