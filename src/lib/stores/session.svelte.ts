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
    private onStateChange?: () => void;

    constructor(id: string, connection: Connection, windowLabel: string = "main", onStateChange?: () => void) {
        this.id = id;
        this.connectionId = connection.id;
        this.connection = connection;
        this.windowLabel = windowLabel;
        this.onStateChange = onStateChange;

        // Load persisted expansion state
        const persistedExpanded = settingsStore.getExpandedNodes(connection.id);
        if (persistedExpanded.length > 0) {
            this.explorerState.expanded = new Set(persistedExpanded);
        }
    }

    private triggerSave() {
        if (this.onStateChange) this.onStateChange();
    }

    persistExpandedNodes() {
        const nodes = Array.from(this.explorerState.expanded);
        settingsStore.setExpandedNodes(this.connectionId, nodes);
        this.triggerSave();
    }

    addView(view: ViewState) {
        this.views.push(view);
        this.activeViewId = view.id;
        this.triggerSave();
    }

    openView(type: ViewType, title: string, data?: any) {
        // For table views, check if one already exists for the same table
        if (type === "table" && data?.tableName) {
            const existing = this.views.find(
                v => v.type === "table" &&
                    v.data?.tableName === data.tableName &&
                    v.data?.schemaName === data.schemaName &&
                    v.data?.databaseName === data.databaseName
            );
            if (existing) {
                this.activeViewId = existing.id;
                this.triggerSave();
                return existing.id;
            }
        }

        const id = crypto.randomUUID();
        if (type === "editor") {
            data = data || {};
            data.results = {
                rows: [],
                columns: [],
                total: 0,
                visible: false,
                loading: false,
                pageSize: 100,
                offset: 0,
                whereClause: "",
                orderByClause: "",
                executionTime: 0,
                executedQueryText: "",
                detectedTable: null,
                pendingDeltas: [],
                isSaving: false,
                currentBatchSize: 0,
                isExactTotal: true,
                isCountLoading: false,
            };
            data.controller = {};
        }
        const newView: ViewState = { id, type, title, data };
        this.addView(newView);
        // addView calls triggerSave
        return id;
    }

    closeView(viewId: string) {
        const index = this.views.findIndex(v => v.id === viewId);
        if (index === -1) return;

        this.views = this.views.filter(v => v.id !== viewId);
        if (this.activeViewId === viewId) {
            this.activeViewId = this.views.length > 0 ? this.views[this.views.length - 1].id : null;
        }
        this.triggerSave();
    }

    closeOtherViews(viewId: string) {
        this.views = this.views.filter(v => v.id === viewId);
        this.activeViewId = viewId;
        this.triggerSave();
    }

    closeViewsToLeft(viewId: string) {
        const index = this.views.findIndex(v => v.id === viewId);
        if (index === -1) return;

        this.views = this.views.slice(index);
        if (this.activeViewId && !this.views.find(v => v.id === this.activeViewId)) {
            this.activeViewId = viewId;
        }
        this.triggerSave();
    }

    closeViewsToRight(viewId: string) {
        const index = this.views.findIndex(v => v.id === viewId);
        if (index === -1) return;

        this.views = this.views.slice(0, index + 1);
        if (this.activeViewId && !this.views.find(v => v.id === this.activeViewId)) {
            this.activeViewId = viewId;
        }
        this.triggerSave();
    }

    closeAllViews() {
        this.views = [];
        this.activeViewId = null;
        this.triggerSave();
    }

    activateView(viewId: string) {
        if (this.views.find(v => v.id === viewId)) {
            this.activeViewId = viewId;
            this.triggerSave();
        }
    }
}
