import * as monaco from 'monaco-editor';
import { invoke } from '@tauri-apps/api/core';

/**
 * Enables current statement isolation highlighting for a Monaco editor.
 * Uses local state to avoid conflicts in a pooled environment.
 */
export function enableQueryHighlighting(editor: monaco.editor.IStandaloneCodeEditor) {
    let debounceTimer: any;
    const decorationCollection = editor.createDecorationsCollection([]);

    const updateHighlight = async () => {
        const model = editor.getModel();
        if (!model || model.getLanguageId() !== 'sql') {
            decorationCollection.clear();
            return;
        }

        const position = editor.getPosition();
        if (!position) return;

        const offset = model.getOffsetAt(position);

        try {
            const range = await invoke<{ start_line: number, end_line: number } | null>('get_current_statement', {
                text: model.getValue(),
                cursorOffset: offset
            });

            if (range) {
                const decorations: monaco.editor.IModelDeltaDecoration[] = [];
                const start = range.start_line;
                const end = range.end_line;

                if (start === end) {
                    // Single line
                    decorations.push({
                        range: new monaco.Range(start, 1, start, 1),
                        options: {
                            isWholeLine: true,
                            className: 'query-border-full current-query-box',
                        }
                    });
                } else {
                    // Multi-line
                    // Top line
                    decorations.push({
                        range: new monaco.Range(start, 1, start, 1),
                        options: {
                            isWholeLine: true,
                            className: 'query-border-top current-query-box',
                        }
                    });

                    // Middle lines
                    if (end - start > 1) {
                        decorations.push({
                            range: new monaco.Range(start + 1, 1, end - 1, 1),
                            options: {
                                isWholeLine: true,
                                className: 'query-border-middle current-query-box',
                            }
                        });
                    }

                    // Bottom line
                    decorations.push({
                        range: new monaco.Range(end, 1, end, 1),
                        options: {
                            isWholeLine: true,
                            className: 'query-border-bottom current-query-box',
                        }
                    });
                }

                decorationCollection.set(decorations);
            } else {
                decorationCollection.clear();
            }
        } catch (e) {
            console.error('[Highlighting] Failed to get current statement range:', e);
        }
    };

    const debouncedUpdate = () => {
        clearTimeout(debounceTimer);
        debounceTimer = setTimeout(updateHighlight, 50);
    };

    const listeners = [
        editor.onDidChangeCursorPosition(debouncedUpdate),
        editor.onDidChangeModelContent(debouncedUpdate),
        editor.onDidChangeModel(debouncedUpdate) // Also update on model swap
    ];

    // Initial highlight
    updateHighlight();

    return () => {
        listeners.forEach(l => l.dispose());
        decorationCollection.clear();
        clearTimeout(debounceTimer);
    };
}
