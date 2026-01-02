import * as monaco from 'monaco-editor';
import { invoke } from '@tauri-apps/api/core';
import { schemaStore } from '$lib/stores/schema.svelte';

/**
 * Enables SQL diagnostics for a Monaco editor instance.
 * Handles model swaps and ensures validation is debounced per-editor.
 */
export function enableDiagnostics(editor: monaco.editor.IStandaloneCodeEditor) {
    let diagnosticTimeout: any;
    let contentChangeListener: monaco.IDisposable | null = null;

    const validate = async () => {
        const model = editor.getModel();
        if (!model || model.getLanguageId() !== 'sql') return;

        try {
            // Use active connection if available, otherwise fallback to "default"
            const connectionId = schemaStore.activeConnection?.id || "default";

            const diagnostics = await invoke<any[]>('request_diagnostics', {
                text: model.getValue(),
                connectionId
            });

            const markers = diagnostics.map(err => {
                const startPos = model.getPositionAt(err.start);
                const endPos = model.getPositionAt(err.end);

                return {
                    severity: err.severity === 1 ? monaco.MarkerSeverity.Error : monaco.MarkerSeverity.Warning,
                    message: err.message,
                    startLineNumber: startPos.lineNumber,
                    startColumn: startPos.column,
                    endLineNumber: endPos.lineNumber,
                    endColumn: endPos.column,
                    source: 'SQL Parser'
                };
            });

            monaco.editor.setModelMarkers(model, 'sql', markers);
        } catch (e) {
            console.error("[Diagnostics] Validation failed", e);
        }
    };

    const triggerValidation = () => {
        clearTimeout(diagnosticTimeout);
        diagnosticTimeout = setTimeout(validate, 500);
    };

    const attachToModel = (model: monaco.editor.ITextModel | null) => {
        if (contentChangeListener) {
            contentChangeListener.dispose();
            contentChangeListener = null;
        }

        if (model && model.getLanguageId() === 'sql') {
            contentChangeListener = model.onDidChangeContent(triggerValidation);
            triggerValidation();
        }
    };

    // Listen for model swaps (crucial for Pooled Editors)
    const modelChangeListener = editor.onDidChangeModel(() => {
        attachToModel(editor.getModel());
    });

    // Initial attachment
    attachToModel(editor.getModel());

    return () => {
        modelChangeListener.dispose();
        if (contentChangeListener) {
            contentChangeListener.dispose();
        }
        clearTimeout(diagnosticTimeout);

        const model = editor.getModel();
        if (model) {
            monaco.editor.setModelMarkers(model, 'sql', []);
        }
    };
}
