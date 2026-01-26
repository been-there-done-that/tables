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

    constructor(
        editor: monaco.editor.IStandaloneCodeEditor,
        onRun: (text: string, start: number, end: number) => void,
        onStop: (start: number, end: number) => void
    ) {
        this.editor = editor;
        this.onRunCallback = onRun;
        this.onStopCallback = onStop;

        // Listen for changes
        editor.onDidChangeModelContent(() => this.scheduleUpdate(200));
        editor.onDidChangeModel(() => {
            this.clearAll();
            this.updateHeaders();
        });

        // Initial load
        this.updateHeaders();
    }

    public updateStatus(line: number, text: string, status: HeaderStatus) {
        if (this.isDisposed) return;
        const model = this.editor.getModel();
        if (!model) return;

        let targetId: string | null = null;
        for (const [id, instance] of this.headers.entries()) {
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

        const range = model.getDecorationRange(decorationId);
        if (!range) return;

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
                    const latestRange = model.getDecorationRange(decorationId);
                    if (latestRange) {
                        // Expand the range to cover full width of lines
                        const fullRange = new monaco.Range(
                            latestRange.startLineNumber,
                            1,
                            latestRange.endLineNumber,
                            model.getLineMaxColumn(latestRange.endLineNumber)
                        );
                        const latestText = model.getValueInRange(fullRange);
                        this.onRunCallback(latestText, latestRange.startLineNumber, latestRange.endLineNumber);
                    } else {
                        console.warn(`[Header] No range found for decoration ${decorationId}`);
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
        let ranges: StatementRangeWithBytes[] = [];

        try {
            ranges = await invoke<StatementRangeWithBytes[]>('get_all_statements', { text });
        } catch (e) {
            console.error("Failed to parse statements:", e);
            return;
        }

        const nextHeaders: Map<string, HeaderInstance> = new Map();
        const rangeMap: Map<number, StatementRangeWithBytes> = new Map();
        ranges.forEach(r => rangeMap.set(r.start_line, r));

        // 1. Reconcile existing headers
        for (const [id, instance] of this.headers.entries()) {
            const range = model.getDecorationRange(id);
            if (!range) {
                this.removeHeader(id);
                continue;
            }

            const newRange = rangeMap.get(range.startLineNumber);
            if (newRange) {
                // Same line, same header. Update text if needed.
                const queryText = text.substring(newRange.start_byte, newRange.end_byte);

                // Track if it moved (Monaco moved the decoration)
                if (instance.line !== range.startLineNumber) {
                    this.updateViewZone(instance, range.startLineNumber);
                    instance.line = range.startLineNumber;
                    this.editor.layoutContentWidget(instance.widget);
                }

                if (instance.lastText !== queryText) {
                    this.statuses.set(id, { state: 'idle' });
                    instance.lastText = queryText;
                    this.refreshComponent(id);
                }
                nextHeaders.set(id, instance);
                rangeMap.delete(range.startLineNumber); // Marked as used
            } else {
                // Gone or moved. Remove.
                this.removeHeader(id);
            }
        }

        // 2. Create new headers for remaining ranges
        for (const [line, range] of rangeMap.entries()) {
            const queryText = text.substring(range.start_byte, range.end_byte);

            // Add Decoration
            const decorationIds = model.deltaDecorations([], [{
                range: new monaco.Range(line, 1, range.end_line, 1),
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
                    afterLineNumber: line - 1,
                    heightInLines: 1.4, // Slightly more compact for ghost design
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
                        position: { lineNumber: r ? r.startLineNumber : line, column: 1 },
                        preference: [monaco.editor.ContentWidgetPositionPreference.ABOVE]
                    };
                }
            };
            this.editor.addContentWidget(widget);

            const instance: HeaderInstance = {
                decorationId: id,
                line,
                viewZoneId,
                widget,
                domNode,
                component: null,
                lastText: queryText
            };

            nextHeaders.set(id, instance);
            this.statuses.set(id, { state: 'idle' });
            this.headers.set(id, instance); // Update primary map
            this.refreshComponent(id);
        }

        // Final sync
        this.headers = nextHeaders;
    }

    private updateViewZone(instance: HeaderInstance, newLine: number) {
        this.editor.changeViewZones(accessor => {
            accessor.removeZone(instance.viewZoneId);
            instance.viewZoneId = accessor.addZone({
                afterLineNumber: newLine - 1,
                heightInLines: 1.4,
                domNode: document.createElement('div'),
            });
        });
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
