import { getCurrentWindow, getAllWindows } from "@tauri-apps/api/window";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { listConnections } from "$lib/commands/client";
import type { Connection } from "$lib/commands/types";
import { schemaStore } from "./schema.svelte";
import { settingsStore } from "./settings.svelte";
import { themeStore } from "$lib/commands/stores.svelte";
import { Session } from "./session.svelte";
import { persistenceStore, type PersistedState } from "./persistence.svelte";
import { toast } from "svelte-sonner";

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
    },
    {
        id: "workbench.action.focusSidebar",
        label: "Focus Sidebar",
        defaultKeybinding: { mac: "Control+1", win: "Control+1" },
        execute: () => {
            console.log("[Shortcut] Executing Focus Sidebar (Ctrl+1)");
            // First open it if it's closed
            settingsStore.sidebarLeftVisible = true;
            // Focus search input or first tree item
            const sidebar = document.querySelector("#explorer-sidebar");
            if (!sidebar) {
                console.error("[Shortcut] Explorer sidebar not found in DOM");
                return;
            }

            // A. Prefer already selected item
            const selected = sidebar.querySelector('[data-selected="true"]') as HTMLElement;
            if (selected) {
                console.log("[Shortcut] Focusing selected sidebar item");
                selected.focus();
                // Double check if focus actually moved away from whatever was focused (like the table)
                if (document.activeElement !== selected && !sidebar.contains(document.activeElement)) {
                    console.warn("[Shortcut] Selected node focus failed, falling back to sidebar container");
                    (sidebar.querySelector('[role="tree"]') as HTMLElement)?.focus();
                }
                toast.info("Focused Sidebar");
                return;
            }

            // B. Then search input
            const searchInput = sidebar.querySelector("input") as HTMLInputElement;
            if (searchInput) {
                console.log("[Shortcut] Focusing sidebar search input");
                searchInput.focus();
                toast.info("Focused Sidebar Search");
                return;
            }

            // C. Fallback: tree or first focusable
            const tree = sidebar.querySelector('[role="tree"]') as HTMLElement;
            const firstItem = tree || (sidebar.querySelector('[tabindex="0"]') as HTMLElement);
            if (firstItem) {
                console.log("[Shortcut] Focusing sidebar element:", firstItem);
                firstItem.focus();
                toast.info("Focused Sidebar");
            }
        }
    },
    {
        id: "workbench.action.focusMain",
        label: "Focus Main Content",
        defaultKeybinding: { mac: "Control+2", win: "Control+2" },
        execute: () => {
            console.log("[Shortcut] Executing Focus Main (Ctrl+2)");
            const mainContent = document.querySelector("#main-content-area");
            if (!mainContent) {
                console.error("[Shortcut] Main content area not found in DOM");
                return;
            }

            // 1. Try to find the cell that is visually focused (Amber ring)
            const focusedCell = mainContent.querySelector('[data-is-focused="true"]') as HTMLElement;
            if (focusedCell) {
                console.log("[Shortcut] Restoring focus to previous cell:", focusedCell.dataset.rowIndex, focusedCell.dataset.colIndex);
                focusedCell.focus();
                toast.info("Restored Table Focus");
                return;
            }

            // 2. Fallback to first visible cell (0,0 or just any cell)
            const firstCell = mainContent.querySelector('[data-row-index="0"][data-col-index="0"]') as HTMLElement;
            if (firstCell) {
                console.log("[Shortcut] Focusing first visible cell (0,0)");
                firstCell.focus();
                toast.info("Focused Table Start");
                return;
            }

            // 3. Last fallback: any focusable in main area (editors, table container)
            const focusable = mainContent.querySelector('[tabindex="0"], .monaco-editor, [data-row-index]') as HTMLElement;
            console.log("[Shortcut] Falling back to general focusable:", focusable);
            if (focusable) {
                focusable.focus();
                toast.info("Focused Main Content");
            }
        }
    }
];

class WindowStateStore {
    label = $state("main");
    settingsWindowOpen = $state(false);
    datasourceWindowOpen = $state(false);
    initialized = $state(false);

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

    private _showSqlEditor = $state(false);

    // activeRightPanel: "logs" | "properties" | null
    activeRightPanel = $state<string | null>(null);

    openRightPanel(view: string) {
        this.activeRightPanel = view;
        this.layout.right = true;
    }

    closeRightPanel() {
        this.layout.right = false;
        // Optional: clear the view state or keep it for next time
        // this.activeRightPanel = null; 
    }

    toggleRightPanel(view: string) {
        if (this.activeRightPanel === view && this.layout.right) {
            this.closeRightPanel();
        } else {
            this.openRightPanel(view);
        }
    }

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

    sessions = $state<Session[]>([]);
    activeSessionId = $state<string | null>(null);

    get activeSession() {
        return this.sessions.find(s => s.id === this.activeSessionId) || null;
    }

    private saveTimeout: ReturnType<typeof setTimeout> | null = null;

    requestSave() {
        if (this.saveTimeout) clearTimeout(this.saveTimeout);
        this.saveTimeout = setTimeout(() => {
            const activeConnId = this.activeSession?.connectionId;
            persistenceStore.saveSessionState(this.sessions, this.activeSessionId, this.label, activeConnId);
        }, 1000); // 1s debounce
    }

    startSession(connection: Connection) {
        const existingSession = this.sessions.find(s => s.connectionId === connection.id);
        if (existingSession) {
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

    reset() {
        this.activeSessionId = null;
        this.settingsWindowOpen = false;
        this.datasourceWindowOpen = false;
        this.requestSave();
        console.log("[WindowStateStore] Workbench UI reset");
    }

    activateSession(sessionId: string) {
        const session = this.sessions.find(s => s.id === sessionId);
        if (session) {
            this.activeSessionId = sessionId;
            this.requestSave();
        }
    }

    async restoreForConnection(connection: Connection) {
        // 1. Stash current sessions to their respective connection keys
        if (this.sessions.length > 0) {
            console.log("[WindowStateStore] Stashing existing sessions before switch...");
            const sessionsByConn = new Map<string, Session[]>();

            // Group sessions by connection to ensure we save them to the correct buckets
            // (fixes issue where mixed sessions were saved to the wrong connection key)
            for (const s of this.sessions) {
                if (!sessionsByConn.has(s.connectionId)) {
                    sessionsByConn.set(s.connectionId, []);
                }
                sessionsByConn.get(s.connectionId)!.push(s);
            }

            for (const [connId, sessions] of sessionsByConn) {
                // Determine active session ID for this group (if applicable)
                const activeId = this.activeSessionId && sessions.some(s => s.id === this.activeSessionId)
                    ? this.activeSessionId
                    : null;

                await persistenceStore.saveSessionState(sessions, activeId, this.label, connId);
            }
        }

        // 2. Clear workspace completely
        this.sessions = [];
        this.activeSessionId = null;

        // 3. Restore state for the new connection
        console.log(`[WindowStateStore] Restoring sessions for connection: ${connection.id}`);
        const persistedState = await persistenceStore.loadSessionState(this.label, connection.id);

        if (persistedState && persistedState.sessions.length > 0) {
            this.hydrateSessions(persistedState, connection);
        } else {
            console.log("[WindowStateStore] No specific session state found for this connection, starting fresh");
            this.startSession(connection);
        }
    }

    private hydrateSessions(state: PersistedState, connection: Connection) {
        console.log("[WindowStateStore] Hydrating sessions", state);

        // Clear conflicting sessions for the same connection if any
        this.sessions = this.sessions.filter(s => s.connectionId !== connection.id);

        for (const pSession of state.sessions) {
            const session = new Session(
                pSession.id,
                connection,
                this.label,
                () => this.requestSave()
            );

            if (pSession.explorerState?.expanded) {
                session.explorerState.expanded = new Set(pSession.explorerState.expanded);
            }

            session.views = pSession.views.map((v: any) => ({
                id: v.id,
                type: v.type,
                title: v.title,
                data: v.data
            }));

            session.activeViewId = pSession.activeViewId;
            this.sessions.push(session);
        }

        if (state.lastActiveSessionId) {
            this.activateSession(state.lastActiveSessionId);
        } else if (this.sessions.length > 0) {
            this.activateSession(this.sessions[this.sessions.length - 1].id);
        }
    }

    closeSession(sessionId: string) {
        const index = this.sessions.findIndex(s => s.id === sessionId);
        if (index === -1) return;

        const session = this.sessions[index];

        // Clear expanded nodes in settings store for this connection
        // This ensures that when we reopen the connection, we start fresh (no expanded folders)
        settingsStore.setExpandedNodes(session.connectionId, []);

        this.sessions.splice(index, 1);

        if (this.activeSessionId === sessionId) {
            const newActive = this.sessions[index] || this.sessions[index - 1];
            if (newActive) {
                this.activateSession(newActive.id);
            } else {
                this.activeSessionId = null;
                schemaStore.disconnect();
            }
        }

        // Always save state after closing a session to ensure persistence is updated (or cleared)
        this.requestSave();
    }

    setLeftRatio(ratio: number) { this.layoutRatios.left = ratio; }
    setRightRatio(ratio: number) { this.layoutRatios.right = ratio; }
    setBottomRatio(ratio: number) { this.layoutRatios.bottom = ratio; }

    connections = $state<Connection[]>([]);
    loadingConnections = $state(false);
    private unlistenFunctions: (() => void)[] = [];
    commands = new Map<string, CommandConfig>();
    keybindings = new Map<string, string>();

    constructor() {
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
        }
    }

    formatKeybinding(commandId: string): string {
        const isMac = navigator.userAgent.includes("Mac");
        let foundKey: string | undefined;
        for (const [key, id] of this.keybindings.entries()) {
            if (id === commandId) {
                foundKey = key;
                break;
            }
        }
        if (!foundKey) return "None";
        return foundKey.split("+").map(part => {
            part = part.trim();
            if (part === "meta") return isMac ? "⌘" : "Ctrl";
            if (part === "control") return "Ctrl";
            if (part === "alt") return isMac ? "⌥" : "Alt";
            if (part === "shift") return isMac ? "⇧" : "Shift";
            return part.toUpperCase();
        }).join(isMac ? " " : "+");
    }

    handleKeydown(event: KeyboardEvent) {
        const modifiers = [];
        if (event.metaKey) modifiers.push("meta");
        if (event.ctrlKey) modifiers.push("control");
        if (event.altKey) modifiers.push("alt");
        if (event.shiftKey) modifiers.push("shift");
        const key = event.key.toLowerCase();
        if (["meta", "control", "alt", "shift"].includes(key)) return;
        const combo = [...modifiers, key].join("+").toLowerCase();

        console.log(`[ShortcutCheck] combo: ${combo}, activeElement:`, document.activeElement);

        const commandId = this.keybindings.get(combo);
        if (commandId) {
            console.log(`[ShortcutMatch] Found command: ${commandId}`);
            event.preventDefault();
            this.executeCommand(commandId);
        }
    }

    async init() {
        try {
            console.log(`[WindowStateStore] Initializing for window: ${this.label}`);
            const windows = await getAllWindows();
            this.settingsWindowOpen = windows.some(w => w.label === "appearance-window");
            this.datasourceWindowOpen = windows.some(w => w.label === "datasource-window");

            const unlistenCreated = await listen("window-created", (event) => {
                const label = event.payload as string;
                if (label === "appearance-window") this.settingsWindowOpen = true;
                if (label === "datasource-window") this.datasourceWindowOpen = true;
            });
            this.unlistenFunctions.push(unlistenCreated);

            const unlistenDestroyed = await listen("window-destroyed", (event) => {
                const label = event.payload as string;
                if (label === "appearance-window") this.settingsWindowOpen = false;
                if (label === "datasource-window") this.datasourceWindowOpen = false;
            });
            this.unlistenFunctions.push(unlistenDestroyed);

            this.unlistenFunctions.push(settingsStore.init(this.label));
            this.unlistenFunctions.push(themeStore.init());

            await settingsStore.waitForInit();
            this.initialized = true;
            console.log("[WindowStateStore] Initialization complete.");
        } catch (e) {
            console.error("[WindowStateStore] Init failed:", e);
        }
    }

    cleanup() {
        this.unlistenFunctions.forEach(unlisten => unlisten());
        this.unlistenFunctions = [];
    }
}

export const windowState = new WindowStateStore();
