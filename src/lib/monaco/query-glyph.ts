/**
 * Monaco Glyph Margin decorations for SQL query run buttons.
 * Shows a play icon (▷) in the left margin of each SQL statement.
 */

import * as monaco from 'monaco-editor';
import { invoke } from '@tauri-apps/api/core';

interface StatementRangeWithBytes {
    start_line: number;
    end_line: number;
    start_byte: number;
    end_byte: number;
}

interface QueryGlyphOptions {
    /** Callback when glyph is clicked */
    onExecute: (queryText: string, startLine: number, endLine: number) => void;
    /** Callback when stop is clicked */
    onStop?: (startLine: number, endLine: number) => void;
    /** Function to check which lines are currently running */
    getRunningLines?: () => Set<number>;
}

// CSS classes
const RUN_CLASS = 'query-run-glyph';
const STOP_CLASS = 'query-stop-glyph';

export function enableQueryGlyphMargin(
    editor: monaco.editor.IStandaloneCodeEditor,
    options: QueryGlyphOptions
): () => void {
    const decorationCollection = editor.createDecorationsCollection([]);
    let statementRanges: StatementRangeWithBytes[] = [];
    let debounceTimer: any;
    // We need to re-render decorations when running state changes
    // This is triggered externally by re-calling updateGlyphs or similar mechanism if we exposed it,
    // but here we might need a way to force update.
    // For now, let's rely on the parent component triggering updates or just polling/event.
    // Actually, simply re-running updateGlyphs() will fetch the latest state.

    // To allow external trigger, we can attach this to the editor instance or return it.
    // But for simplicity, we'll just use the getRunningLines() inside updateGlyphs.

    const updateGlyphs = async () => {
        const model = editor.getModel();
        if (!model || model.getLanguageId() !== 'sql') {
            decorationCollection.clear();
            return;
        }

        const text = model.getValue();
        const runningLines = options.getRunningLines ? options.getRunningLines() : new Set<number>();

        try {
            statementRanges = await invoke<StatementRangeWithBytes[]>('get_all_statements', { text });

            const decorations: monaco.editor.IModelDeltaDecoration[] = statementRanges
                .filter(range => {
                    const queryText = text.substring(range.start_byte, range.end_byte);
                    return queryText.trim().length > 0;
                })
                .map(range => {
                    const isRunning = runningLines.has(range.start_line);
                    return {
                        range: new monaco.Range(range.start_line, 1, range.start_line, 1),
                        options: {
                            linesDecorationsClassName: isRunning ? STOP_CLASS : RUN_CLASS,
                            stickiness: monaco.editor.TrackedRangeStickiness.NeverGrowsWhenTypingAtEdges
                        }
                    };
                });

            decorationCollection.set(decorations);
        } catch (e) {
            console.error('[GlyphMargin] Failed to get statements:', e);
        }
    };

    const debouncedUpdate = () => {
        clearTimeout(debounceTimer);
        debounceTimer = setTimeout(updateGlyphs, 100);
    };

    // Poll for running state changes (simple approach since we don't have a signal)
    const runningStateInterval = setInterval(() => {
        // Only update if we have statements, to avoid expensive calls
        if (statementRanges.length > 0 && options.getRunningLines) {
            // In a perfect world we'd diff the state, but re-setting decorations is fast enough
            // optimization: only update if running set size changed or specific lines changed?
            // For now, just re-run updateGlyphs sync part (we can split it).
            // Let's just call updateGlyphs() - if it invokes tauri every time it's bad.
            // Refactor: split parsing and decoration setting.
            updateDecorationsOnly();
        }
    }, 200);

    const updateDecorationsOnly = () => {
        const runningLines = options.getRunningLines ? options.getRunningLines() : new Set<number>();
        const decorations: monaco.editor.IModelDeltaDecoration[] = statementRanges.map(range => {
            const isRunning = runningLines.has(range.start_line);
            return {
                range: new monaco.Range(range.start_line, 1, range.start_line, 1),
                options: {
                    linesDecorationsClassName: isRunning ? STOP_CLASS : RUN_CLASS,
                    stickiness: monaco.editor.TrackedRangeStickiness.NeverGrowsWhenTypingAtEdges
                }
            };
        });
        decorationCollection.set(decorations);
    };

    // Handle glyph click
    const mouseDownDisposable = editor.onMouseDown((e) => {
        // Check for click in line decorations gutter (where our icon is)
        if (e.target.type === monaco.editor.MouseTargetType.GUTTER_LINE_DECORATIONS) {
            const lineNumber = e.target.position?.lineNumber;
            if (!lineNumber) return;

            // Check if the click element was actually our icon
            // (Standard line decorations click might trigger on the whole width, we want to be nice)
            // But Monaco doesn't easily differentiate "which" decoration class was clicked in the event target detail.
            // Since we only put one actionable thing there, if they click the gutter decoration area for that line, run it.

            // Find the statement at this line
            const model = editor.getModel();
            if (!model) return;

            const text = model.getValue();
            const matchingRange = statementRanges.find(
                r => lineNumber >= r.start_line && lineNumber <= r.end_line
            );

            if (matchingRange) {
                const queryText = text.substring(matchingRange.start_byte, matchingRange.end_byte);
                if (queryText.trim()) {
                    const runningLines = options.getRunningLines ? options.getRunningLines() : new Set<number>();
                    if (runningLines.has(matchingRange.start_line)) {
                        // Stop query
                        if (options.onStop) {
                            options.onStop(matchingRange.start_line, matchingRange.end_line);
                        }
                    } else {
                        // Run query
                        options.onExecute(queryText, matchingRange.start_line, matchingRange.end_line);
                    }
                }
            }
        }
    });

    // Update glyphs on content/model changes
    const contentDisposable = editor.onDidChangeModelContent(debouncedUpdate);
    const modelDisposable = editor.onDidChangeModel(debouncedUpdate);

    // Initial update
    updateGlyphs();

    return () => {
        clearTimeout(debounceTimer);
        decorationCollection.clear();
        mouseDownDisposable.dispose();
        contentDisposable.dispose();
        modelDisposable.dispose();
    };
}
