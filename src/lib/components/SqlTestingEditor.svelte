<script lang="ts">
    import { onMount, onDestroy } from "svelte";
    import { useMonacoEditor } from "$lib/monaco/useMonacoEditor";
    import type { EditorHandle } from "$lib/monaco/editor-types";
    import { MONACO_THEME_NAME } from "$lib/monaco/monaco-theme";
    import { cn } from "$lib/utils";
    import IconPlayerPlay from "@tabler/icons-svelte/icons/player-play";
    import * as monaco from "monaco-editor";

    let editorContainer: HTMLElement;
    let editorHandle: EditorHandle | null = null;
    let logs: string[] = $state([]);

    function log(msg: string) {
        logs = [
            `${new Date().toISOString().split("T")[1].substring(0, 12)} - ${msg}`,
            ...logs,
        ];
    }

    async function executeCurrent() {
        if (!editorHandle) return;
        const editor = editorHandle.editor;
        const model = editor.getModel();
        if (!model) return;

        let query = "";
        let source = "";

        // 1. Check for manual selection first
        const selection = editor.getSelection();
        if (selection && !selection.isEmpty()) {
            query = model.getValueInRange(selection);
            source = "manual selection";
        } else {
            // 2. Fallback to auto-highlighted statement
            const decorations = editor.getDecorationsInRange(
                new monaco.Range(1, 1, model.getLineCount(), 1),
            );
            const highlight = decorations?.find(
                (d) => d.options.className === "current-query-border",
            );

            if (highlight) {
                query = model.getValueInRange(highlight.range);
                source = "auto-highlighted statement";
            } else {
                // 3. Fallback to full text
                query = editor.getValue();
                source = "full text";
            }
        }

        if (query.trim()) {
            console.log(`[Execute] Running query from ${source}:`, query);
            log(`Executing (${source}):\n${query}`);
        } else {
            log("No query to execute");
        }
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

            // Add Command+Enter / Ctrl+Enter shortcut
            handle.editor.addCommand(
                monaco.KeyMod.CtrlCmd | monaco.KeyCode.Enter,
                () => {
                    executeCurrent();
                },
            );

            // Only set value if empty
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
    </div>
</div>
