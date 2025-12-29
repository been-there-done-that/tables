import { getCurrentWindow, getAllWindows } from "@tauri-apps/api/window";
import { listen } from "@tauri-apps/api/event";

export interface CommandConfig {
    id: string;
    label: string;
    defaultKeybinding: {
        mac: string;
        win: string;
    };
    execute: (store: WindowStateStore) => void;
}

const COMMANDS: CommandConfig[] = [
    {
        id: "workbench.action.toggleSidebarLeft",
        label: "Toggle Left Sidebar",
        defaultKeybinding: { mac: "Meta+j", win: "Control+j" },
        execute: (s) => { s.layout.left = !s.layout.left; }
    },
    {
        id: "workbench.action.toggleSidebarRight",
        label: "Toggle Right Sidebar",
        defaultKeybinding: { mac: "Meta+k", win: "Control+k" },
        execute: (s) => { s.layout.right = !s.layout.right; }
    },
    {
        id: "workbench.action.togglePanel",
        label: "Toggle Bottom Panel",
        defaultKeybinding: { mac: "Meta+n", win: "Control+n" },
        execute: (s) => { s.layout.bottom = !s.layout.bottom; }
    }
];

class WindowStateStore {
    label = $state("main");
    settingsWindowOpen = $state(false);
    datasourceWindowOpen = $state(false);
    // Layout State
    layout = $state({
        left: true,
        right: true,
        bottom: true
    });

    // Metrics State
    metrics = $state<{ cpu_percent: number; pid: number; threads: number } | null>(null);
    cpuHistory = $state<number[]>([]);

    private unlistenFunctions: (() => void)[] = [];

    // Command Registry
    commands = new Map<string, CommandConfig>();
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
        const isMac = navigator.userAgent.includes("Mac");

        COMMANDS.forEach(cmd => {
            this.commands.set(cmd.id, cmd);
            const keybinding = isMac ? cmd.defaultKeybinding.mac : cmd.defaultKeybinding.win;
            if (keybinding) {
                this.registerKeybinding(cmd.id, keybinding);
            }
        });
    }

    registerKeybinding(commandId: string, keybinding: string) {
        this.keybindings.set(keybinding.toLowerCase(), commandId);
    }

    executeCommand(commandId: string) {
        const command = this.commands.get(commandId);
        if (command) {
            command.execute(this);
        } else {
            console.warn(`[WindowStateStore] Command not found: ${commandId}`);
        }
    }

    // Public method for UI to get display string
    formatKeybinding(commandId: string): string {
        const isMac = navigator.userAgent.includes("Mac");
        // Find keybinding for this command
        let foundKey: string | undefined;
        for (const [key, id] of this.keybindings.entries()) {
            if (id === commandId) {
                foundKey = key;
                break;
            }
        }

        if (!foundKey) return "None";

        // Formatting logic
        return foundKey
            .split("+")
            .map(part => {
                part = part.trim();
                if (part === "meta") return isMac ? "⌘" : "Ctrl";
                if (part === "control") return "Ctrl";
                if (part === "alt") return isMac ? "⌥" : "Alt";
                if (part === "shift") return isMac ? "⇧" : "Shift";
                return part.toUpperCase();
            })
            .join(isMac ? " " : "+"); // Mac uses spaces (⌘ ⇧ P), Win uses + (Ctrl+Shift+P)
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

            // Listen for system metrics
            const unlistenMetrics = await listen("metrics:update", (event) => {
                const m = event.payload as { cpu_percent: number; pid: number; threads: number };
                this.metrics = m;

                // Update history buffer (keep last 100 samples to allow flexibility in UI)
                const next = [...this.cpuHistory, m.cpu_percent];
                if (next.length > 100) next.shift();
                this.cpuHistory = next;
            });
            this.unlistenFunctions.push(unlistenMetrics);

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
