import { onMount, onDestroy } from "svelte";
import { preloadMonaco } from "./monaco-runtime";
import { getWindowEditorPool } from "./editor-pool";
import type { EditorContext, EditorHandle } from "./editor-types";

export function useMonacoEditor(context: EditorContext, onReady?: (handle: EditorHandle) => void) {
    let handle: EditorHandle | null = null;

    onMount(async () => {
        const monaco = await preloadMonaco();
        const pool = getWindowEditorPool(monaco);
        handle = pool.acquire(context);
        if (onReady && handle) {
            onReady(handle);
        }
    });

    onDestroy(() => {
        handle?.release();
        handle = null;
    });
}
