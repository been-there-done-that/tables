/**
 * Monaco CodeLens provider for query execution status.
 * Shows "Running..." or "✔ 123ms" ABOVE the query.
 */

import * as monaco from 'monaco-editor';

export interface QueryStatus {
    state: 'running' | 'success' | 'error';
    durationMs?: number;
    errorMessage?: string;
    ranges: { startLine: number; endLine: number };
}

export function createQueryStatusLensProvider(
    monacoInstance: typeof monaco,
    getStatus: () => QueryStatus[]
): monaco.IDisposable {

    // Command that does nothing (just for display)
    const noopCommandId = `tables.statusLens.noop.${Date.now()}`;
    const noopCommand = monacoInstance.editor.registerCommand(noopCommandId, () => { });

    const provider = monacoInstance.languages.registerCodeLensProvider('sql', {
        provideCodeLenses: (model, _token) => {
            const statuses = getStatus();
            const lenses: monaco.languages.CodeLens[] = [];

            for (const status of statuses) {
                let title = '';
                let tooltip = '';

                switch (status.state) {
                    case 'running':
                        title = 'Running...'; // Spinner handled by glyph
                        tooltip = 'Query is executing';
                        break;
                    case 'success':
                        title = `✓ ${formatDuration(status.durationMs || 0)}`;
                        tooltip = 'Previous execution time';
                        break;
                    case 'error':
                        title = '⚠ Error';
                        tooltip = status.errorMessage || 'Query failed';
                        break;
                }

                lenses.push({
                    range: new monacoInstance.Range(status.ranges.startLine, 1, status.ranges.startLine, 1),
                    command: {
                        id: noopCommandId,
                        title: title,
                        tooltip: tooltip
                    }
                });
            }

            return { lenses, dispose: () => { } };
        },
        resolveCodeLens: (_model, codeLens, _token) => {
            return codeLens;
        }
    });

    return {
        dispose: () => {
            provider.dispose();
            noopCommand.dispose();
        }
    };
}

function formatDuration(ms: number): string {
    if (ms < 1000) return `${ms}ms`;
    return `${(ms / 1000).toFixed(2)}s`;
}

/**
 * Helper to enable status lens on an editor.
 */
export function enableQueryStatusLens(
    editor: monaco.editor.IStandaloneCodeEditor,
    monacoInstance: typeof monaco,
    getStatus: () => QueryStatus[]
): () => void {
    const provider = createQueryStatusLensProvider(monacoInstance, getStatus);

    // Trigger update when model content changes (though status usually changes independently)
    const contentDisposable = editor.onDidChangeModelContent(() => {
        // Monaco automatically updates lenses
    });

    return () => {
        provider.dispose();
        contentDisposable.dispose();
    };
}
