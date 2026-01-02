import type * as monaco from 'monaco-editor';
import { ModelRegistry } from './model-registry';
import type { EditorContext, EditorHandle, EditorViewSnapshot } from './editor-types';
import { MONACO_THEME_NAME } from './monaco-theme';

interface PooledEditor {
    editor: monaco.editor.IStandaloneCodeEditor;
    containerDiv: HTMLDivElement;
    editorId: string;
    active: boolean;
    lastUsed: number;
    snapshot?: EditorViewSnapshot;
}

export class EditorPool {
    public pool: PooledEditor[] = []; // Changed to public for dev-only health probe access
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

        if (pooled.snapshot?.modelUri === context.modelUri) {
            pooled.editor.restoreViewState(pooled.snapshot.viewState);
            console.log("[EditorPool] Restored view state");
        }

        const container = typeof context.container === 'function' ? context.container() : context.container;

        if (!container) {
            console.warn("[EditorPool] Container not ready during acquire!");
        }

        // Move the entire containerDiv to preserve Monaco's event delegation
        if (container) {
            pooled.containerDiv.style.position = 'absolute';
            pooled.containerDiv.style.top = '0';
            pooled.containerDiv.style.left = '0';
            pooled.containerDiv.style.width = '100%';
            pooled.containerDiv.style.height = '100%';
            pooled.containerDiv.style.display = 'block';
            container.appendChild(pooled.containerDiv);

            const w = container.clientWidth;
            const h = container.clientHeight;
            if (w > 0 && h > 0) {
                pooled.editor.layout({ width: w, height: h });
            }
        }

        if (context.options) {
            pooled.editor.updateOptions(context.options);
            if (context.options.theme) {
                this.monacoInstance.editor.setTheme(context.options.theme);
            } else {
                this.monacoInstance.editor.setTheme(MONACO_THEME_NAME);
            }
        } else {
            this.monacoInstance.editor.setTheme(MONACO_THEME_NAME);
        }

        pooled.active = true;
        pooled.lastUsed = Date.now();

        // Focus after attach + layout
        pooled.editor.focus();

        console.log("[EditorPool] Acquire complete", {
            editorId: pooled.editorId,
            hasTextFocus: pooled.editor.hasTextFocus()
        });

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

        // Move containerDiv back to pool
        const poolContainer = document.getElementById('monaco-editor-pool');
        if (poolContainer) {
            pooled.containerDiv.style.display = 'block';
            poolContainer.appendChild(pooled.containerDiv);
            console.log("[EditorPool] Editor released back to pool");
        }
    }

    private getReusableEditor(): PooledEditor {
        const idle = this.pool.find(e => !e.active);
        if (idle) return idle;

        if (this.pool.length < this.MAX) {
            // Pool container: offscreen, real dimensions, no pointer-events:none
            let poolContainer = document.getElementById('monaco-editor-pool');
            if (!poolContainer) {
                poolContainer = document.createElement('div');
                poolContainer.id = 'monaco-editor-pool';
                poolContainer.style.cssText = `
                    position: fixed;
                    top: -10000px;
                    left: -10000px;
                    width: 800px;
                    height: 600px;
                    overflow: hidden;
                    opacity: 0;
                `;
                document.body.appendChild(poolContainer);
                console.log("[EditorPool] Created pool container");
            }

            // Container that moves WITH the editor
            const containerDiv = document.createElement("div");
            containerDiv.style.cssText = `
                position: absolute;
                inset: 0;
            `;
            poolContainer.appendChild(containerDiv);

            const editor = this.monacoInstance.editor.create(containerDiv, {
                theme: MONACO_THEME_NAME,
                automaticLayout: false,
                minimap: { enabled: false },
                lineNumbersMinChars: 3,       // Reduce gutter width
                lineDecorationsWidth: 8,      // Narrow decoration area (acts as border spacing)
                glyphMargin: false,           // Remove extra glyph margin
            });

            const pooled: PooledEditor = {
                editor,
                containerDiv,
                editorId: crypto.randomUUID(),
                active: false,
                lastUsed: Date.now()
            };

            this.pool.push(pooled);
            console.log("[EditorPool] New editor created", { editorId: pooled.editorId });
            return pooled;
        }

        // LRU eviction
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

/**
 * Dev-only health probe to monitor Monaco's internal state.
 * Answers: are we leaking? is layout broken? are font metrics sane?
 */
export function getMonacoHealth(pool: EditorPool, monaco: typeof import("monaco-editor")) {
    const editors = pool.pool;
    const models = monaco.editor.getModels();
    const sqlModels = models.filter(m => m.uri.scheme === "sql");
    const jsonModels = models.filter(m => m.uri.scheme === "json");

    let layoutInfo = null;
    let metrics = null;
    const activeEditor = editors.find(e => e.active)?.editor;
    if (activeEditor) {
        layoutInfo = activeEditor.getLayoutInfo();
        const options = activeEditor.getOptions();

        // Try multiple ways to get font metrics to be safe
        const fontInfo = options.get((monaco.editor.EditorOption as any).fontInfo);
        const lLineHeight = options.get((monaco.editor.EditorOption as any).lineHeight);

        metrics = {
            lineHeight: lLineHeight || (layoutInfo as any).lineHeight || 0,
            charWidth: fontInfo?.typicalHalfwidthCharacterWidth || (layoutInfo as any).typicalHalfwidthCharacterWidth || 0
        };
    }

    const warnings: string[] = [];
    if (layoutInfo && metrics) {
        if (metrics.lineHeight <= 0) warnings.push("Invalid lineHeight");
        if (metrics.charWidth <= 0) warnings.push("Invalid char width");
        if (layoutInfo.width < 50 || layoutInfo.height < 50) warnings.push("Suspicious editor size");
    }

    // Check for "ghost" editors (more than MAX)
    if (editors.length > 3) warnings.push("Exceeded editor pool limit");

    return {
        editors: {
            total: editors.length,
            active: editors.filter(e => e.active).length,
            idle: editors.filter(e => !e.active).length,
            instances: editors.map(e => ({
                id: e.editorId,
                active: e.active
            }))
        },
        models: {
            total: models.length,
            sql: sqlModels.length,
            json: jsonModels.length
        },
        layout: layoutInfo && metrics
            ? {
                width: layoutInfo.width,
                height: layoutInfo.height,
                lineHeight: metrics.lineHeight,
                charWidth: metrics.charWidth
            }
            : null,
        warnings
    };
}
