import { get, set, del } from "idb-keyval";
import type { Session, ViewState } from "./session.svelte";

const BASE_STORAGE_KEY = "tables_session_state_v2";
function getPersistenceKey(label: string, connectionId?: string) {
    if (connectionId) {
        return `${BASE_STORAGE_KEY}_${label}_conn_${connectionId}`;
    }
    return `${BASE_STORAGE_KEY}_${label}`;
}

export interface PersistedSession {
    id: string;
    connectionId: string;
    windowLabel: string;
    views: ViewState[];
    activeViewId: string | null;
    explorerState: {
        expanded: string[];
    };
}

export interface PersistedState {
    sessions: PersistedSession[];
    lastActiveSessionId: string | null;
}

function debugLog(action: string, details?: any) {
    console.log(`[PersistenceStore] ${action}`, details || "");
}

function safeSerialize(obj: any): any {
    if (obj === undefined) return undefined;
    try {
        return JSON.parse(JSON.stringify(obj));
    } catch (e) {
        console.warn("[PersistenceStore] Failed to serialize object:", e);
        return null;
    }
}

export const persistenceStore = {
    async saveSessionState(sessions: Session[], activeSessionId: string | null, windowLabel: string, connectionId?: string) {
        const key = getPersistenceKey(windowLabel, connectionId);

        if (!sessions || sessions.length === 0) {
            debugLog("Clearing state: No sessions active", { key });
            try {
                await del(key);
                debugLog("State cleared successfully");
            } catch (err) {
                console.error("[PersistenceStore] Failed to clear state:", err);
            }
            return;
        }

        const start = performance.now();
        const state: PersistedState = {
            lastActiveSessionId: activeSessionId,
            sessions: sessions.map(s => ({
                id: s.id,
                connectionId: s.connectionId,
                windowLabel: s.windowLabel,
                activeViewId: s.activeViewId,
                explorerState: {
                    expanded: Array.from(s.explorerState.expanded)
                },
                views: s.views.map(v => ({
                    id: v.id,
                    type: v.type,
                    title: v.title,
                    data: safeSerialize(v.data)
                }))
            }))
        };

        try {
            const safeState = JSON.parse(JSON.stringify(state));

            await set(getPersistenceKey(windowLabel, connectionId), safeState);
            const elapsed = Math.round(performance.now() - start);
            debugLog(`State saved successfully in ${elapsed}ms`, {
                sessionCount: state.sessions.length,
                totalViews: state.sessions.reduce((acc, s) => acc + s.views.length, 0),
                key: getPersistenceKey(windowLabel, connectionId)
            });
        } catch (err) {
            console.error("[PersistenceStore] Failed to save state:", err);
        }
    },

    async loadSessionState(windowLabel: string, connectionId?: string): Promise<PersistedState | null> {
        debugLog(`Loading session state for ${windowLabel} (conn: ${connectionId})...`);
        const start = performance.now();
        try {
            const state = await get<PersistedState>(getPersistenceKey(windowLabel, connectionId));
            const elapsed = Math.round(performance.now() - start);

            if (state) {
                debugLog(`State loaded successfully in ${elapsed}ms`, {
                    sessionCount: state.sessions?.length || 0,
                    activeSessionId: state.lastActiveSessionId
                });
                return state;
            } else {
                debugLog("No persisted state found");
                return null;
            }
        } catch (err) {
            console.error("[PersistenceStore] Failed to load state:", err);
            return null;
        }
    }
};
