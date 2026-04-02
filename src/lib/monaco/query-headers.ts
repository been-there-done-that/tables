/**
 * Manages "Rich Query Headers" inside Monaco Editor.
 * Inserts ViewZones (vertical space) above queries and mounts Svelte components (ContentWidgets) into them.
 */

import * as monaco from 'monaco-editor';
import { invoke } from '@tauri-apps/api/core';
import { mount, unmount } from 'svelte';
import QueryHeader from '$lib/components/editor/QueryHeader.svelte';

interface StatementRangeWithBytes {
    start_line: number;
    end_line: number;
    start_byte: number;
    end_byte: number;
}

export interface HeaderStatus {
    state: 'idle' | 'running' | 'success' | 'error';
    duration?: number;
    errorMessage?: string;
}

interface HeaderInstance {
    decorationId: string;
    line: number;
    viewZoneId: string;
    widget: monaco.editor.IContentWidget;
    component: any; // Svelte component instance
    domNode: HTMLElement;
    lastText: string;
}

export class QueryHeaderController {
    private editor: monaco.editor.IStandaloneCodeEditor;
    private headers: Map<string, HeaderInstance> = new Map(); // decorationId -> instance
    private statuses: Map<string, HeaderStatus> = new Map(); // decorationId -> status
    private debounceTimer: any;
    private onRunCallback: (text: string, startLine: number, endLine: number) => void;
    private onStopCallback: (startLine: number, endLine: number) => void;
    private isDisposed = false;

    // Cache for active detection
    private cachedRanges: StatementRangeWithBytes[] = [];
    private activeLine: number = 0;
    private _showAll: boolean = false;

    // Last non-empty selection — cached because clicking a button clears Monaco's selection
    private cachedSelection: monaco.Selection | null = null;

    public get showAll() { return this._showAll; }
    public set showAll(v: boolean) {
        if (this._showAll === v) return;
        this._showAll = v;
        this.reconcileHeaders();
    }

    constructor(
        editor: monaco.editor.IStandaloneCodeEditor,
        onRun: (text: string, start: number, end: number) => void,
        onStop: (start: number, end: number) => void
    ) {
        this.editor = editor;
        this.onRunCallback = onRun;
        this.onStopCallback = onStop;

        // Cache selection so it survives the focus-loss that happens when a button is clicked.
        // Also re-run reconcile so the header immediately appears/disappears as selection changes.
        editor.onDidChangeCursorSelection((e) => {
            this.cachedSelection = e.selection.isEmpty() ? null : e.selection;
            this.reconcileHeaders();
        });

        // Listen for changes
        editor.onDidChangeModelContent(() => this.scheduleUpdate(200));
        editor.onDidChangeModel(() => {
            this.clearAll();
            this.cachedRanges = [];
            this.updateHeaders();
        });

        // Initial load
        this.updateHeaders();
    }

    public onCursor(line: number) {
        if (this.activeLine === line) return;
        this.activeLine = line;

        // Re-evaluate headers without re-parsing
        this.reconcileHeaders();
    }

    public updateStatus(line: number, _text: string, status: HeaderStatus) {
        if (this.isDisposed) return;
        const model = this.editor.getModel();
        if (!model) return;

        let targetId: string | null = null;
        for (const [id] of this.headers.entries()) {
            const range = model.getDecorationRange(id);
            if (range && range.startLineNumber === line) {
                targetId = id;
                break;
            }
        }

        if (targetId) {
            this.statuses.set(targetId, status);
            this.refreshComponent(targetId);
        }
    }

    private refreshComponent(decorationId: string) {
        const header = this.headers.get(decorationId);
        if (!header) return;

        const status = this.statuses.get(decorationId) || { state: 'idle' };
        const model = this.editor.getModel();
        if (!model) return;

        // Unmount old if exists
        if (header.component) {
            unmount(header.component);
            header.component = null;
        }

        // Mount Svelte Component
        header.component = mount(QueryHeader, {
            target: header.domNode,
            props: {
                status: status.state,
                duration: status.duration,
                errorMessage: status.errorMessage,
                onRun: () => {
                    // Use the cached selection — clicking the button clears Monaco's live selection
                    // before this handler fires, so editor.getSelection() would already be empty.
                    if (this.cachedSelection) {
                        const selectedText = model.getValueInRange(this.cachedSelection);
                        this.onRunCallback(selectedText, this.cachedSelection.startLineNumber, this.cachedSelection.endLineNumber);
                        this.cachedSelection = null; // consume it
                        return;
                    }
                    const latestRange = model.getDecorationRange(decorationId);
                    if (latestRange) {
                        const fullRange = new monaco.Range(
                            latestRange.startLineNumber,
                            1,
                            latestRange.endLineNumber,
                            model.getLineMaxColumn(latestRange.endLineNumber)
                        );
                        const latestText = model.getValueInRange(fullRange);
                        this.onRunCallback(latestText, latestRange.startLineNumber, latestRange.endLineNumber);
                    }
                },
                onStop: () => {
                    const latestRange = model.getDecorationRange(decorationId);
                    if (latestRange) {
                        this.onStopCallback(latestRange.startLineNumber, latestRange.endLineNumber);
                    }
                }
            }
        });
    }

    private scheduleUpdate(ms: number) {
        clearTimeout(this.debounceTimer);
        this.debounceTimer = setTimeout(() => this.updateHeaders(), ms);
    }

    private async updateHeaders() {
        if (this.isDisposed) return;
        const model = this.editor.getModel();
        if (!model || model.getLanguageId() !== 'sql') return;

        const text = model.getValue();
        try {
            this.cachedRanges = await invoke<StatementRangeWithBytes[]>('get_all_statements', { text });
        } catch (e) {
            console.error("Failed to parse statements:", e);
            return;
        }

        this.reconcileHeaders();
    }

    /**
     * Reconciles the visible headers based on cachedRanges and activeLine.
     * ONLY the active statement gets a header.
     */
    private reconcileHeaders() {
        if (this.isDisposed) return;
        const model = this.editor.getModel();
        if (!model) return;

        const text = model.getValue();
        const nextHeaders: Map<string, HeaderInstance> = new Map();

        // If showAll is true, we want ALL ranges to have headers.
        // If not, we ONLY want the active statement to get a header.

        let activeRanges = this.cachedRanges;
        if (!this._showAll) {
            if (this.cachedSelection) {
                // When text is selected, show header for the first statement overlapping the selection.
                // This works for both selection directions and Select All.
                const selStart = this.cachedSelection.startLineNumber;
                const selEnd = this.cachedSelection.endLineNumber;
                const range = this.cachedRanges.find(r =>
                    r.start_line <= selEnd && r.end_line >= selStart
                );
                activeRanges = range ? [range] : [];
            } else {
                const range = this.cachedRanges.find(r =>
                    this.activeLine >= r.start_line && this.activeLine <= r.end_line
                );
                activeRanges = range ? [range] : [];
            }
        }

        // 1. Identify which existing header matches the active ranges (to preserve status/state).
        // 2. Remove all others.
        // 3. Create new if needed.

        // Map existing headers to ranges for easier lookup
        type ExistingHeader = { id: string, instance: HeaderInstance, startLine: number };
        const existing: ExistingHeader[] = [];

        for (const [id, instance] of this.headers.entries()) {
            const range = model.getDecorationRange(id);
            if (!range) {
                // Invalid decoration, clean up immediately
                this.removeHeader(id);
                continue;
            }
            existing.push({ id, instance, startLine: range.startLineNumber });
        }

        // Logic for Active Ranges
        for (const range of activeRanges) {
            // Check if we already have a header for this line
            const match = existing.find(e => e.startLine === range.start_line);

            if (match) {
                // KEEP existing
                const { id, instance } = match;

                // Update text tracking
                const queryText = text.substring(range.start_byte, range.end_byte);
                if (instance.lastText !== queryText) {
                    this.statuses.set(id, { state: 'idle' });
                    instance.lastText = queryText;
                    this.refreshComponent(id);
                }

                nextHeaders.set(id, instance);

                // Remove it from 'existing' so we know what to delete
                const idx = existing.indexOf(match);
                if (idx > -1) existing.splice(idx, 1);
            } else {
                // CREATE new
                const queryText = text.substring(range.start_byte, range.end_byte);
                this.createHeader(range.start_line, range.end_line, queryText, nextHeaders);
            }
        }

        // DELETE all remaining (non-active)
        for (const item of existing) {
            this.removeHeader(item.id);
        }

        this.headers = nextHeaders;
    }

    private createHeader(startLine: number, endLine: number, text: string, collection: Map<string, HeaderInstance>) {
        const model = this.editor.getModel();
        if (!model) return;

        // Add Decoration
        const decorationIds = model.deltaDecorations([], [{
            range: new monaco.Range(startLine, 1, endLine, 1),
            options: { isWholeLine: true }
        }]);
        const id = decorationIds[0];

        // Create DOM node
        const domNode = document.createElement('div');
        domNode.className = 'query-header-widget';

        // Add ViewZone
        let viewZoneId = '';
        this.editor.changeViewZones(accessor => {
            viewZoneId = accessor.addZone({
                afterLineNumber: startLine - 1,
                heightInLines: 1.4,
                domNode: document.createElement('div'),
            });
        });

        // Add ContentWidget
        const widgetId = `query.header.${id}.${Date.now()}`;
        const widget: monaco.editor.IContentWidget = {
            getId: () => widgetId,
            getDomNode: () => domNode,
            getPosition: () => {
                const r = model.getDecorationRange(id);
                return {
                    position: { lineNumber: r ? r.startLineNumber : startLine, column: 1 },
                    preference: [monaco.editor.ContentWidgetPositionPreference.ABOVE]
                };
            }
        };
        this.editor.addContentWidget(widget);

        const instance: HeaderInstance = {
            decorationId: id,
            line: startLine,
            viewZoneId,
            widget,
            domNode,
            component: null,
            lastText: text
        };

        collection.set(id, instance);
        this.statuses.set(id, { state: 'idle' });
        this.headers.set(id, instance);
        this.refreshComponent(id);
    }

    private removeHeader(id: string) {
        const h = this.headers.get(id);
        if (h) {
            this.editor.changeViewZones(accessor => {
                accessor.removeZone(h.viewZoneId);
            });
            this.editor.removeContentWidget(h.widget);
            if (h.component) unmount(h.component);
            const model = this.editor.getModel();
            if (model) model.deltaDecorations([id], []);
            this.headers.delete(id);
            this.statuses.delete(id);
        }
    }

    public clearAll() {
        for (const id of Array.from(this.headers.keys())) {
            this.removeHeader(id);
        }
        this.headers.clear();
        this.statuses.clear();
    }

    public dispose() {
        this.isDisposed = true;
        this.clearAll();
    }
}

/**
 * Helper to enable rich headers
 */
export function enableQueryHeaders(
    editor: monaco.editor.IStandaloneCodeEditor,
    onRun: (text: string, start: number, end: number) => void,
    onStop: (start: number, end: number) => void
): QueryHeaderController {
    return new QueryHeaderController(editor, onRun, onStop);
}
