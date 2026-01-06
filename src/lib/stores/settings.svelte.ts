import { browser } from "$app/environment";

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

    if (browser) {
        const stored = localStorage.getItem("app_settings");
        if (stored) {
            try {
                settings = { ...DEFAULT_SETTINGS, ...JSON.parse(stored) };
            } catch (e) {
                console.error("Failed to parse settings:", e);
            }
        }
    }

    // Effect to sync with localStorage not strictly needed if we update manually,
    // but useful. However, in .svelte.ts module, $effect is tricky outside component context.
    // We'll update persistent storage in the setters.

    return {
        get editorFontFamily() {
            return settings.editorFontFamily;
        },
        set editorFontFamily(v: string) {
            settings.editorFontFamily = v;
            if (browser) localStorage.setItem("app_settings", JSON.stringify(settings));
        },
        get editorFontSize() {
            return settings.editorFontSize;
        },
        set editorFontSize(v: number) {
            settings.editorFontSize = v;
            if (browser) localStorage.setItem("app_settings", JSON.stringify(settings));
        },
        reset() {
            settings = DEFAULT_SETTINGS;
            if (browser) localStorage.setItem("app_settings", JSON.stringify(settings));
        },
    };
}

export const settingsStore = createSettingsStore();
