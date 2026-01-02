import type * as monaco from 'monaco-editor';
import { invoke } from '@tauri-apps/api/core';
import { schemaStore } from '$lib/stores/schema.svelte';

// Helper to prevent race conditions on the UI side
let currentRequestId = 0;

interface CompletionItemDto {
    label: string;
    kind: number;
    detail: string | null;
    insert_text: string;
    score: number;
}

export function registerSqlCompletion(monacoInstance: typeof monaco) {
    monacoInstance.languages.registerCompletionItemProvider('sql', {
        // Trigger on space, dot, etc.
        triggerCharacters: [' ', '.', '(', ','],

        provideCompletionItems: async (model, position) => {
            // Only proceed if we have an active connection
            if (!schemaStore.activeConnection) {
                return { suggestions: [] };
            }

            const myRequestId = ++currentRequestId;
            const text = model.getValue();
            const offset = model.getOffsetAt(position);
            const connectionId = schemaStore.activeConnection.id;

            try {
                // console.time("rust_completion");
                const items = await invoke<CompletionItemDto[]>('request_completions', {
                    connectionId,
                    text,
                    cursorOffset: offset,
                });
                // console.timeEnd("rust_completion");

                // If a newer request started while we were waiting, ignore this one
                if (myRequestId !== currentRequestId) {
                    return { suggestions: [] };
                }

                return {
                    suggestions: items.map((item) => {
                        const word = model.getWordUntilPosition(position);
                        const range = {
                            startLineNumber: position.lineNumber,
                            endLineNumber: position.lineNumber,
                            startColumn: word.startColumn,
                            endColumn: word.endColumn,
                        };

                        return {
                            label: item.label,
                            kind: mapKind(monacoInstance, item.kind),
                            insertText: item.insert_text,
                            detail: item.detail || undefined,
                            sortText: getSortText(item.score),
                            range: range,
                        };
                    })
                };
            } catch (e) {
                console.error("Completion failed:", e);
                return { suggestions: [] };
            }
        }
    });
}

// Map internal kind ID back to Monaco enum
// Rust CompletionKind: Table=0, Column=1, Alias=2, Keyword=3, Function=4, JoinCondition=5
function mapKind(m: typeof monaco, kind: number): monaco.languages.CompletionItemKind {
    switch (kind) {
        case 0: return m.languages.CompletionItemKind.Class;     // Table
        case 1: return m.languages.CompletionItemKind.Field;     // Column
        case 2: return m.languages.CompletionItemKind.Variable;  // Alias
        case 3: return m.languages.CompletionItemKind.Keyword;   // Keyword
        case 4: return m.languages.CompletionItemKind.Function;  // Function
        case 5: return m.languages.CompletionItemKind.Snippet;   // JoinCondition
        default: return m.languages.CompletionItemKind.Text;
    }
}

// Monaco sorts alphabetically by default. 
// We verify our score by generating a sort key that respects it.
// "9999" -> "0000" (Top)
// "0000" -> "9999"
function getSortText(score: number): string {
    // Invert score so higher score = lower alphabetical sort key
    // Max score is usually around 100-1000, but let's be safe
    const inverted = 100000 - score;
    return inverted.toString().padStart(6, '0');
}
