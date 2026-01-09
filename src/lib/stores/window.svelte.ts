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
import { persistenceStore } from "./persistence.svelte";

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
        }
    },
    {
        id: "workbench.action.toggleSidebarRight",
        label: "Toggle Right Sidebar",
        defaultKeybinding: { mac: "Meta+k", win: "Control+k" },
        execute: (s) => {
            s.layout.right = !s.layout.right;
        }
    },
    {
        id: "workbench.action.togglePanel",
        label: "Toggle Bottom Panel",
        defaultKeybinding: { mac: "Meta+n", win: "Control+n" },
        execute: (s) => {
            s.layout.bottom = !s.layout.bottom;
        }
    },
    {
        id: "workbench.action.newWindow",
        label: "New Window",
        defaultKeybinding: { mac: "Meta+Shift+n", win: "Control+Shift+n" },
        execute: async () => {
            try {
                await invoke("create_new_window", { connectionId: null });
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
    initialized = $state(false); // Track if settings have been loaded and applied
    // Layout State (visibility)
    // We use a getter/setter proxy pattern to settingsStore for automatic persistence
    get layout() {
        const self = this;
        return {
            get left() { return settingsStore.sidebarLeftVisible; },
            set left(v: boolean) { settingsStore.sidebarLeftVisible = v; },
            get right() { return settingsStore.sidebarRightVisible; },
            set right(v: boolean) { settingsStore.sidebarRightVisible = v; },
            get bottom() { return settingsStore.sidebarBottomVisible; },
            set bottom(v: boolean) { settingsStore.sidebarBottomVisible = v; },
            get showSqlEditor() { return self._showSqlEditor; },
            set showSqlEditor(v: boolean) { self._showSqlEditor = v; }
        };
    }

    // internal state for non-persisted layout flags
    private _showSqlEditor = $state(false);

    // Layout Ratios (proxied to settingsStore for debounced persistence)
    get layoutRatios() {
        return {
            get left() { return settingsStore.sidebarLeftRatio; },
            set left(v: number) { settingsStore.sidebarLeftRatio = v; },
            get right() { return settingsStore.sidebarRightRatio; },
            set right(v: number) { settingsStore.sidebarRightRatio = v; },
            get bottom() { return settingsStore.sidebarBottomRatio; },
            set bottom(v: number) { settingsStore.sidebarBottomRatio = v; }
        };
    }

    // Session Management
    sessions = $state<Session[]>([]);
    activeSessionId = $state<string | null>(null);

    get activeSession() {
        return this.sessions.find(s => s.id === this.activeSessionId) || null;
    }

    private saveTimeout: ReturnType<typeof setTimeout> | null = null;

    requestSave() {
        if (this.saveTimeout) clearTimeout(this.saveTimeout);
        this.saveTimeout = setTimeout(() => {
            persistenceStore.saveSessionState(this.sessions, this.activeSessionId, this.label);
        }, 1000); // 1s debounce
    }

    startSession(connection: Connection) {
        // Check if a session for this connection already exists
        const existingSession = this.sessions.find(s => s.connectionId === connection.id);
        if (existingSession) {
            // Reuse existing session instead of creating duplicate
            this.activateSession(existingSession.id);
            return;
        }

        const newSession = new Session(
            crypto.randomUUID(),
            connection,
            this.label,
            () => this.requestSave()
        );
        this.sessions.push(newSession);
        this.activateSession(newSession.id);
        this.requestSave();
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
            this.requestSave();
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

    // Layout ratio update methods (proxies to layoutRatios setters)
    setLeftRatio(ratio: number) {
        this.layoutRatios.left = ratio;
    }

    setRightRatio(ratio: number) {
        this.layoutRatios.right = ratio;
    }

    setBottomRatio(ratio: number) {
        this.layoutRatios.bottom = ratio;
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
            this.unlistenFunctions.push(settingsStore.init(this.label));
            this.unlistenFunctions.push(themeStore.init());

            // Wait for settings to load
            await settingsStore.waitForInit();

            // Load persisted sessions
            const persistedState = await persistenceStore.loadSessionState(this.label);
            if (persistedState && persistedState.sessions.length > 0) {
                console.log("[WindowStateStore] Hydrating sessions from persistence", persistedState);

                // Reconstruct sessions
                for (const pSession of persistedState.sessions) {
                    // Find connection object from loaded connections if possible, or wait?
                    // Issue: connections might not be loaded yet. 
                    // Ideally we should wait for connections, but they are async. 
                    // For now, we'll try to reconstruct essential parts.

                    // Note: We need the full Connection object to create a valid Session.
                    // If we don't have it, the session might be invalid.
                    // Let's assume we can lazily match it later or just use ID for now.
                }

                // Actually, we should wait for connections to be loaded before hydrating sessions 
                // to ensure we have valid connection objects.
                if (this.connections.length === 0) {
                    await this.loadConnections();
                }

                this.sessions = persistedState.sessions.map(s => {
                    const conn = this.connections.find(c => c.id === s.connectionId);
                    if (!conn) {
                        console.warn(`[WindowStateStore] Could not find connection for persisted session ${s.id}`);
                        // Create a placeholder connection if needed or skip? 
                        // Better to skip invalid sessions to avoid errors.
                        return null;
                    }
                    const session = new Session(
                        s.id,
                        conn,
                        s.windowLabel,
                        () => this.requestSave()
                    );

                    // Hydrate views
                    session.views = s.views; // ViewState interfaces match
                    session.activeViewId = s.activeViewId;
                    if (s.explorerState?.expanded) {
                        session.explorerState.expanded = new Set(s.explorerState.expanded);
                    }
                    return session;
                }).filter(s => s !== null) as Session[];

                if (persistedState.lastActiveSessionId) {
                    this.activeSessionId = persistedState.lastActiveSessionId;
                    // Connect if active
                    if (this.activeSession) {
                        // Don't auto-connect immediately to avoid spamming? 
                        // Or do we? User expects it.
                        // Let's activate checking connection status
                        this.activateSession(this.activeSessionId);
                    }
                }
            }

            this.initialized = true;
            console.log("[WindowStateStore] Layout initialized from settings");

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
