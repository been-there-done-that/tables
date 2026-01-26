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
                decorationCollection.set([
                    {
                        range: new monaco.Range(range.start_line, 1, range.end_line, 1),
                        options: {
                            isWholeLine: true,
                            className: 'current-query-highlight',
                        }
                    }
                ]);
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
