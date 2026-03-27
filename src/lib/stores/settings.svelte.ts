import { commandClient } from "$lib/commands/client";
import { debounce } from "$lib/utils";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export interface Settings {
    editorFontFamily: string;
    editorFontSize: number;
    // Layout settings
    sidebarLeftVisible: boolean;
    sidebarLeftRatio: number;
    sidebarRightVisible: boolean;
    sidebarRightRatio: number;
    sidebarBottomVisible: boolean;
    sidebarBottomRatio: number;
    // Selected database
    selectedDatabase: string | null;
    expandedNodes: Record<string, string[]>;
    // Active panel in the right sidebar
    activeRightPanel: string | null;
    // Editor settings
    editorShowAllRunButtons: boolean;
}

const DEFAULT_SETTINGS: Settings = {
    editorFontFamily: "Fira Code, monospace",
    editorFontSize: 14,
    // Layout defaults
    sidebarLeftVisible: true,
    sidebarLeftRatio: 0.2,
    sidebarRightVisible: false,
    sidebarRightRatio: 0.75,
    sidebarBottomVisible: false,
    sidebarBottomRatio: 0.7,
    // No database selected by default
    selectedDatabase: null,
    expandedNodes: {},
    activeRightPanel: null,
    editorShowAllRunButtons: false,
};

function createSettingsStore() {
    let settings = $state<Settings>(DEFAULT_SETTINGS);
    let windowLabel = $state("main");
    let initialized = $state(false);
    let initPromise: Promise<void> | null = null;

    const apply = () => {
        const family = settings.editorFontFamily;
        // Ensure strictly quoted string for CSS variable to handle spaces safely
        // e.g. "Fira Code" -> '"Fira Code"' so var(--font) resolves to "Fira Code"
        const safeFamily = family.includes(" ") ? `"${family}"` : family;

        // Update root and body to ensure visibility everywhere
        document.documentElement.style.setProperty("--font-user-mono", safeFamily);
        document.body.style.setProperty("--font-user-mono", safeFamily);

        // Also update main UI font as requested
        document.documentElement.style.setProperty("--font-user-ui", safeFamily);
        document.body.style.setProperty("--font-user-ui", safeFamily);
    };

    // Debounced persistence for ratio changes (avoid excessive API calls during drag)
    const persistLeftRatio = debounce((v: number) => {
        commandClient.updateAppSetting(`window:${windowLabel}:sidebar_left_ratio`, v.toString());
    }, 300);
    const persistRightRatio = debounce((v: number) => {
        commandClient.updateAppSetting(`window:${windowLabel}:sidebar_right_ratio`, v.toString());
    }, 300);
    const persistBottomRatio = debounce((v: number) => {
        commandClient.updateAppSetting(`window:${windowLabel}:sidebar_bottom_ratio`, v.toString());
    }, 300);

    return {
        // Run button visibility
        get editorShowAllRunButtons() {
            return settings.editorShowAllRunButtons;
        },
        set editorShowAllRunButtons(v: boolean) {
            settings.editorShowAllRunButtons = v;
            commandClient.updateAppSetting("editor_show_all_run_buttons", v.toString());
        },

        // Font settings
        get editorFontFamily() {
            return settings.editorFontFamily;
        },
        set editorFontFamily(v: string) {
            settings.editorFontFamily = v;
            apply();
            commandClient.updateAppSetting("editor_font_family", v);
        },
        get editorFontSize() {
            return settings.editorFontSize;
        },
        set editorFontSize(v: number) {
            settings.editorFontSize = v;
            commandClient.updateAppSetting("editor_font_size", v.toString());
        },

        // Layout settings - left sidebar
        get sidebarLeftVisible() {
            return settings.sidebarLeftVisible;
        },
        set sidebarLeftVisible(v: boolean) {
            settings.sidebarLeftVisible = v;
            commandClient.updateAppSetting(`window:${windowLabel}:sidebar_left_visible`, v.toString());
        },
        get sidebarLeftRatio() {
            return settings.sidebarLeftRatio;
        },
        set sidebarLeftRatio(v: number) {
            settings.sidebarLeftRatio = v;
            persistLeftRatio(v);
        },

        // Layout settings - right sidebar
        get sidebarRightVisible() {
            return settings.sidebarRightVisible;
        },
        set sidebarRightVisible(v: boolean) {
            settings.sidebarRightVisible = v;
            // DON'T persist "visible" if we are opening to pending changes
            if (v && settings.activeRightPanel === "pending-changes") {
                return;
            }
            commandClient.updateAppSetting(`window:${windowLabel}:sidebar_right_visible`, v.toString());
        },
        get sidebarRightRatio() {
            return settings.sidebarRightRatio;
        },
        set sidebarRightRatio(v: number) {
            settings.sidebarRightRatio = v;
            persistRightRatio(v);
        },

        // Layout settings - bottom panel
        get sidebarBottomVisible() {
            return settings.sidebarBottomVisible;
        },
        set sidebarBottomVisible(v: boolean) {
            settings.sidebarBottomVisible = v;
            commandClient.updateAppSetting(`window:${windowLabel}:sidebar_bottom_visible`, v.toString());
        },
        get sidebarBottomRatio() {
            return settings.sidebarBottomRatio;
        },
        set sidebarBottomRatio(v: number) {
            settings.sidebarBottomRatio = v;
            persistBottomRatio(v);
        },

        // Active right panel settings
        get activeRightPanel() {
            return settings.activeRightPanel;
        },
        set activeRightPanel(v: string | null) {
            settings.activeRightPanel = v;
            // DON'T persist "pending-changes" as an active panel state
            if (v === "pending-changes") {
                return;
            }
            commandClient.updateAppSetting(`window:${windowLabel}:active_right_panel`, v || "");
        },

        // Selected database
        get selectedDatabase() {
            return settings.selectedDatabase;
        },
        set selectedDatabase(v: string | null) {
            settings.selectedDatabase = v;
            commandClient.updateAppSetting(`window:${windowLabel}:selected_database`, v || "");
        },

        getExpandedNodes(connId: string): string[] {
            return settings.expandedNodes[connId] || [];
        },

        setExpandedNodes(connId: string, nodes: string[]) {
            // Ensure we don't trigger unnecessary updates if nodes haven't changed
            const current = settings.expandedNodes[connId] || [];
            if (current.length === nodes.length && current.every((n, i) => n === nodes[i])) {
                return;
            }

            settings.expandedNodes[connId] = [...nodes];
            commandClient.updateAppSetting(`window:${windowLabel}:conn:${connId}:expanded`, nodes.join(","));
        },

        // Utility
        get initialized() {
            return initialized;
        },

        reset() {
            settings = { ...DEFAULT_SETTINGS };
            apply();
            commandClient.updateAppSetting("editor_font_family", DEFAULT_SETTINGS.editorFontFamily);
            commandClient.updateAppSetting("editor_font_size", DEFAULT_SETTINGS.editorFontSize.toString());
            commandClient.updateAppSetting("editor_show_all_run_buttons", DEFAULT_SETTINGS.editorShowAllRunButtons.toString());
        },

        /** Wait for settings to be loaded from backend */
        async waitForInit(): Promise<void> {
            if (initialized) return;
            if (initPromise) return initPromise;
            // If not initialized and no promise, just wait a bit
            return new Promise((resolve) => setTimeout(resolve, 100));
        },

        init(label: string = "main") {
            windowLabel = label;

            // Fetch initial settings from backend
            initPromise = commandClient.getAppSettings().then((res) => {
                console.log("[Settings] Backend response:", res);
                if (res.success && res.data) {
                    console.log("[Settings] Applying backend settings:", res.data);

                    const { windowLayouts, ...globalSettings } = res.data as any;

                    // Helper to parse value based on key/default type
                    const parsedSettings: any = {};
                    console.log("[Settings] Raw global settings:", globalSettings);

                    for (const [key, value] of Object.entries(globalSettings)) {
                        if (value === null || value === undefined) continue;

                        // Handle Booleans
                        if (
                            key.endsWith("Visible") ||
                            key === "editorShowAllRunButtons" ||
                            value === "true" ||
                            value === "false"
                        ) {
                            parsedSettings[key] = value === "true";
                            continue;
                        }

                        // Handle Numbers
                        if (
                            key.endsWith("Ratio") ||
                            key === "editorFontSize"
                        ) {
                            const num = parseFloat(value as string);
                            if (!isNaN(num)) {
                                parsedSettings[key] = num;
                                continue;
                            }
                        }

                        // Default to string
                        parsedSettings[key] = value;
                    }

                    // Merge global settings
                    settings = { ...DEFAULT_SETTINGS, ...parsedSettings };

                    // Then merge window-specific layout if available
                    if (windowLayouts && windowLayouts[windowLabel]) {
                        console.log(`[Settings] Applying layout for window: ${windowLabel}`, windowLayouts[windowLabel]);
                        const layout = windowLayouts[windowLabel];

                        // Parse layout values too
                        const parsedLayout: any = {};
                        for (const [key, value] of Object.entries(layout)) {
                            if (key === "sidebar_left_visible" || key === "sidebar_right_visible" || key === "sidebar_bottom_visible") {
                                parsedLayout[key] = value === "true";
                            } else if (key.endsWith("Ratio")) {
                                parsedLayout[key] = parseFloat(value as string);
                            } else {
                                parsedLayout[key] = value;
                            }
                        }

                        settings = {
                            ...settings,
                            ...parsedLayout,
                            // Ensure expandedNodes is merged correctly
                            expandedNodes: (layout.expandedNodes as any) || {}
                        };
                    }

                    apply();
                } else {
                    console.error("[Settings] Failed to load settings:", res.error);
                }
                initialized = true;
            }).catch(err => {
                console.error("[Settings] Error fetching settings:", err);
                initialized = true;
            });

            // Initial application of default/current state
            apply();

            // Listen for settings changes from other windows
            let unlisten: UnlistenFn | undefined;
            listen<[string, string]>("settings-changed", (event) => {
                console.log("[Settings] Remote change event:", event);
                const [key, value] = event.payload;

                // Handle global settings
                switch (key) {
                    case "editor_font_family":
                        settings.editorFontFamily = value;
                        apply();
                        return;
                    case "editor_font_size":
                        settings.editorFontSize = parseInt(value);
                        return;
                    case "editor_show_all_run_buttons":
                        settings.editorShowAllRunButtons = value === "true";
                        return;
                }

                // Handle window-specific settings
                if (key.startsWith(`window:${windowLabel}:`)) {
                    const prop = key.replace(`window:${windowLabel}:`, "");
                    switch (prop) {
                        case "sidebar_left_visible":
                            settings.sidebarLeftVisible = value === "true";
                            break;
                        case "sidebar_left_ratio":
                            settings.sidebarLeftRatio = parseFloat(value);
                            break;
                        case "sidebar_right_visible":
                            settings.sidebarRightVisible = value === "true";
                            break;
                        case "sidebar_right_ratio":
                            settings.sidebarRightRatio = parseFloat(value);
                            break;
                        case "sidebar_bottom_visible":
                            settings.sidebarBottomVisible = value === "true";
                            break;
                        case "sidebar_bottom_ratio":
                            settings.sidebarBottomRatio = parseFloat(value);
                            break;
                        case "active_right_panel":
                            settings.activeRightPanel = value || null;
                            break;
                        case "selected_database":
                            settings.selectedDatabase = value || null;
                            break;
                    }

                    // Handle connection-specific expanded nodes: window:{label}:conn:{id}:expanded
                    if (prop.startsWith("conn:") && prop.endsWith(":expanded")) {
                        const connId = prop.split(":")[1];
                        settings.expandedNodes[connId] = value.split(",").filter(s => s !== "");
                    }
                }
            }).then(fn => unlisten = fn);

            return () => {
                if (unlisten) unlisten();
            };
        }
    };
}

export const settingsStore = createSettingsStore();
