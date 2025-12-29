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

    // Command Registry
    commands = new Map<string, { id: string; label: string; execute: () => void }>();
    keybindings = new Map<string, string>(); // Key combo -> Action ID

    constructor() {
        // We initialize the label immediately as it's synchronous
        try {
            this.label = getCurrentWindow().label;
        } catch (e) {
            console.error("[WindowStateStore] Failed to get current window label:", e);
        }
        this.registerDefaultCommands();
    }

    private registerDefaultCommands() {
        const isMac = navigator.platform.toUpperCase().indexOf('MAC') >= 0;
        const mod = isMac ? "Meta" : "Control";

        // Register Commands
        this.registerCommand({
            id: "workbench.action.toggleSidebarLeft",
            label: "Toggle Left Sidebar",
            execute: () => { this.layout.left = !this.layout.left; }
        });

        this.registerCommand({
            id: "workbench.action.toggleSidebarRight",
            label: "Toggle Right Sidebar",
            execute: () => { this.layout.right = !this.layout.right; }
        });

        this.registerCommand({
            id: "workbench.action.togglePanel",
            label: "Toggle Bottom Panel",
            execute: () => { this.layout.bottom = !this.layout.bottom; }
        });

        // Register Keybindings
        this.registerKeybinding("workbench.action.toggleSidebarLeft", `${mod}+j`);
        this.registerKeybinding("workbench.action.toggleSidebarRight", `${mod}+k`);
        this.registerKeybinding("workbench.action.togglePanel", `${mod}+n`);
    }

    registerCommand(command: { id: string; label: string; execute: () => void }) {
        this.commands.set(command.id, command);
    }

    registerKeybinding(commandId: string, keybinding: string) {
        this.keybindings.set(keybinding.toLowerCase(), commandId);
    }

    executeCommand(commandId: string) {
        const command = this.commands.get(commandId);
        if (command) {
            command.execute();
        } else {
            console.warn(`[WindowStateStore] Command not found: ${commandId}`);
        }
    }

    handleKeydown(event: KeyboardEvent) {
        const modifiers = [];
        if (event.metaKey) modifiers.push("meta");
        if (event.ctrlKey) modifiers.push("control");
        if (event.altKey) modifiers.push("alt");
        if (event.shiftKey) modifiers.push("shift");

        const key = event.key.toLowerCase();
        // Ignore modifier-only keydowns
        if (["meta", "control", "alt", "shift"].includes(key)) return;

        const combo = [...modifiers, key].join("+").toLowerCase();

        const commandId = this.keybindings.get(combo);
        if (commandId) {
            event.preventDefault();
            console.log(`[WindowStateStore] Executing shortcuts: ${combo} -> ${commandId}`);
            this.executeCommand(commandId);
        }
    }

    async init() {
        try {
            console.log(`[WindowStateStore] Initializing for window: ${this.label}`);

            // Initial check for existing windows
            const windows = await getAllWindows();
            this.settingsWindowOpen = windows.some(w => w.label === "appearance-window");
            this.datasourceWindowOpen = windows.some(w => w.label === "datasource-window");
            // ... listeners


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
