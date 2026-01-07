import { browser } from "$app/environment";
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
        // Font settings
        get editorFontFamily() {
            return settings.editorFontFamily;
        },
        set editorFontFamily(v: string) {
            settings.editorFontFamily = v;
            if (browser) {
                apply();
                commandClient.updateAppSetting("editor_font_family", v);
            }
        },
        get editorFontSize() {
            return settings.editorFontSize;
        },
        set editorFontSize(v: number) {
            settings.editorFontSize = v;
            if (browser) {
                commandClient.updateAppSetting("editor_font_size", v.toString());
            }
        },

        // Layout settings - left sidebar
        get sidebarLeftVisible() {
            return settings.sidebarLeftVisible;
        },
        set sidebarLeftVisible(v: boolean) {
            settings.sidebarLeftVisible = v;
            if (browser) {
                commandClient.updateAppSetting(`window:${windowLabel}:sidebar_left_visible`, v.toString());
            }
        },
        get sidebarLeftRatio() {
            return settings.sidebarLeftRatio;
        },
        set sidebarLeftRatio(v: number) {
            settings.sidebarLeftRatio = v;
            if (browser) {
                persistLeftRatio(v);
            }
        },

        // Layout settings - right sidebar
        get sidebarRightVisible() {
            return settings.sidebarRightVisible;
        },
        set sidebarRightVisible(v: boolean) {
            settings.sidebarRightVisible = v;
            if (browser) {
                commandClient.updateAppSetting(`window:${windowLabel}:sidebar_right_visible`, v.toString());
            }
        },
        get sidebarRightRatio() {
            return settings.sidebarRightRatio;
        },
        set sidebarRightRatio(v: number) {
            settings.sidebarRightRatio = v;
            if (browser) {
                persistRightRatio(v);
            }
        },

        // Layout settings - bottom panel
        get sidebarBottomVisible() {
            return settings.sidebarBottomVisible;
        },
        set sidebarBottomVisible(v: boolean) {
            settings.sidebarBottomVisible = v;
            if (browser) {
                commandClient.updateAppSetting(`window:${windowLabel}:sidebar_bottom_visible`, v.toString());
            }
        },
        get sidebarBottomRatio() {
            return settings.sidebarBottomRatio;
        },
        set sidebarBottomRatio(v: number) {
            settings.sidebarBottomRatio = v;
            if (browser) {
                persistBottomRatio(v);
            }
        },

        // Selected database
        get selectedDatabase() {
            return settings.selectedDatabase;
        },
        set selectedDatabase(v: string | null) {
            settings.selectedDatabase = v;
            if (browser) {
                commandClient.updateAppSetting(`window:${windowLabel}:selected_database`, v || "");
            }
        },

        // Utility
        get initialized() {
            return initialized;
        },

        reset() {
            settings = { ...DEFAULT_SETTINGS };
            apply();
            if (browser) {
                commandClient.updateAppSetting("editor_font_family", DEFAULT_SETTINGS.editorFontFamily);
                commandClient.updateAppSetting("editor_font_size", DEFAULT_SETTINGS.editorFontSize.toString());
            }
        },

        /** Wait for settings to be loaded from backend */
        async waitForInit(): Promise<void> {
            if (initialized) return;
            if (initPromise) return initPromise;
            // If not initialized and no promise, just wait a bit
            return new Promise((resolve) => setTimeout(resolve, 100));
        },

        init(label: string = "main") {
            if (!browser) return () => { };
            windowLabel = label;

            // Fetch initial settings from backend
            initPromise = commandClient.getAppSettings().then((res) => {
                console.log("[Settings] Backend response:", res);
                if (res.success && res.data) {
                    console.log("[Settings] Applying backend settings:", res.data);

                    const { windowLayouts, ...globalSettings } = res.data as any;

                    // Merge global settings first
                    settings = { ...DEFAULT_SETTINGS, ...globalSettings };

                    // Then merge window-specific layout if available
                    if (windowLayouts && windowLayouts[windowLabel]) {
                        console.log(`[Settings] Applying layout for window: ${windowLabel}`, windowLayouts[windowLabel]);
                        settings = { ...settings, ...windowLayouts[windowLabel] };
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
                    case "selected_database":
                        settings.selectedDatabase = value || null;
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
