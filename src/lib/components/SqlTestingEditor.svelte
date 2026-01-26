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
    import {
        saveEditorSession,
        loadEditorSession,
        createDebouncedSave,
    } from "$lib/services/editor-persistence";
    import {
        enableQueryHeaders,
        type QueryHeaderController,
    } from "$lib/monaco/query-headers";

    let { id = "playground", context = $bindable({}) } = $props<{
        id?: string;
        context?: any;
    }>();

    let editorContainer: HTMLElement;
    let editorHandle = $state<EditorHandle | null>(null);
    let logs: string[] = $state([]);
    let isLoadingSession = $state(true);

    let headerController: QueryHeaderController | null = null;

    // Debounced save for editor content
    const debouncedSave = createDebouncedSave(2000);

    // Stable context and URI derived from ID
    const stableContextId = $derived(`sql-playground-${id}`);
    const stableModelUri = $derived(
        context?.modelUri || `file:///playground-${id}.sql`,
    );

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
        console.log("[SQL] executeCurrent triggered");
        if (!editorHandle) return;
        const editor = editorHandle.editor;
        const model = editor.getModel();
        if (!model) return;

        let query = "";
        let source = "";
        let startLine: number | undefined;

        // 1. Check for manual selection first
        const selection = editor.getSelection();
        if (selection && !selection.isEmpty()) {
            query = model.getValueInRange(selection);
            source = "manual selection";
            startLine = selection.startLineNumber;
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
                startLine = highlight.range.startLineNumber;
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

            if (startLine && headerController) {
                headerController.updateStatus(startLine, query, {
                    state: "running",
                });
            }

            const startTime = performance.now();

            try {
                const result = await invoke<any>("execute_query", {
                    connectionId: schemaStore.activeConnection.id,
                    database: schemaStore.selectedDatabase,
                    schema: schemaStore.activeSchema || "public",
                    query: query,
                    component: "editor",
                });

                const duration =
                    result.duration_ms ?? performance.now() - startTime;

                if (startLine && headerController) {
                    headerController.updateStatus(startLine, query, {
                        state: "success",
                        duration,
                    });
                }

                console.log("Query Result:", result);
                log("Query completed successfully.");
            } catch (e) {
                if (startLine && headerController) {
                    headerController.updateStatus(startLine, query, {
                        state: "error",
                        errorMessage: String(e),
                    });
                }
                console.error("Query execution failed:", e);
                log(`Query failed: ${e}`);
            }
        } else {
            log("No query to execute");
        }
    }

    // Execute a specific query text (used by inline run buttons)
    // Accept line ranges to track status
    async function executeQueryText(
        queryText: string,
        startLine?: number,
        endLine?: number,
    ) {
        console.log(`[SQL] executeQueryText called for line ${startLine}`, {
            queryText,
        });
        if (!queryText.trim()) {
            log("No query to execute");
            return;
        }

        log(
            `Executing (inline button) in ${schemaStore.selectedDatabase}.${schemaStore.activeSchema}:\n${queryText}`,
        );

        if (!schemaStore.activeConnection) {
            log("No active connection selected.");
            return;
        }

        // Mark as running
        if (startLine && endLine && headerController) {
            headerController.updateStatus(startLine, queryText, {
                state: "running",
            });
        }

        const startTime = performance.now();

        try {
            const result = await invoke<any>("execute_query", {
                connectionId: schemaStore.activeConnection.id,
                database: schemaStore.selectedDatabase,
                schema: schemaStore.activeSchema || "public",
                query: queryText,
                component: "editor",
            });

            // Use backend duration if available (more accurate), else fallback to frontend measure
            const duration =
                result.duration_ms ?? performance.now() - startTime;

            // Mark success
            if (startLine && endLine && headerController) {
                headerController.updateStatus(startLine, queryText, {
                    state: "success",
                    duration,
                });
            }

            console.log("Query Result:", result);
            log("Query completed successfully.");
        } catch (e) {
            // Mark error
            if (startLine && endLine && headerController) {
                headerController.updateStatus(startLine, queryText, {
                    state: "error",
                    errorMessage: String(e),
                });
            }

            console.error("Query execution failed:", e);
            log(`Query failed: ${e}`);
        }
    }

    function handleExplain(raw: boolean = false) {
        log(`Explain ${raw ? "(Raw)" : ""} functionality not implemented yet.`);
    }

    useMonacoEditor(
        {
            contextId: stableContextId,
            windowId: windowState.label,
            kind: "sql",
            modelUri: stableModelUri,
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
            console.log("[EDITOR-DEBUG] ========== CALLBACK START ==========");
            console.log("[EDITOR-DEBUG] Editor callback received for:", {
                id,
                stableModelUri,
                stableContextId,
                editorId: handle.editorId,
                contentOnCallback: handle.editor.getValue().substring(0, 100),
            });

            editorHandle = handle;
            log("Editor initialized");

            // Add Command+Enter / Ctrl+Enter shortcut
            handle.editor.addCommand(
                monaco.KeyMod.CtrlCmd | monaco.KeyCode.Enter,
                () => {
                    executeCurrent();
                },
            );

            // Load session from backend
            console.log("[EDITOR-DEBUG] Loading session for id:", id);
            loadEditorSession(id)
                .then((session) => {
                    isLoadingSession = false;
                    console.log("[EDITOR-DEBUG] Session loaded:", {
                        hasSession: !!session,
                        sessionContent:
                            session?.content?.substring(0, 100) || "null",
                        currentEditorContent: handle.editor
                            .getValue()
                            .substring(0, 100),
                    });

                    if (session) {
                        log(
                            `Restored session from ${new Date(session.lastOpenedAt * 1000).toLocaleString()}`,
                        );
                        console.log(
                            "[EDITOR-DEBUG] Setting content from session",
                        );
                        handle.editor.setValue(session.content);
                        handle.editor.setPosition({
                            lineNumber: session.cursorLine,
                            column: session.cursorColumn,
                        });
                        if (context) {
                            context.content = session.content;
                        }
                        console.log(
                            "[EDITOR-DEBUG] After setValue from session:",
                            handle.editor.getValue().substring(0, 100),
                        );
                    } else {
                        // No saved session - ALWAYS set content to clear stale pooled content
                        log(
                            "No saved session, initializing with default content",
                        );
                        if (context?.content) {
                            console.log(
                                "[EDITOR-DEBUG] Setting content from context:",
                                context.content.substring(0, 100),
                            );
                            handle.editor.setValue(context.content);
                        } else {
                            const defaultContent = `-- SQL Auto-Completion Playground\n-- Context: ${schemaStore.selectedDatabase}.${schemaStore.activeSchema}\n-- Type 'SELECT' or table names from your active connection\n\nSELECT * FROM `;
                            console.log(
                                "[EDITOR-DEBUG] Setting default content",
                            );
                            handle.editor.setValue(defaultContent);
                            handle.editor.setPosition({
                                lineNumber: 4,
                                column: 15,
                            });
                        }
                        console.log(
                            "[EDITOR-DEBUG] After setValue default:",
                            handle.editor.getValue().substring(0, 100),
                        );
                    }
                    handle.editor.focus();
                    console.log(
                        "[EDITOR-DEBUG] ========== CALLBACK COMPLETE ==========",
                    );
                })
                .catch((e) => {
                    isLoadingSession = false;
                    console.error("[EDITOR-DEBUG] Failed to load session:", e);
                    handle.editor.focus();
                });

            // Store disposables for cleanup - CRITICAL to prevent event listener leaks!
            // Monaco subscriptions return IDisposable objects that MUST be disposed
            // when the component unmounts, otherwise they accumulate across pool reuse.
            const contentChangeDisposable =
                handle.editor.onDidChangeModelContent(() => {
                    const val = handle.editor.getValue();
                    const capturedId = id; // Capture current id value NOW
                    if (context) {
                        context.content = val;
                    }
                    // Trigger frontend state save
                    windowState.requestSave();
                    // Debounced backend save
                    const pos = handle.editor.getPosition();
                    console.log(
                        "[SAVE-DEBUG] Scheduling save for id:",
                        capturedId,
                        "content preview:",
                        val.substring(0, 50),
                    );
                    debouncedSave.save(() => {
                        console.log(
                            "[SAVE-DEBUG] Executing save for id:",
                            capturedId,
                        );
                        return saveEditorSession(
                            capturedId,
                            windowState.label,
                            val,
                            pos?.lineNumber ?? 1,
                            pos?.column ?? 1,
                            schemaStore.activeConnection?.id,
                            schemaStore.activeSchema,
                        );
                    });
                });

            // Also save cursor position changes (debounced)
            const cursorChangeDisposable =
                handle.editor.onDidChangeCursorPosition(() => {
                    const pos = handle.editor.getPosition();
                    const val = handle.editor.getValue();
                    const capturedId = id;
                    debouncedSave.save(() => {
                        return saveEditorSession(
                            capturedId,
                            windowState.label,
                            val,
                            pos?.lineNumber ?? 1,
                            pos?.column ?? 1,
                            schemaStore.activeConnection?.id,
                            schemaStore.activeSchema,
                        );
                    });
                });

            // Store disposables for cleanup on unmount
            editorDisposables = [
                contentChangeDisposable,
                cursorChangeDisposable,
            ];

            // Enable Rich Headers (ViewZones above queries)
            const executeQuery = (
                queryText: string,
                startLine: number,
                endLine: number,
            ) => {
                executeQueryText(queryText, startLine, endLine);
            };

            const stopQuery = (startLine: number, endLine: number) => {
                log("Stop functionality not fully implemented yet");
            };

            headerController = enableQueryHeaders(
                handle.editor,
                executeQuery,
                stopQuery,
            );
        },
    );

    // Track disposables for cleanup
    let editorDisposables: { dispose: () => void }[] = [];

    // Flush pending saves and dispose event listeners on destroy
    onDestroy(() => {
        console.log(
            "[EDITOR-DEBUG] Component destroying, disposing",
            editorDisposables.length,
            "listeners",
        );
        debouncedSave.flush();
        // CRITICAL: Dispose all Monaco event subscriptions to prevent leaks
        editorDisposables.forEach((d) => d.dispose());
        editorDisposables = [];
        // Dispose headers
        headerController?.dispose();
        headerController = null;
    });
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
