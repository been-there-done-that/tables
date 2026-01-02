import type { Connection, MetaSchema } from "$lib/commands/types";

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
    // We might want to store the full connection object for easy access
    connection = $state<Connection | null>(null);

    // Explorer State
    explorerState = $state<ExplorerState>({ expanded: new Set() });

    // Schema Cache (per session)
    schemas = $state<MetaSchema[]>([]);

    views = $state<ViewState[]>([]);
    activeViewId = $state<string | null>(null);

    constructor(id: string, connection: Connection) {
        this.id = id;
        this.connectionId = connection.id;
        this.connection = connection;
    }

    addView(view: ViewState) {
        this.views.push(view);
        this.activeViewId = view.id;
    }

    closeView(viewId: string) {
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
