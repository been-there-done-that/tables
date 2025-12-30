import type * as monaco from 'monaco-editor';
import { ModelRegistry } from './model-registry';
import type { EditorContext, EditorHandle, EditorViewSnapshot } from './editor-types';

interface PooledEditor {
    editor: monaco.editor.IStandaloneCodeEditor;
    editorId: string;
    active: boolean;
    lastUsed: number;
    snapshot?: EditorViewSnapshot;
}

export class EditorPool {
    private pool: PooledEditor[] = [];
    private MAX = 3;

    constructor(
        private monacoInstance: typeof monaco,
        private modelRegistry: ModelRegistry
    ) { }

    acquire(context: EditorContext): EditorHandle {
        const pooled = this.getReusableEditor();

        const model = this.modelRegistry.getOrCreate(
            context.modelUri,
            context.kind === "sql" ? "sql" : "json",
            context.kind
        );

        pooled.editor.setModel(model);

        if (pooled.snapshot?.modelUri === context.modelUri) {
            pooled.editor.restoreViewState(pooled.snapshot.viewState);
        }

        // Ensure container is cleared or we append correctly
        // The spec says: context.container.appendChild(pooled.editor.getDomNode()!)
        // We should probably remove it from its previous parent if any
        const container = typeof context.container === 'function' ? context.container() : context.container;
        if (!container) {
            console.warn("EditorPool: Container not ready during acquire");
            // Should we throw or return a handle that might fail to attach?
            // If we return, the editor exists but isn't in DOM.
            // We'll proceed, but warn.
        }

        const editorNode = pooled.editor.getDomNode();
        if (editorNode && container) {
            if (editorNode.parentElement && editorNode.parentElement !== container) {
                // Already attached elsewhere? It shouldn't be if it's inactive, 
                // but for safety we can remove it.
                // The logic below 'appendChild' moves the node, so strictly not required to remove first.
                // However, we need to ensure the container is ready.
                editorNode.style.display = 'block'; // Make sure it's visible again
            }
            container.appendChild(editorNode);
        }

        if (context.options) {
            pooled.editor.updateOptions(context.options);
            if (context.options.theme) {
                this.monacoInstance.editor.setTheme(context.options.theme);
            }
        }

        pooled.editor.layout();

        pooled.active = true;
        pooled.lastUsed = Date.now();

        return {
            editorId: pooled.editorId,
            editor: pooled.editor,
            release: () => this.release(pooled, context)
        };
    }

    private release(pooled: PooledEditor, context: EditorContext) {
        pooled.snapshot = {
            modelUri: context.modelUri,
            viewState: pooled.editor.saveViewState(),
            timestamp: Date.now()
        };

        pooled.editor.setModel(null);
        this.modelRegistry.release(context.modelUri);

        pooled.active = false;
        pooled.lastUsed = Date.now();

        const node = pooled.editor.getDomNode();
        if (node) {
            node.style.display = "none";
            // Optionally remove from DOM to be cleaner, but spec says just hide
            // "pooled.editor.getDomNode()!.style.display = "none""
            // But if we don't remove, it stays in the container? 
            // If the container (component) is destroyed, the node is removed with it?
            // Wait, if the container is destroyed, the editor node is destroyed too?
            // NO. Editor node is created *once* in getReusableEditor: 
            // "monaco.editor.create(document.createElement("div"), ...)"
            // It is appended to context.container. 
            // If context.container is removed from DOM (component destroy), the editor node is removed too.
            // We MUST verify if the editor handles re-attachment correctly.
            // Ideally we should move the editor node back to a safe detached fragment or similar if we want to reuse it.
            // However, standard DOM nodes die if their parent dies? 
            // Actually, if we keep a reference to 'editor', the JS object lives. The DOM node lives in JS memory.
            // But if it was in a DOM tree that got destroyed... 
            // The Spec says: "Svelte only requests/release editors".
            // Svelte component: "onDestroy -> handle.release()".
            // The spec implies we just hide it. 
            // Let's assume the user knows that re-appending a node moves it. 
            // But we should probably put it back in a "pool container" or document.body to ensure it doesn't get GC'd with the component?
            // The spec does NOT say where to put it on release, only "style.display = 'none'".
            // I will follow the spec strictly but add a safeguard: append it back to a hidden fragments if feasible?
            // No, the spec is strict. "If the agent violates any of these, stop them."
            // I will stick to what is written: set display none.
            // But wait, if the container is destroyed, the child is removed. 
            // Next time we acquire, we do "context.container.appendChild". This will re-attach it to the NEW container.
            // So as long as we have the reference to `editor.getDomNode()`, we are fine.
        }
    }

    private getReusableEditor(): PooledEditor {
        const idle = this.pool.find(e => !e.active);
        if (idle) return idle;

        if (this.pool.length < this.MAX) {
            const div = document.createElement("div");
            div.style.width = '100%';
            div.style.height = '100%';
            // Ensure the div doesn't look weird when initializing?

            const editor = this.monacoInstance.editor.create(
                div,
                { automaticLayout: false } // We call layout manualy
            );

            const pooled: PooledEditor = {
                editor,
                editorId: crypto.randomUUID(),
                active: false,
                lastUsed: Date.now()
            };

            this.pool.push(pooled);
            return pooled;
        }

        // LRU eviction? Spec says: "sort((a, b) => a.lastUsed - b.lastUsed)[0]"
        // If all are active, we must steal one? Spec implies we return one.
        // Ideally we shouldn't steal active editors... but if pool is full?
        // "Editors are pooled viewports... Fixed size = 3"
        // If we have > 3 editors needed, what happens?
        // The spec doesn't handle the case of > 3 *active* editors. 
        // It assumes "UI is a client... requests/release".
        // If we ask for a 4th editor, we steal the LRU one (which might be active?).
        // "getReusableEditor": looks for idle first.
        // If NO idle, what?
        // "return this.pool.sort...[0]"
        // This returns the LRU editor. If it's active, we are stealing it from another view?
        // That would check "active" flag?
        // The sort logic doesn't check 'active'.
        // If all 3 are active, we return the one used longest ago.
        // The caller of 'acquire' will then setModel and appendChild. 
        // This effectively "moves" the editor from the old component to the new one.
        // The old component will have an empty div? 
        // This seems like a valid "View Pool" behavior.
        return this.pool.sort((a, b) => a.lastUsed - b.lastUsed)[0];
    }
}

let poolInstance: EditorPool | null = null;
let registryInstance: ModelRegistry | null = null;

export function getWindowEditorPool(monaco?: typeof import('monaco-editor')): EditorPool {
    if (!poolInstance) {
        if (!monaco) {
            throw new Error("EditorPool not initialized and no monaco instance provided.");
        }
        // The comments below are from the user's instruction, kept for context.
        // const { ModelRegistry } = require('./model-registry'); // Dynamic import to avoid cycles if any, though imports are cleaner at top
        // However, we are in the same module structure.
        // Let's rely on standard imports.
        // We need to instantiate ModelRegistry here or pass it in?
        // The spec implies ModelRegistry is part of the infrastructure.

        registryInstance = new ModelRegistry(monaco);
        poolInstance = new EditorPool(monaco, registryInstance);
    }
    return poolInstance!;
}

export function getModelRegistry(): ModelRegistry {
    if (!registryInstance) {
        throw new Error("ModelRegistry not initialized. Ensure getWindowEditorPool has been called with monaco instance.");
    }
    return registryInstance;
}
