import { browser } from "$app/environment";
import { commandClient } from "$lib/commands/client";

export interface Settings {
    editorFontFamily: string;
    editorFontSize: number;
}

const DEFAULT_SETTINGS: Settings = {
    editorFontFamily: "Fira Code, monospace",
    editorFontSize: 14,
};

function createSettingsStore() {
    let settings = $state<Settings>(DEFAULT_SETTINGS);

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

    return {
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
        reset() {
            settings = DEFAULT_SETTINGS;
            apply();
            if (browser) {
                commandClient.updateAppSetting("editor_font_family", DEFAULT_SETTINGS.editorFontFamily);
                commandClient.updateAppSetting("editor_font_size", DEFAULT_SETTINGS.editorFontSize.toString());
            }
        },
        init() {
            if (!browser) return () => { };

            // Fetch initial settings from backend
            commandClient.getAppSettings().then((res) => {
                console.log("[Settings] Backend response:", res);
                if (res.success && res.data) {
                    console.log("[Settings] Applying backend settings:", res.data);
                    settings = { ...DEFAULT_SETTINGS, ...res.data };
                    apply();
                } else {
                    console.error("[Settings] Failed to load settings:", res.error);
                }
            }).catch(err => {
                console.error("[Settings] Error fetching settings:", err);
            });

            // Initial application of default/current state
            apply();

            // Listen for settings changes from other windows
            let unlisten: () => void;
            import("@tauri-apps/api/event").then(async ({ listen }) => {
                unlisten = await listen<[string, string]>("settings-changed", (event) => {
                    console.log("[Settings] Remote change event:", event);
                    const [key, value] = event.payload;
                    if (key === "editor_font_family") {
                        settings.editorFontFamily = value;
                        apply();
                    } else if (key === "editor_font_size") {
                        settings.editorFontSize = parseInt(value);
                    }
                });
            });

            return () => {
                if (unlisten) unlisten();
            };
        }
    };
}

export const settingsStore = createSettingsStore();
