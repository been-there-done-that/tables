import type * as monaco from 'monaco-editor';
import { getWindowEditorPool, getModelRegistry } from './editor-pool';

// Interval in ms
const SAVE_INTERVAL = 10_000;

interface PersistedSqlModel {
    uri: string;
    value: string;
    viewState?: monaco.editor.ICodeEditorViewState;
    updatedAt: number;
}

export function startRecoveryService() {
    // Ensure we don't start multiple times?
    // Using a simple flag or reliance on single call site.
    if ((window as any).__monacoRecoveryStarted) return;
    (window as any).__monacoRecoveryStarted = true;

    setInterval(() => {
        try {
            const registry = getModelRegistry();
            if (registry) {
                const models = registry.getPersistentModels();
                persist(models);
            }
        } catch (e) {
            // Pool not ready, skip
        }
    }, SAVE_INTERVAL);
}

export function persist(models: [string, any][]) { // Typed as any for now to avoid registry dependancy details
    const data: PersistedSqlModel[] = models.map(([uri, entry]) => ({
        uri,
        value: entry.model.getValue(),
        updatedAt: Date.now()
    }));

    // Write to local storage or file
    // localStorage.setItem('monaco-recovery', JSON.stringify(data));
}
