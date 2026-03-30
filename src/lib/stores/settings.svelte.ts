import { commandClient } from "$lib/commands/client";
import { debounce } from "$lib/utils";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export interface CachedModel {
    id: string;
    contextLength?: number;
    pricingIn?: number;
    pricingOut?: number;
}

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
    // AI settings
    aiModel: string;
    aiEffort: "auto" | "low" | "medium" | "high" | "max";
    queryApproval: "auto" | "ask";
    aiProvider: string;
    googleApiKey: string;
    openrouterApiKey: string;
    googleBaseUrl: string;
    openrouterBaseUrl: string;
    googlePinnedModels: string[];
    openrouterPinnedModels: string[];
    googleCachedModels: CachedModel[];
    openrouterCachedModels: CachedModel[];
    lastActiveThreadId: string | null;
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
    aiModel: "claude-sonnet-4-6",
    aiEffort: "auto",
    queryApproval: "ask",
    aiProvider: "claude",
    googleApiKey: "",
    openrouterApiKey: "",
    googleBaseUrl: "",
    openrouterBaseUrl: "",
    googlePinnedModels: [],
    openrouterPinnedModels: [],
    googleCachedModels: [],
    openrouterCachedModels: [],
    lastActiveThreadId: null,
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
        // AI settings
        get aiModel() {
            return settings.aiModel;
        },
        set aiModel(v: string) {
            settings.aiModel = v;
            commandClient.updateAppSetting("ai_model", v);
        },
        get aiEffort(): "auto" | "low" | "medium" | "high" | "max" {
            return settings.aiEffort;
        },
        set aiEffort(v: "auto" | "low" | "medium" | "high" | "max") {
            settings.aiEffort = v;
            commandClient.updateAppSetting("ai_effort", v);
        },
        get queryApproval(): "auto" | "ask" {
            return settings.queryApproval;
        },
        set queryApproval(v: "auto" | "ask") {
            settings.queryApproval = v;
            commandClient.updateAppSetting("query_approval", v);
        },
        get aiProvider(): string {
            return settings.aiProvider;
        },
        set aiProvider(v: string) {
            settings.aiProvider = v;
            commandClient.updateAppSetting("ai_provider", v);
        },
        get googleApiKey(): string {
            return settings.googleApiKey;
        },
        set googleApiKey(v: string) {
            settings.googleApiKey = v;
            commandClient.updateAppSetting("google_api_key", v);
        },
        get openrouterApiKey(): string {
            return settings.openrouterApiKey;
        },
        set openrouterApiKey(v: string) {
            settings.openrouterApiKey = v;
            commandClient.updateAppSetting("openrouter_api_key", v);
        },
        get googleBaseUrl(): string {
            return settings.googleBaseUrl;
        },
        set googleBaseUrl(v: string) {
            settings.googleBaseUrl = v;
            commandClient.updateAppSetting("google_base_url", v);
        },
        get openrouterBaseUrl(): string {
            return settings.openrouterBaseUrl;
        },
        set openrouterBaseUrl(v: string) {
            settings.openrouterBaseUrl = v;
            commandClient.updateAppSetting("openrouter_base_url", v);
        },
        get googlePinnedModels(): string[] {
            return settings.googlePinnedModels;
        },
        set googlePinnedModels(v: string[]) {
            settings.googlePinnedModels = v;
            commandClient.updateAppSetting("google_pinned_models", JSON.stringify(v));
        },
        get openrouterPinnedModels(): string[] {
            return settings.openrouterPinnedModels;
        },
        set openrouterPinnedModels(v: string[]) {
            settings.openrouterPinnedModels = v;
            commandClient.updateAppSetting("openrouter_pinned_models", JSON.stringify(v));
        },
        get googleCachedModels(): CachedModel[] {
            return settings.googleCachedModels;
        },
        set googleCachedModels(v: CachedModel[]) {
            settings.googleCachedModels = v;
            commandClient.updateAppSetting("google_cached_models", JSON.stringify(v));
        },
        get openrouterCachedModels(): CachedModel[] {
            return settings.openrouterCachedModels;
        },
        set openrouterCachedModels(v: CachedModel[]) {
            settings.openrouterCachedModels = v;
            commandClient.updateAppSetting("openrouter_cached_models", JSON.stringify(v));
        },
        get lastActiveThreadId(): string | null {
            return settings.lastActiveThreadId;
        },
        set lastActiveThreadId(v: string | null) {
            settings.lastActiveThreadId = v;
            commandClient.updateAppSetting("last_active_thread_id", v ?? "");
        },

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

                        // Unpack the nested aiSettings map (Rust HashMap<String,String> → { google_api_key: ... })
                        if (key === "aiSettings" && value && typeof value === "object") {
                            for (const [aiKey, aiVal] of Object.entries(value as Record<string, string>)) {
                                switch (aiKey) {
                                    case "google_api_key":           parsedSettings.googleApiKey = aiVal; break;
                                    case "openrouter_api_key":       parsedSettings.openrouterApiKey = aiVal; break;
                                    case "google_base_url":          parsedSettings.googleBaseUrl = aiVal; break;
                                    case "openrouter_base_url":      parsedSettings.openrouterBaseUrl = aiVal; break;
                                    case "google_pinned_models":
                                        try { parsedSettings.googlePinnedModels = JSON.parse(aiVal); }
                                        catch { parsedSettings.googlePinnedModels = []; }
                                        break;
                                    case "openrouter_pinned_models":
                                        try { parsedSettings.openrouterPinnedModels = JSON.parse(aiVal); }
                                        catch { parsedSettings.openrouterPinnedModels = []; }
                                        break;
                                    case "google_cached_models":
                                        try { parsedSettings.googleCachedModels = JSON.parse(aiVal); }
                                        catch { parsedSettings.googleCachedModels = []; }
                                        break;
                                    case "openrouter_cached_models":
                                        try { parsedSettings.openrouterCachedModels = JSON.parse(aiVal); }
                                        catch { parsedSettings.openrouterCachedModels = []; }
                                        break;
                                    case "ai_provider":              parsedSettings.aiProvider = aiVal || "claude"; break;
                                    case "ai_model":                 parsedSettings.aiModel = aiVal; break;
                                    case "ai_effort":                parsedSettings.aiEffort = aiVal as any; break;
                                    case "query_approval":           parsedSettings.queryApproval = aiVal as "auto" | "ask"; break;
                                    case "last_active_thread_id":    parsedSettings.lastActiveThreadId = aiVal || null; break;
                                }
                            }
                            continue;
                        }

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

                        // Handle JSON arrays (pinned models)
                        if (key === "googlePinnedModels" || key === "openrouterPinnedModels") {
                            try { parsedSettings[key] = JSON.parse(value as string); } catch { parsedSettings[key] = []; }
                            continue;
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
                    case "query_approval":
                        settings.queryApproval = (value === "auto" ? "auto" : "ask") as "auto" | "ask";
                        return;
                    case "ai_provider":
                        settings.aiProvider = value || "claude";
                        return;
                    case "google_api_key":
                        settings.googleApiKey = value || "";
                        return;
                    case "openrouter_api_key":
                        settings.openrouterApiKey = value || "";
                        return;
                    case "google_base_url":
                        settings.googleBaseUrl = value || "";
                        return;
                    case "openrouter_base_url":
                        settings.openrouterBaseUrl = value || "";
                        return;
                    case "google_pinned_models":
                        try { settings.googlePinnedModels = JSON.parse(value || "[]"); } catch { settings.googlePinnedModels = []; }
                        return;
                    case "openrouter_pinned_models":
                        try { settings.openrouterPinnedModels = JSON.parse(value || "[]"); } catch { settings.openrouterPinnedModels = []; }
                        return;
                    case "last_active_thread_id":
                        settings.lastActiveThreadId = value || null;
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
