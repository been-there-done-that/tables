<script lang="ts">
    import { onMount } from "svelte";
    import * as monaco from "monaco-editor";
    import editorWorker from "monaco-editor/esm/vs/editor/editor.worker?worker";
    import jsonWorker from "monaco-editor/esm/vs/language/json/json.worker?worker";

    let container: HTMLDivElement;
    let logs: string[] = $state([]);

    function log(msg: string) {
        console.log(`[MONACO-RAW] ${msg}`);
        logs = [
            ...logs,
            `${new Date().toISOString().split("T")[1].substring(0, 12)} - ${msg}`,
        ];
    }

    onMount(() => {
        log("Setting up MonacoEnvironment...");

        // Setup workers
        (self as any).MonacoEnvironment = {
            getWorker(_: string, label: string) {
                if (label === "json") {
                    return new jsonWorker();
                }
                return new editorWorker();
            },
        };

        log("Creating editor directly...");

        const editor = monaco.editor.create(container, {
            value: JSON.stringify({ hello: "world", test: true }, null, 2),
            language: "json",
            theme: "vs-dark",
            automaticLayout: true,
            minimap: { enabled: false },
        });

        log(`Editor created. DOM node exists: ${!!editor.getDomNode()}`);

        const domNode = editor.getDomNode();
        if (domNode) {
            log(
                `DOM node dimensions: ${domNode.offsetWidth}x${domNode.offsetHeight}`,
            );
            log(
                `DOM node computed style display: ${getComputedStyle(domNode).display}`,
            );
        }

        const model = editor.getModel();
        log(
            `Model exists: ${!!model}, value: ${model?.getValue().substring(0, 30)}...`,
        );

        // Force layout
        setTimeout(() => {
            editor.layout();
            log("Forced layout after 100ms");
        }, 100);

        return () => {
            editor.dispose();
        };
    });
</script>

<div class="p-8 space-y-4 bg-gray-900 text-white min-h-screen font-mono">
    <h1 class="text-2xl font-bold">Raw Monaco Test (No Pool)</h1>
    <p class="text-sm text-gray-400">
        This bypasses our EditorPool entirely to test if Monaco works at all.
    </p>

    <div class="flex gap-4">
        <div class="w-1/2 border border-blue-500 h-[500px] relative">
            <div
                bind:this={container}
                class="absolute inset-0 w-full h-full"
            ></div>
        </div>

        <div
            class="w-1/2 border border-gray-600 h-[500px] overflow-auto p-2 bg-gray-800 text-xs"
        >
            <h2 class="font-bold mb-2">Logs:</h2>
            {#each logs as l}
                <div class="border-b border-gray-700 py-1">{l}</div>
            {/each}
        </div>
    </div>
</div>
