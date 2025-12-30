<script lang="ts">
    import { onMount } from "svelte";
    import { useMonacoEditor } from "$lib/monaco/useMonacoEditor";
    import { getWindowEditorPool } from "$lib/monaco/editor-pool";
    import type { EditorHandle } from "$lib/monaco/editor-types";

    let container: HTMLDivElement | null = null;
    let isMounted = $state(true);
    let logs = $state<string[]>([]);

    function log(msg: string) {
        logs = [...logs, `${new Date().toISOString().split("T")[1]} - ${msg}`];
    }

    const initialJson = JSON.stringify(
        { hello: "world", debug: true },
        null,
        2,
    );
    const modelUri = "json://debug-page";

    let handle: EditorHandle | null = null;

    useMonacoEditor(
        {
            contextId: "debug-editor",
            windowId: "main",
            kind: "json",
            modelUri: modelUri,
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
            handle = h;
            log("Editor acquired!");
            h.editor.setValue(initialJson);
            log("Value set to initial JSON");

            // Log dimensions
            const dom = h.editor.getDomNode();
            if (dom) {
                log(`DOM dimensions: ${dom.clientWidth}x${dom.clientHeight}`);
            } else {
                log("DOM node not found on editor");
            }

            h.editor.onDidLayoutChange((e) => {
                log(`Layout changed: ${e.width}x${e.height}`);
            });

            // Debug model
            const model = h.editor.getModel();
            if (model) {
                log(
                    `Model attached: ${model.uri.toString()} (${model.getLineCount()} lines)`,
                );
                log(
                    `Model value snippet: ${model.getValue().substring(0, 20)}...`,
                );
            } else {
                log("ERROR: No model attached to editor!");
            }
        },
    );

    function toggleMount() {
        isMounted = !isMounted;
        log(`Mounted state: ${isMounted}`);
    }

    async function checkPoolParams() {
        if (typeof window === "undefined") return;
        // @ts-ignore
        const pool = getWindowEditorPool();
        log("Checking pool...");
    }
</script>

<div class="p-8 space-y-4 bg-gray-900 text-white min-h-screen font-mono">
    <h1 class="text-2xl font-bold">Monaco Debug Page</h1>

    <div class="flex gap-4">
        <button class="px-3 py-1 bg-blue-600 rounded" onclick={toggleMount}>
            {isMounted ? "Unmount Editor" : "Mount Editor"}
        </button>
        <button
            class="px-3 py-1 bg-green-600 rounded"
            onclick={() => handle?.editor.layout()}
        >
            Force Layout
        </button>
    </div>

    <div class="flex gap-4">
        <!-- Editor Area -->
        <div class="w-1/2 border border-gray-600 h-[500px] relative">
            {#if isMounted}
                <div
                    bind:this={container}
                    class="absolute inset-0 w-full h-full"
                ></div>
            {:else}
                <div
                    class="flex items-center justify-center h-full text-gray-500"
                >
                    Editor Unmounted
                </div>
            {/if}
        </div>

        <!-- Logs -->
        <div
            class="w-1/2 border border-gray-600 h-[500px] overflow-auto p-2 bg-gray-800 text-xs"
        >
            {#each logs as l}
                <div class="border-b border-gray-700 py-1">{l}</div>
            {/each}
        </div>
    </div>
</div>
