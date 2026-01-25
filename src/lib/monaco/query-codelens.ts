/**
 * Monaco CodeLens provider for inline SQL query run buttons.
 * Shows "▷ Run" above each SQL statement in the editor.
 */

import * as monaco from 'monaco-editor';
import { invoke } from '@tauri-apps/api/core';

interface StatementRangeWithBytes {
    start_line: number;
    end_line: number;
    start_byte: number;
    end_byte: number;
}

interface QueryCodeLensOptions {
    /** Callback when "Run" is clicked */
    onExecute: (queryText: string, startLine: number, endLine: number) => void;
    /** Optional callback for "Explain" button */
    onExplain?: (queryText: string, startLine: number, endLine: number) => void;
}

/**
 * Creates a CodeLens provider for SQL queries.
 * Returns a disposable and the command IDs for cleanup.
 */
export function createQueryCodeLensProvider(
    monacoInstance: typeof monaco,
    options: QueryCodeLensOptions
): { provider: monaco.IDisposable; commands: monaco.IDisposable[] } {
    // Generate unique command IDs to avoid conflicts
    const runCommandId = `tables.runQuery.${Date.now()}`;
    const explainCommandId = `tables.explainQuery.${Date.now()}`;

    // Register the run command
    const runCommand = monacoInstance.editor.registerCommand(runCommandId, (_accessor, queryText: string, startLine: number, endLine: number) => {
        options.onExecute(queryText, startLine, endLine);
    });

    // Register explain command if callback provided
    const commands: monaco.IDisposable[] = [runCommand];
    if (options.onExplain) {
        const explainCommand = monacoInstance.editor.registerCommand(explainCommandId, (_accessor, queryText: string, startLine: number, endLine: number) => {
            options.onExplain!(queryText, startLine, endLine);
        });
        commands.push(explainCommand);
    }

    // Create the CodeLens provider
    const provider = monacoInstance.languages.registerCodeLensProvider('sql', {
        provideCodeLenses: async (model, _token) => {
            const text = model.getValue();

            try {
                const ranges = await invoke<StatementRangeWithBytes[]>('get_all_statements', { text });

                const lenses: monaco.languages.CodeLens[] = [];

                for (const range of ranges) {
                    // Extract query text using byte offsets
                    const queryText = text.substring(range.start_byte, range.end_byte);

                    // Skip empty or whitespace-only statements
                    if (!queryText.trim()) continue;

                    // Run button
                    lenses.push({
                        range: new monacoInstance.Range(range.start_line, 1, range.start_line, 1),
                        command: {
                            id: runCommandId,
                            title: '▷ Run',
                            tooltip: 'Execute this query',
                            arguments: [queryText, range.start_line, range.end_line]
                        }
                    });

                    // Explain button (if handler provided)
                    if (options.onExplain) {
                        lenses.push({
                            range: new monacoInstance.Range(range.start_line, 1, range.start_line, 1),
                            command: {
                                id: explainCommandId,
                                title: 'Explain',
                                tooltip: 'Show query execution plan',
                                arguments: [queryText, range.start_line, range.end_line]
                            }
                        });
                    }
                }

                return { lenses, dispose: () => { } };
            } catch (e) {
                console.error('[CodeLens] Failed to get statements:', e);
                return { lenses: [], dispose: () => { } };
            }
        },
        resolveCodeLens: (_model, codeLens, _token) => {
            return codeLens;
        }
    });

    return { provider, commands };
}

/**
 * Helper to enable query CodeLens on an editor.
 * Returns a cleanup function.
 */
export function enableQueryCodeLens(
    editor: monaco.editor.IStandaloneCodeEditor,
    monacoInstance: typeof monaco,
    options: QueryCodeLensOptions
): () => void {
    const { provider, commands } = createQueryCodeLensProvider(monacoInstance, options);

    // Force refresh CodeLenses when content changes
    const contentDisposable = editor.onDidChangeModelContent(() => {
        // Monaco will automatically trigger provideCodeLenses
    });

    return () => {
        provider.dispose();
        commands.forEach(cmd => cmd.dispose());
        contentDisposable.dispose();
    };
}
