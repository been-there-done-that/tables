import { getCurrentWindow, getAllWindows } from "@tauri-apps/api/window";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { METRICS } from "$lib/constants";
import { listConnections } from "$lib/commands/client";
import type { Connection } from "$lib/commands/types";
import { schemaStore } from "./schema.svelte";
import { settingsStore } from "./settings.svelte";
import { themeStore } from "$lib/commands/stores.svelte";
import { Session } from "./session.svelte";

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
        execute: (s) => {
            s.layout.left = !s.layout.left;
            settingsStore.sidebarLeftVisible = s.layout.left;
        }
    },
    {
        id: "workbench.action.toggleSidebarRight",
        label: "Toggle Right Sidebar",
        defaultKeybinding: { mac: "Meta+k", win: "Control+k" },
        execute: (s) => {
            s.layout.right = !s.layout.right;
            settingsStore.sidebarRightVisible = s.layout.right;
        }
    },
    {
        id: "workbench.action.togglePanel",
        label: "Toggle Bottom Panel",
        defaultKeybinding: { mac: "Meta+n", win: "Control+n" },
        execute: (s) => {
            s.layout.bottom = !s.layout.bottom;
            settingsStore.sidebarBottomVisible = s.layout.bottom;
        }
    },
    {
        id: "workbench.action.newWindow",
        label: "New Window",
        defaultKeybinding: { mac: "Meta+Shift+n", win: "Control+Shift+n" },
        execute: async () => {
            try {
                await invoke("create_new_window");
            } catch (e) {
                console.error("Failed to create new window:", e);
            }
        }
    },
    {
        id: "workbench.action.openDatasource",
        label: "Open Data Sources",
        defaultKeybinding: { mac: "Meta+Shift+d", win: "Control+Shift+d" },
        execute: async () => {
            try {
                await invoke("open_datasource_window");
            } catch (e) {
                console.error("Failed to open datasource window:", e);
            }
        }
    }
];

class WindowStateStore {
    label = $state("main");
    settingsWindowOpen = $state(false);
    datasourceWindowOpen = $state(false);
    // Layout State (visibility)
    layout = $state({
        left: true,
        right: false,
        bottom: false,
        showSqlEditor: false
    });

    // Layout Ratios (persisted separately)
    layoutRatios = $state({
        left: 0.2,
        right: 0.75,
        bottom: 0.7
    });

    // Session Management
    sessions = $state<Session[]>([]);
    activeSessionId = $state<string | null>(null);

    get activeSession() {
        return this.sessions.find(s => s.id === this.activeSessionId) || null;
    }

    startSession(connection: Connection) {
        // Check if a session for this connection already exists
        const existingSession = this.sessions.find(s => s.connectionId === connection.id);
        if (existingSession) {
            // Reuse existing session instead of creating duplicate
            this.activateSession(existingSession.id);
            return;
        }

        const newSession = new Session(crypto.randomUUID(), connection);
        this.sessions.push(newSession);
        this.activateSession(newSession.id);
    }

    activateSession(sessionId: string) {
        const session = this.sessions.find(s => s.id === sessionId);
        if (session) {
            this.activeSessionId = sessionId;
            // Sync with schemaStore if needed (but don't interrupt active connection)
            if (session.connection &&
                schemaStore.activeConnection?.id !== session.connection.id &&
                schemaStore.status !== "connecting") {
                schemaStore.connect(session.connection);
            }
        }
    }

    closeSession(sessionId: string) {
        const index = this.sessions.findIndex(s => s.id === sessionId);
        if (index === -1) return;

        this.sessions.splice(index, 1);

        if (this.activeSessionId === sessionId) {
            // Activate the neighbor or null
            const newActive = this.sessions[index] || this.sessions[index - 1];
            if (newActive) {
                this.activateSession(newActive.id);
            } else {
                this.activeSessionId = null;
                schemaStore.disconnect();
            }
        }
    }

    // Layout ratio update methods (with persistence)
    setLeftRatio(ratio: number) {
        this.layoutRatios.left = ratio;
        settingsStore.sidebarLeftRatio = ratio;
    }

    setRightRatio(ratio: number) {
        this.layoutRatios.right = ratio;
        settingsStore.sidebarRightRatio = ratio;
    }

    setBottomRatio(ratio: number) {
        this.layoutRatios.bottom = ratio;
        settingsStore.sidebarBottomRatio = ratio;
    }

    // Metrics (Moved to metrics.svelte.ts)

    // Global Connections (shared across components in this window)
    connections = $state<Connection[]>([]);
    loadingConnections = $state(false);

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

            // Listen for system metrics - Handled by metrics.svelte.ts now
            // const unlistenMetrics = await listen("metrics:update", (event) => { ... });

            // Initialize global settings/theme listeners
            this.unlistenFunctions.push(settingsStore.init());
            this.unlistenFunctions.push(themeStore.init());

            // Wait for settings to load, then apply layout
            await settingsStore.waitForInit();
            this.layout.left = settingsStore.sidebarLeftVisible;
            this.layout.right = settingsStore.sidebarRightVisible;
            this.layout.bottom = settingsStore.sidebarBottomVisible;
            this.layoutRatios.left = settingsStore.sidebarLeftRatio;
            this.layoutRatios.right = settingsStore.sidebarRightRatio;
            this.layoutRatios.bottom = settingsStore.sidebarBottomRatio;
            console.log("[WindowStateStore] Layout restored from settings:", this.layout, this.layoutRatios);

        } catch (e) {
            console.error("[WindowStateStore] Failed to setup window listeners:", e);
        }
    }

    cleanup() {
        this.unlistenFunctions.forEach(unlisten => unlisten());
        this.unlistenFunctions = [];
    }
    async loadConnections() {
        this.loadingConnections = true;
        try {
            const response = await listConnections();
            if (response.success && response.data) {
                this.connections = response.data;
            } else {
                console.error("Failed to load connections:", response.error);
            }
        } catch (e) {
            console.error("Failed to load connections:", e);
        } finally {
            this.loadingConnections = false;
        }
    }
}

export const windowState = new WindowStateStore();
