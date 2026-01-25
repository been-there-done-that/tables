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
    id: string;
    line: number;
    viewZoneId: string;
    widget: monaco.editor.IContentWidget;
    component: any; // Svelte component instance
    domNode: HTMLElement;
}

export class QueryHeaderController {
    private editor: monaco.editor.IStandaloneCodeEditor;
    private headers: HeaderInstance[] = [];
    private statuses: Map<number, HeaderStatus> = new Map(); // line -> status
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
        editor.onDidChangeModelContent(() => this.scheduleUpdate());
        editor.onDidChangeModel(() => this.updateHeaders()); // Immediate on model switch

        // Initial load
        this.updateHeaders();
    }

    public updateStatus(line: number, status: HeaderStatus) {
        this.statuses.set(line, status);

        // Update existing component props directly if found
        const header = this.headers.find(h => h.line === line);
        if (header && header.component) {
            // In Svelte 5 with $props, we can't easily update props from outside unless we use accessors
            // OR re-mount. But simpler: just force full update since this is rare event (start/end)
            // Ideally we'd validly update the component props.
            // Let's re-mount for correctness for now, or use a store if we passed one.
            // Actually, we can just re-create the component on the existing node.

            unmount(header.component);
            header.component = mount(QueryHeader, {
                target: header.domNode,
                props: {
                    status: status.state,
                    duration: status.duration,
                    errorMessage: status.errorMessage,
                    onRun: () => this.handleRun(line),
                    onStop: () => this.handleStop(line)
                }
            });
        }
    }

    private scheduleUpdate() {
        clearTimeout(this.debounceTimer);
        this.debounceTimer = setTimeout(() => this.updateHeaders(), 300);
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

        // Diff and reconcile headers
        // Simple approach: Clear all and rebuild. 
        // Optimization: In real prod we'd diff, but for < 100 queries this is fast enough.

        // 1. Remove old ViewZones and Widgets
        this.editor.changeViewZones(accessor => {
            this.headers.forEach(h => {
                accessor.removeZone(h.viewZoneId);
                this.editor.removeContentWidget(h.widget);
                unmount(h.component);
            });
        });
        this.headers = [];

        // 2. Add new ViewZones and Widgets
        this.editor.changeViewZones(accessor => {
            ranges.forEach(range => {
                const line = range.start_line;

                // Skip if line invalid
                if (line > model.getLineCount()) return;

                // Create DOM node for widget
                const domNode = document.createElement('div');
                domNode.className = 'query-header-widget';

                // Insert ViewZone (space)
                const viewZoneId = accessor.addZone({
                    afterLineNumber: line - 1, // Insert above the line
                    heightInLines: 1.6, // Enough space for button
                    domNode: document.createElement('div'), // Placeholder
                });

                // Create Content Widget
                const widgetId = `query.header.${line}.${Date.now()}`;
                const widget: monaco.editor.IContentWidget = {
                    getId: () => widgetId,
                    getDomNode: () => domNode,
                    getPosition: () => ({
                        position: { lineNumber: line, column: 1 },
                        preference: [monaco.editor.ContentWidgetPositionPreference.ABOVE]
                    })
                };

                this.editor.addContentWidget(widget);

                // Mount Svelte Component
                const currentStatus = this.statuses.get(line) || { state: 'idle' };

                // Helper execution handlers
                const onRun = () => {
                    const queryText = text.substring(range.start_byte, range.end_byte);
                    this.onRunCallback(queryText, range.start_line, range.end_line);
                };

                const onStop = () => {
                    this.onStopCallback(range.start_line, range.end_line);
                };

                const component = mount(QueryHeader, {
                    target: domNode,
                    props: {
                        status: currentStatus.state,
                        duration: currentStatus.duration,
                        errorMessage: currentStatus.errorMessage,
                        onRun,
                        onStop
                    }
                });

                this.headers.push({
                    id: widgetId,
                    line,
                    viewZoneId,
                    widget,
                    component,
                    domNode
                });
            });
        });
    }

    private handleRun(line: number) {
        // Find range from current headers (bit redundant but safe)
        // Accessing from closure in mount() is cleaner, see above.
    }

    private handleStop(line: number) {
        // Accessing from closure in mount() is cleaner.
    }

    public dispose() {
        this.isDisposed = true;
        this.editor.changeViewZones(accessor => {
            this.headers.forEach(h => {
                accessor.removeZone(h.viewZoneId);
                this.editor.removeContentWidget(h.widget);
                unmount(h.component);
            });
        });
        this.headers = [];
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
