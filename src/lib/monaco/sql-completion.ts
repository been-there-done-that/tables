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
    trigger_suggest?: boolean;  // If true, trigger completions again after selection
}

export function registerSqlCompletion(monacoInstance: typeof monaco) {
    monacoInstance.languages.registerCompletionItemProvider('sql', {
        // Trigger on space, dot, etc.
        triggerCharacters: [' ', '.', '(', ','],

        provideCompletionItems: async (model, position) => {
            // Only proceed if we have an active connection
            if (!schemaStore.activeConnection) {
                console.log('[Completion] No active connection');
                return { suggestions: [] };
            }

            const myRequestId = ++currentRequestId;
            const text = model.getValue();
            const offset = model.getOffsetAt(position);
            const connectionId = schemaStore.activeConnection.id;

            console.log('[Completion] Request:', {
                requestId: myRequestId,
                offset,
                defaultSchema: schemaStore.activeSchema,
                textSnippet: text.slice(Math.max(0, offset - 50), offset + 10),
            });

            try {
                console.time(`[Completion] Request #${myRequestId}`);
                const items = await invoke<CompletionItemDto[]>('request_completions', {
                    connectionId,
                    text,
                    cursorOffset: offset,
                    defaultSchema: schemaStore.activeSchema,
                });
                console.timeEnd(`[Completion] Request #${myRequestId}`);

                console.log(`[Completion] Received ${items.length} items from backend`);
                if (items.length > 0) {
                    console.log('[Completion] First 5 items:', items.slice(0, 5));
                }

                // If a newer request started while we were waiting, ignore this one
                if (myRequestId !== currentRequestId) {
                    console.log('[Completion] Stale request, ignoring');
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

                        const suggestion: monaco.languages.CompletionItem = {
                            label: item.label,
                            kind: mapKind(monacoInstance, item.kind),
                            insertText: item.insert_text,
                            detail: item.detail || undefined,
                            sortText: getSortText(item.score),
                            range: range,
                        };

                        // Add command to trigger completions again after selecting this item
                        // This enables chained completions (e.g., schema. -> tables)
                        if (item.trigger_suggest) {
                            suggestion.command = {
                                id: 'editor.action.triggerSuggest',
                                title: 'Trigger Suggest',
                            };
                        }

                        return suggestion;
                    })
                };
            } catch (e) {
                console.error("[Completion] Request failed:", e);
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
