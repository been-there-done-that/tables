import * as monaco from 'monaco-editor';
import { invoke } from '@tauri-apps/api/core';

let debounceTimer: any;

export function enableQueryHighlighting(editor: monaco.editor.IStandaloneCodeEditor) {
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
                            className: 'current-query-border',
                        }
                    }
                ]);
            } else {
                decorationCollection.clear();
            }
        } catch (e) {
            console.error('Failed to get current statement range:', e);
        }
    };

    const debouncedUpdate = () => {
        clearTimeout(debounceTimer);
        debounceTimer = setTimeout(updateHighlight, 50); // Faster feedback than 100ms
    };

    editor.onDidChangeCursorPosition(debouncedUpdate);
    editor.onDidChangeModelContent(debouncedUpdate);

    // Initial highlight
    updateHighlight();

    return () => {
        decorationCollection.clear();
        clearTimeout(debounceTimer);
    };
}
