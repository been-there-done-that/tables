import { invoke } from "@tauri-apps/api/core";

export interface EditorSession {
    id: string;
    windowLabel: string;
    connectionId: string | null;
    schemaName: string | null;
    content: string;
    cursorLine: number;
    cursorColumn: number;
    createdAt: number;
    lastOpenedAt: number;
}

export interface EditorSessionSummary {
    id: string;
    windowLabel: string;
    connectionId: string | null;
    schemaName: string | null;
    createdAt: number;
    lastOpenedAt: number;
}

/**
 * Save an editor session to the backend SQLite database.
 * Uses debouncing at the caller level to avoid excessive writes.
 */
export async function saveEditorSession(
    id: string,
    windowLabel: string,
    content: string,
    cursorLine: number,
    cursorColumn: number,
    connectionId?: string | null,
    schemaName?: string | null
): Promise<void> {
    await invoke("save_editor_session", {
        id,
        windowLabel,
        connectionId: connectionId ?? null,
        schemaName: schemaName ?? null,
        content,
        cursorLine,
        cursorColumn,
    });
}

/**
 * Load an editor session from the backend by its ID.
 */
export async function loadEditorSession(id: string): Promise<EditorSession | null> {
    return await invoke<EditorSession | null>("load_editor_session", { id });
}

/**
 * List editor sessions, optionally filtered by window label and/or connection ID.
 */
export async function listEditorSessions(
    windowLabel?: string,
    connectionId?: string
): Promise<EditorSessionSummary[]> {
    return await invoke<EditorSessionSummary[]>("list_editor_sessions", {
        windowLabel: windowLabel ?? null,
        connectionId: connectionId ?? null,
    });
}

/**
 * Delete an editor session by ID.
 */
export async function deleteEditorSession(id: string): Promise<void> {
    await invoke("delete_editor_session", { id });
}

/**
 * Debounce utility for saving editor content.
 * Returns a debounced version of the save function.
 */
export function createDebouncedSave(delayMs: number = 2000) {
    let timeoutId: ReturnType<typeof setTimeout> | null = null;
    let pendingSave: (() => Promise<void>) | null = null;

    return {
        save: (saveFn: () => Promise<void>) => {
            pendingSave = saveFn;
            if (timeoutId) {
                clearTimeout(timeoutId);
            }
            timeoutId = setTimeout(async () => {
                if (pendingSave) {
                    try {
                        await pendingSave();
                    } catch (e) {
                        console.error("[EditorPersistence] Debounced save failed:", e);
                    }
                    pendingSave = null;
                }
                timeoutId = null;
            }, delayMs);
        },
        flush: async () => {
            if (timeoutId) {
                clearTimeout(timeoutId);
                timeoutId = null;
            }
            if (pendingSave) {
                try {
                    await pendingSave();
                } catch (e) {
                    console.error("[EditorPersistence] Flush save failed:", e);
                }
                pendingSave = null;
            }
        },
        cancel: () => {
            if (timeoutId) {
                clearTimeout(timeoutId);
                timeoutId = null;
            }
            pendingSave = null;
        }
    };
}
