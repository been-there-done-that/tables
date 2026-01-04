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

    let { context = {} } = $props<{ context?: any }>();

    let editorContainer: HTMLElement;
    let editorHandle: EditorHandle | null = null;
    let logs: string[] = $state([]);

    // Toolbar state
    // We only track schema locally, database comes from global store
    let selectedSchema = $state(context?.schemaName || "public");

    // Sync state with context if it changes
    $effect(() => {
        if (context?.schemaName) selectedSchema = context.schemaName;
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
                `Executing (${source}) in ${schemaStore.selectedDatabase}.${selectedSchema}:\n${query}`,
            );
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
                    `-- SQL Auto-Completion Playground\n-- Context: ${schemaStore.selectedDatabase}.${selectedSchema}\n-- Type 'SELECT' or table names from your active connection\n\nSELECT * FROM `,
                );
                handle.editor.setPosition({ lineNumber: 4, column: 15 });
            }
            handle.editor.focus();
        },
    );
</script>

<div class="flex h-full w-full flex-col bg-background">
    <!-- Toolbar -->
    <div
        class="flex h-10 items-center justify-between border-b border-border bg-muted/20 px-2 gap-2"
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
            <!-- Schema Picker (Database is implicit from global selection) -->
            <DropdownMenu.Root>
                <DropdownMenu.Trigger
                    class="flex items-center gap-1.5 rounded border border-border bg-background px-3 py-1 text-xs font-medium hover:bg-accent hover:text-accent-foreground transition-colors min-w-[100px]"
                    title="Select Schema"
                >
                    <IconSchema class="size-3 text-muted-foreground" />
                    <span class="truncate max-w-[100px]"
                        >{selectedSchema || "public"}</span
                    >
                    <IconChevronDown
                        class="ml-auto size-3 text-muted-foreground"
                    />
                </DropdownMenu.Trigger>
                <DropdownMenu.Content
                    align="end"
                    class="max-h-[300px] overflow-auto"
                >
                    <DropdownMenu.Label>Schemas</DropdownMenu.Label>
                    <DropdownMenu.Separator />
                    <DropdownMenu.RadioGroup
                        value={selectedSchema}
                        onValueChange={(v) => (selectedSchema = v)}
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
