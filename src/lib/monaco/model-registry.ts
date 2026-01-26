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
        initialValue = "",
        forceCleanContent = false
    ): monaco.editor.ITextModel {
        console.log("[MODEL-DEBUG] getOrCreate called:", { uri, language, kind, forceCleanContent, initialValueLen: initialValue.length });

        const existing = this.models.get(uri);
        if (existing) {
            console.log("[MODEL-DEBUG] Found existing model in local registry:", {
                uri,
                refCount: existing.refCount,
                contentBefore: existing.model.getValue().substring(0, 100)
            });
            existing.refCount++;
            existing.lastAccessed = Date.now();
            if (forceCleanContent) {
                existing.model.setValue(initialValue);
            }
            return existing.model;
        }

        // Check if monaco already knows about this model even if we don't
        // This can happen during hot reloads or pooling edge cases
        const parsedUri = this.monacoInstance.Uri.parse(uri);
        const existingMonacoModel = this.monacoInstance.editor.getModel(parsedUri);
        if (existingMonacoModel) {
            console.log("[MODEL-DEBUG] Model exists in Monaco, adopting into registry:", uri);
            const entry: ModelEntry = {
                model: existingMonacoModel,
                refCount: 1,
                kind,
                dirty: false,
                lastAccessed: Date.now()
            };
            this.models.set(uri, entry);
            if (forceCleanContent) {
                existingMonacoModel.setValue(initialValue);
            }
            return existingMonacoModel;
        }

        console.log("[MODEL-DEBUG] Creating NEW model for uri:", uri);
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
        console.log("[MODEL-DEBUG] New model created and stored:", { uri, totalModels: this.models.size });
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
