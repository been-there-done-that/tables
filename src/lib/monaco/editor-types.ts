import type * as monaco from 'monaco-editor';

export type EditorKind = "sql" | "json" | "inline";

export interface EditorContext {
    contextId: string;            // stable logical ID
    windowId: string;             // per-window isolation
    kind: EditorKind;
    modelUri: string;
    container: HTMLElement | (() => HTMLElement | null);
    options?: monaco.editor.IStandaloneEditorConstructionOptions;
}

export interface EditorHandle {
    editorId: string;
    editor: monaco.editor.IStandaloneCodeEditor;
    release(): void;
}

export interface EditorViewSnapshot {
    modelUri: string;
    viewState: monaco.editor.ICodeEditorViewState | null;
    timestamp: number;
}
