<script lang="ts">
    import { useMonacoEditor } from "$lib/monaco/useMonacoEditor";
    import type { EditorHandle } from "$lib/monaco/editor-types";

    let { onAcquired, onLog } = $props<{
        onAcquired: (handle: EditorHandle) => void;
        onLog: (msg: string) => void;
    }>();

    let container: HTMLDivElement | null = null;

    useMonacoEditor(
        {
            contextId: "debug-editor",
            windowId: "main",
            kind: "json",
            modelUri: "json://debug-page",
            container: () => container,
            options: {
                theme: "vs-dark",
                automaticLayout: true,
                minimap: { enabled: true },
                lineNumbers: "on",
                glyphMargin: true,
            },
        },
        (h) => {
            onAcquired(h);
            onLog("Editor acquired!");

            const dom = h.editor.getDomNode();
            if (dom) {
                onLog(`DOM: ${dom.clientWidth}x${dom.clientHeight}`);
            }
        },
    );
</script>

<div bind:this={container} class="w-full h-full"></div>
