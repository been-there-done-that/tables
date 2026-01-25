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
}

// CSS class for the run glyph
const GLYPH_CLASS = 'query-run-glyph';

/**
 * Enables glyph margin decorations for SQL queries.
 * 
 * IMPORTANT: CSS must be added for `.query-run-glyph`:
 * ```css
 * .query-run-glyph {
 *   background: url('data:image/svg+xml,...') center center no-repeat;
 *   cursor: pointer;
 * }
 * ```
 */
export function enableQueryGlyphMargin(
    editor: monaco.editor.IStandaloneCodeEditor,
    options: QueryGlyphOptions
): () => void {
    // Ensure glyph margin is NOT forced (we using line/gutter decorations now)
    // editor.updateOptions({ glyphMargin: true }); 

    const decorationCollection = editor.createDecorationsCollection([]);
    let statementRanges: StatementRangeWithBytes[] = [];
    let debounceTimer: any;

    const updateGlyphs = async () => {
        const model = editor.getModel();
        if (!model || model.getLanguageId() !== 'sql') {
            decorationCollection.clear();
            return;
        }

        const text = model.getValue();

        try {
            statementRanges = await invoke<StatementRangeWithBytes[]>('get_all_statements', { text });

            const decorations: monaco.editor.IModelDeltaDecoration[] = statementRanges
                .filter(range => {
                    const queryText = text.substring(range.start_byte, range.end_byte);
                    return queryText.trim().length > 0;
                })
                .map(range => ({
                    range: new monaco.Range(range.start_line, 1, range.start_line, 1),
                    options: {
                        linesDecorationsClassName: GLYPH_CLASS, // Use line decorations (right of line numbers)
                        stickiness: monaco.editor.TrackedRangeStickiness.NeverGrowsWhenTypingAtEdges
                    }
                }));

            decorationCollection.set(decorations);
        } catch (e) {
            console.error('[GlyphMargin] Failed to get statements:', e);
        }
    };

    const debouncedUpdate = () => {
        clearTimeout(debounceTimer);
        debounceTimer = setTimeout(updateGlyphs, 100);
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
                    options.onExecute(queryText, matchingRange.start_line, matchingRange.end_line);
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
