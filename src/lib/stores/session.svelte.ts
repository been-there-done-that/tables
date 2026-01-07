import type { Connection, MetaSchema } from "$lib/commands/types";
import { settingsStore } from "./settings.svelte";

export type ViewType = "editor" | "table" | "custom";

export interface ViewState {
    id: string;
    type: ViewType;
    title: string;
    data?: any; // To hold editor content, table state, etc.
}

export interface ExplorerState {
    expanded: Set<string>;
}

export class Session {
    id = $state("");
    connectionId = $state("");
    windowLabel = $state("");
    // We might want to store the full connection object for easy access
    connection = $state<Connection | null>(null);

    // Explorer State
    explorerState = $state<ExplorerState>({ expanded: new Set() });

    // Schema Cache (per session)
    schemas = $state<MetaSchema[]>([]);

    views = $state<ViewState[]>([]);
    activeViewId = $state<string | null>(null);

    private cleanup: (() => void) | null = null;

    constructor(id: string, connection: Connection, windowLabel: string = "main") {
        this.id = id;
        this.connectionId = connection.id;
        this.connection = connection;
        this.windowLabel = windowLabel;

        // Load persisted expansion state
        const persistedExpanded = settingsStore.getExpandedNodes(connection.id);
        if (persistedExpanded.length > 0) {
            this.explorerState.expanded = new Set(persistedExpanded);
        }
    }

    persistExpandedNodes() {
        const nodes = Array.from(this.explorerState.expanded);
        settingsStore.setExpandedNodes(this.connectionId, nodes);
    }

    addView(view: ViewState) {
        this.views.push(view);
        this.activeViewId = view.id;
    }

    openView(type: ViewType, title: string, data?: any) {
        // Find existing view if it matches type/title/data to avoid duplicates?
        // For now, just create a new one
        const id = crypto.randomUUID();
        const newView: ViewState = { id, type, title, data };
        this.addView(newView);
        return id;
    }

    closeView(viewId: string) {
        const index = this.views.findIndex(v => v.id === viewId);
        if (index === -1) return;

        this.views = this.views.filter(v => v.id !== viewId);
        if (this.activeViewId === viewId) {
            this.activeViewId = this.views.length > 0 ? this.views[this.views.length - 1].id : null;
        }
    }

    activateView(viewId: string) {
        if (this.views.find(v => v.id === viewId)) {
            this.activeViewId = viewId;
        }
    }
}
