<script lang="ts">
    import { onMount, onDestroy } from "svelte";
    import { useMonacoEditor } from "$lib/monaco/useMonacoEditor";
    import type { EditorHandle } from "$lib/monaco/editor-types";
    import { MONACO_THEME_NAME } from "$lib/monaco/monaco-theme";
    import { cn } from "$lib/utils";
    import IconPlayerPlay from "@tabler/icons-svelte/icons/player-play";
    import * as monaco from "monaco-editor";
    import * as DropdownMenu from "$lib/components/ui/dropdown-menu";
    import { schemaStore } from "$lib/stores/schema.svelte";
    import IconDatabase from "@tabler/icons-svelte/icons/database";
    import IconSchema from "@tabler/icons-svelte/icons/table";
    import IconChevronDown from "@tabler/icons-svelte/icons/chevron-down";

    import { settingsStore } from "$lib/stores/settings.svelte";
    import { windowState } from "$lib/stores/window.svelte";

    import { invoke } from "@tauri-apps/api/core";

    let { context = $bindable({}) } = $props<{ context?: any }>();

    let editorContainer: HTMLElement;
    let editorHandle = $state<EditorHandle | null>(null);
    let logs: string[] = $state([]);

    // Toolbar state
    // Use schemaStore.activeSchema instead of local state
    // We synchronize it with context if provided
    $effect(() => {
        if (context?.schemaName) {
            schemaStore.activeSchema = context.schemaName;
        }
    });

    // Reactive font settings
    $effect(() => {
        if (editorHandle?.editor) {
            const family = settingsStore.editorFontFamily.includes(" ")
                ? `"${settingsStore.editorFontFamily}"`
                : settingsStore.editorFontFamily;

            editorHandle.editor.updateOptions({
                fontFamily: family,
                fontSize: settingsStore.editorFontSize,
            });
        }
    });

    const currentSchemas = $derived.by(() => {
        const dbName = schemaStore.selectedDatabase;
        if (!dbName) return [];
        const db = schemaStore.databases.find((d) => d.name === dbName);
        return db?.schemas || [];
    });

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
            log(
                `Executing (${source}) in ${schemaStore.selectedDatabase}.${schemaStore.activeSchema}:\n${query}`,
            );

            if (!schemaStore.activeConnection) {
                log("No active connection selected.");
                return;
            }

            try {
                const result = await invoke("execute_query", {
                    connectionId: schemaStore.activeConnection.id,
                    database: schemaStore.selectedDatabase,
                    schema: schemaStore.activeSchema || "public",
                    query: query,
                    component: "editor",
                });
                console.log("Query Result:", result);
                log("Query completed successfully.");
            } catch (e) {
                console.error("Query execution failed:", e);
                log(`Query failed: ${e}`);
            }
        } else {
            log("No query to execute");
        }
    }

    function handleExplain(raw: boolean = false) {
        log(`Explain ${raw ? "(Raw)" : ""} functionality not implemented yet.`);
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
                fontSize: settingsStore.editorFontSize,
                fontFamily: settingsStore.editorFontFamily,
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
                if (context?.content) {
                    handle.editor.setValue(context.content);
                } else {
                    handle.editor.setValue(
                        `-- SQL Auto-Completion Playground\n-- Context: ${schemaStore.selectedDatabase}.${schemaStore.activeSchema}\n-- Type 'SELECT' or table names from your active connection\n\nSELECT * FROM `,
                    );
                    handle.editor.setPosition({ lineNumber: 4, column: 15 });
                }
            } else if (
                context?.content &&
                handle.editor.getValue() !== context.content
            ) {
                // Should we overwrite if editor has content? Usually creating new editor starts empty.
                // But if restoring, it should be empty initially.
                // Safe to assume we can set it if provided and we are just initing.
                handle.editor.setValue(context.content);
            }

            // Listen for content changes
            handle.editor.onDidChangeModelContent(() => {
                const val = handle.editor.getValue();
                if (context) {
                    context.content = val;
                }
                // Trigger save
                windowState.requestSave();
            });

            handle.editor.focus();
        },
    );
</script>

<div class="flex h-full w-full flex-col bg-background">
    <!-- Toolbar -->
    <div
        class="flex h-8 items-center justify-between border-b border-border bg-muted/20 px-2 gap-2"
    >
        <div class="flex items-center gap-2">
            <button
                class="flex items-center gap-1 rounded bg-(--theme-accent-primary) px-3 py-1 text-xs font-semibold text-white hover:opacity-90 transition-opacity"
                onclick={executeCurrent}
                title="Run (Cmd+Enter)"
            >
                <IconPlayerPlay class="size-3 fill-current" />
                Run
            </button>

            <DropdownMenu.Root>
                <DropdownMenu.Trigger
                    class="flex items-center gap-1 rounded border border-border bg-background px-3 py-1 text-xs font-medium hover:bg-accent hover:text-accent-foreground transition-colors"
                >
                    Explain
                    <IconChevronDown class="size-3 text-muted-foreground" />
                </DropdownMenu.Trigger>
                <DropdownMenu.Content align="start">
                    <DropdownMenu.Item onclick={() => handleExplain(false)}
                        >Explain Plan</DropdownMenu.Item
                    >
                    <DropdownMenu.Item onclick={() => handleExplain(true)}
                        >Explain Plan (Raw)</DropdownMenu.Item
                    >
                </DropdownMenu.Content>
            </DropdownMenu.Root>
        </div>

        <div class="flex items-center gap-2">
            <!-- Schema Picker -->
            <DropdownMenu.Root>
                <DropdownMenu.Trigger
                    class="flex items-center gap-1.5 rounded border border-border bg-background px-3 py-1 text-xs font-medium hover:bg-accent hover:text-accent-foreground transition-colors"
                    title="Select Schema"
                >
                    <IconSchema class="size-3 text-muted-foreground" />
                    <span class="truncate max-w-[150px]"
                        >{schemaStore.activeSchema || "public"}</span
                    >
                    <IconChevronDown
                        class="ml-auto size-3 text-muted-foreground"
                    />
                </DropdownMenu.Trigger>
                <DropdownMenu.Content
                    align="end"
                    class="min-w-[120px] w-max max-w-[300px] max-h-[300px] overflow-auto"
                >
                    <DropdownMenu.Label>Schemas</DropdownMenu.Label>
                    <DropdownMenu.Separator />
                    <DropdownMenu.RadioGroup
                        value={schemaStore.activeSchema || undefined}
                        onValueChange={(v) => (schemaStore.activeSchema = v)}
                    >
                        {#each currentSchemas as schema (schema.name)}
                            <DropdownMenu.RadioItem value={schema.name}>
                                {schema.name}
                            </DropdownMenu.RadioItem>
                        {/each}
                    </DropdownMenu.RadioGroup>
                </DropdownMenu.Content>
            </DropdownMenu.Root>
        </div>
    </div>

    <div class="flex-1 relative">
        <div
            bind:this={editorContainer}
            class="absolute inset-0 w-full h-full"
        ></div>
    </div>
</div>
