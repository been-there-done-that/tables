import type * as monaco from 'monaco-editor';
import type { EditorKind } from './editor-types';

interface ModelEntry {
    model: monaco.editor.ITextModel;
    refCount: number;
    kind: EditorKind;
    dirty: boolean;
    lastAccessed: number;
}

export class ModelRegistry {
    private models = new Map<string, ModelEntry>();

    constructor(private monacoInstance: typeof monaco) { }

    getOrCreate(
        uri: string,
        language: string,
        kind: EditorKind,
        initialValue = ""
    ): monaco.editor.ITextModel {
        const existing = this.models.get(uri);
        if (existing) {
            existing.refCount++;
            existing.lastAccessed = Date.now();
            return existing.model;
        }

        const model = this.monacoInstance.editor.createModel(
            initialValue,
            language,
            this.monacoInstance.Uri.parse(uri)
        );

        const entry: ModelEntry = {
            model,
            refCount: 1,
            kind,
            dirty: false,
            lastAccessed: Date.now()
        };

        model.onDidChangeContent(() => {
            entry.dirty = true;
        });

        this.models.set(uri, entry);
        return model;
    }

    release(uri: string) {
        const entry = this.models.get(uri);
        if (!entry) return;
        entry.refCount--;

        if (entry.refCount <= 0 && entry.kind === "json") {
            entry.model.dispose();
            this.models.delete(uri);
        }
    }

    getPersistentModels() {
        return [...this.models.entries()]
            .filter(([, e]) => e.kind === "sql");
    }

    getModelEntry(uri: string): ModelEntry | undefined {
        return this.models.get(uri);
    }
}
