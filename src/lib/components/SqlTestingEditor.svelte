<script lang="ts">
    import { onMount, onDestroy } from "svelte";
    import { useMonacoEditor } from "$lib/monaco/useMonacoEditor";
    import type { EditorHandle } from "$lib/monaco/editor-types";
    import { MONACO_THEME_NAME } from "$lib/monaco/monaco-theme";
    import { cn } from "$lib/utils";

    let editorContainer: HTMLElement;
    let editorHandle: EditorHandle | null = null;
    let logs: string[] = $state([]);

    function log(msg: string) {
        logs = [
            `${new Date().toISOString().split("T")[1].substring(0, 12)} - ${msg}`,
            ...logs,
        ];
    }

    useMonacoEditor(
        {
            contextId: "sql-main-playground",
            windowId: "main",
            kind: "sql",
            modelUri: "file:///playground.sql",
            container: () => editorContainer,
            options: {
                theme: MONACO_THEME_NAME,
                minimap: { enabled: false },
                automaticLayout: true,
                fontSize: 14,
                fontFamily: "Fira Code, monospace",
            },
        },
        (handle) => {
            editorHandle = handle;
            log("Editor initialized");

            // Only set value if empty to preserve content if component stays alive (though it likely won't)
            if (!handle.editor.getValue()) {
                handle.editor.setValue(
                    "-- SQL Auto-Completion Playground\n-- Type 'SELECT' or table names from your active connection\n\nSELECT * FROM ",
                );
                handle.editor.setPosition({ lineNumber: 4, column: 15 });
            }
            handle.editor.focus();
        },
    );
</script>

<div class="flex h-full w-full flex-col bg-background">
    <div class="flex-1 relative">
        <div
            bind:this={editorContainer}
            class="absolute inset-0 w-full h-full"
        ></div>

        <!-- Overlay Log Panel (Collapsible or small) -->
        <div
            class="absolute bottom-4 right-4 w-64 max-h-48 flex flex-col bg-muted/90 backdrop-blur border border-border rounded-lg shadow-lg text-[10px] overflow-hidden"
        >
            <div
                class="px-2 py-1 bg-muted font-bold border-b border-border flex justify-between"
            >
                <span>Event Log</span>
                <span class="text-muted-foreground">{logs.length} events</span>
            </div>
            <div class="flex-1 overflow-auto p-2 font-mono space-y-1">
                {#each logs as l}
                    <div class="border-b border-border/50 pb-0.5 last:border-0">
                        {l}
                    </div>
                {/each}
                {#if logs.length === 0}
                    <div class="text-muted-foreground italic">Ready...</div>
                {/if}
            </div>
        </div>
    </div>
</div>
