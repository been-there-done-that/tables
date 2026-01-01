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

    useMonacoEditor({
        contextId: "sql-test-page",
        windowId: "main",
        kind: "sql", 
        modelUri: "file:///test.sql", // .sql extension triggers the SQL language mode
        container: () => editorContainer,
        options: {
            theme: MONACO_THEME_NAME,
            minimap: { enabled: true },
            automaticLayout: true,
            fontSize: 14,
            fontFamily: "Fira Code, monospace",
        }
    }, (handle) => {
        editorHandle = handle;
        log("Editor initialized");
        
        handle.editor.setValue("-- SQL Auto-Completion Test\n-- Type 'SELECT' or table names to see suggestions\n\nSELECT * FROM ");
        handle.editor.focus();
        handle.editor.setPosition({ lineNumber: 4, column: 15 });
    });

    onDestroy(() => {
        // Cleanup handled by useMonacoEditor usually, but we can log
        log("Destroying SQL test page");
    });
</script>

<div class="flex h-screen w-full flex-col bg-background text-foreground">
    <div class="flex items-center justify-between border-b border-border px-4 py-2">
        <h1 class="text-lg font-semibold">SQL Completion Test</h1>
        <div class="text-xs text-muted-foreground">Context: sql-test-page</div>
    </div>
    
    <div class="flex flex-1 overflow-hidden">
        <!-- Editor Area -->
        <div class="flex-1 relative border-r border-border">
            <div 
                bind:this={editorContainer} 
                class="absolute inset-0 w-full h-full"
            ></div>
        </div>
        
        <!-- Logs / Instructions -->
        <div class="w-64 flex flex-col bg-muted/10 p-4 space-y-4">
            <div>
                <h3 class="font-bold text-sm mb-2">Instructions</h3>
                <p class="text-xs text-muted-foreground">
                    1. Ensure you have an active connection.
                    2. Type a query.
                    3. Suggestions should appear triggered by space, dot, or typing.
                </p>
            </div>
            
            <div class="flex-1 overflow-hidden flex flex-col">
                <h3 class="font-bold text-sm mb-2">Logs</h3>
                <div class="flex-1 overflow-auto bg-muted/20 p-2 rounded text-[10px] font-mono border border-border">
                    {#each logs as l}
                        <div class="border-b border-border/50 py-1">{l}</div>
                    {/each}
                </div>
            </div>
        </div>
    </div>
</div>
