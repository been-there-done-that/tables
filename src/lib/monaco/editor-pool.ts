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
        console.log("[EditorPool] acquire called", { modelUri: context.modelUri, kind: context.kind });

        const pooled = this.getReusableEditor();
        console.log("[EditorPool] Got pooled editor", { editorId: pooled.editorId, active: pooled.active });

        const model = this.modelRegistry.getOrCreate(
            context.modelUri,
            context.kind === "sql" ? "sql" : "json",
            context.kind
        );
        console.log("[EditorPool] Model created/retrieved", { uri: model.uri.toString(), lineCount: model.getLineCount() });

        pooled.editor.setModel(model);
        console.log("[EditorPool] Model attached to editor");

        if (pooled.snapshot?.modelUri === context.modelUri) {
            pooled.editor.restoreViewState(pooled.snapshot.viewState);
            console.log("[EditorPool] Restored view state");
        }

        const container = typeof context.container === 'function' ? context.container() : context.container;
        console.log("[EditorPool] Container resolved", { exists: !!container, dimensions: container ? `${container.offsetWidth}x${container.offsetHeight}` : 'N/A' });

        if (!container) {
            console.warn("[EditorPool] Container not ready during acquire!");
        }

        const editorNode = pooled.editor.getDomNode();
        console.log("[EditorPool] Editor DOM node", { exists: !!editorNode });

        if (editorNode && container) {
            editorNode.style.position = 'absolute';
            editorNode.style.top = '0';
            editorNode.style.left = '0';
            editorNode.style.width = '100%';
            editorNode.style.height = '100%';
            editorNode.style.display = 'block';
            container.appendChild(editorNode);
            console.log("[EditorPool] Editor DOM appended to container");
        }

        if (context.options) {
            pooled.editor.updateOptions(context.options);
            if (context.options.theme) {
                this.monacoInstance.editor.setTheme(context.options.theme);
            }
            console.log("[EditorPool] Options applied", context.options);
        }

        pooled.editor.layout();
        console.log("[EditorPool] Layout triggered");

        setTimeout(() => {
            pooled.editor.layout();
            const dom = pooled.editor.getDomNode();
            console.log("[EditorPool] Delayed layout", { dimensions: dom ? `${dom.offsetWidth}x${dom.offsetHeight}` : 'N/A' });
        }, 100);

        pooled.active = true;
        pooled.lastUsed = Date.now();

        console.log("[EditorPool] Acquire complete, returning handle");
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

        // CRITICAL: Move editor back to pool container to keep it attached to DOM
        // This prevents the editor from being destroyed if the component is destroyed
        const node = pooled.editor.getDomNode();
        const poolContainer = document.getElementById('monaco-editor-pool');
        if (node && poolContainer) {
            node.style.display = 'block'; // Keep it visible (just offscreen)
            poolContainer.appendChild(node);
            console.log("[EditorPool] Editor moved back to pool container");
        }
    }

    private getReusableEditor(): PooledEditor {
        const idle = this.pool.find(e => !e.active);
        if (idle) return idle;

        if (this.pool.length < this.MAX) {
            // CRITICAL: Monaco editors MUST be created in an ATTACHED DOM element
            // Use position:fixed with offscreen positioning (NOT visibility:hidden or display:none)
            // IMPORTANT: Container must have real dimensions for Monaco to calculate font metrics correctly
            let poolContainer = document.getElementById('monaco-editor-pool');
            if (!poolContainer) {
                poolContainer = document.createElement('div');
                poolContainer.id = 'monaco-editor-pool';
                poolContainer.style.position = 'fixed';
                poolContainer.style.top = '-10000px';
                poolContainer.style.left = '-10000px';
                poolContainer.style.width = '800px';  // Real dimensions needed for font metrics
                poolContainer.style.height = '600px';
                poolContainer.style.overflow = 'hidden';
                poolContainer.style.pointerEvents = 'none';
                document.body.appendChild(poolContainer);
                console.log("[EditorPool] Created hidden pool container (position: fixed, 800x600)");
            }

            const editorDiv = document.createElement("div");
            editorDiv.style.width = '100%';
            editorDiv.style.height = '100%';
            editorDiv.style.position = 'absolute';
            editorDiv.style.top = '0';
            editorDiv.style.left = '0';
            poolContainer.appendChild(editorDiv);

            console.log("[EditorPool] Creating new editor in attached pool container");
            const editor = this.monacoInstance.editor.create(editorDiv, {
                theme: 'vs-dark',
                automaticLayout: true,
                minimap: { enabled: false },
                language: 'json'
            });

            const pooled: PooledEditor = {
                editor,
                editorId: crypto.randomUUID(),
                active: false,
                lastUsed: Date.now()
            };

            this.pool.push(pooled);
            console.log("[EditorPool] New editor created", { editorId: pooled.editorId });
            return pooled;
        }

        // LRU eviction: return the least recently used editor
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
