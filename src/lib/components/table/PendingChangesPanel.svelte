<script lang="ts">
    import { cn } from "$lib/utils";
    import IconX from "@tabler/icons-svelte/icons/x";
    import IconCode from "@tabler/icons-svelte/icons/code";
    import IconEye from "@tabler/icons-svelte/icons/eye";
    import IconCopy from "@tabler/icons-svelte/icons/copy";
    import IconArrowBackUp from "@tabler/icons-svelte/icons/arrow-back-up";
    import IconTrash from "@tabler/icons-svelte/icons/trash";
    import IconDeviceFloppy from "@tabler/icons-svelte/icons/device-floppy";
    import IconLoader2 from "@tabler/icons-svelte/icons/loader-2";
    import { toast } from "svelte-sonner";
    import type { EditDelta } from "./TableEditManager.svelte";
    import { pendingChangesStore } from "$lib/stores/pendingChanges.svelte";
    import { windowState } from "$lib/stores/window.svelte";
    import { settingsStore } from "$lib/stores/settings.svelte";
    import { NULL_TOKEN, DEFAULT_TOKEN } from "./valueUtils";
    import { useMonacoEditor } from "$lib/monaco/useMonacoEditor";
    import { MONACO_THEME_NAME } from "$lib/monaco/monaco-theme";
    import * as Popover from "$lib/components/ui/popover";
    import type { EditorHandle } from "$lib/monaco/editor-types";

    let activeTab = $state<"visual" | "sql">("visual");

    // Access state from store
    const deltas = $derived(pendingChangesStore.deltas);
    const tableName = $derived(pendingChangesStore.tableName);
    const tableSchema = $derived(pendingChangesStore.tableSchema);
    const columns = $derived(pendingChangesStore.columns);
    const primaryKeyColumns = $derived(pendingChangesStore.primaryKeyColumns);
    const isSaving = $derived(pendingChangesStore.isSaving);

    async function saveChanges() {
        if (!pendingChangesStore.onSaveChanges) {
            toast.error("Save not available");
            return;
        }

        pendingChangesStore.isSaving = true;
        try {
            const result = await pendingChangesStore.onSaveChanges();
            if (result.success) {
                toast.success("Changes saved successfully");
                // Panel stays open to show cleared state
            } else {
                const errMsg = result.errors?.join(", ") || "Unknown error";
                toast.error(`Failed to save: ${errMsg}`);
            }
        } catch (e: any) {
            toast.error(`Save failed: ${e.message || String(e)}`);
        } finally {
            pendingChangesStore.isSaving = false;
        }
    }

    // Group deltas by rowId
    const groupedDeltas = $derived(() => {
        const groups = new Map<any, EditDelta[]>();
        for (const delta of deltas) {
            if (!groups.has(delta.rowId)) {
                groups.set(delta.rowId, []);
            }
            groups.get(delta.rowId)!.push(delta);
        }
        return groups;
    });

    // Determine operation type for each row
    function getOperationType(rowDeltas: EditDelta[]): "U" | "I" | "D" {
        return rowDeltas[0]?.type || "U";
    }

    // Generate SQL statements
    const generatedSql = $derived(() => {
        const statements: string[] = [];

        for (const [rowId, rowDeltas] of groupedDeltas()) {
            const firstColDef = columns.find((c) => c.id === rowDeltas[0]?.columnId);
            const rowTable = (firstColDef as any)?.sourceTable ?? tableName;
            const rowSchema = (firstColDef as any)?.sourceSchema ?? tableSchema;
            const fullTableName = rowSchema ? `"${rowSchema}"."${rowTable}"` : `"${rowTable}"`;

            const type = getOperationType(rowDeltas);
            let sql = "";

            if (type === "I") {
                const cols: string[] = [];
                const vals: string[] = [];
                rowDeltas.forEach((d) => {
                    if (d.newValue === undefined) return;
                    cols.push(`"${d.columnId}"`);
                    vals.push(formatSqlValue(d.newValue));
                });
                if (cols.length > 0) {
                    sql = `INSERT INTO ${fullTableName} (${cols.join(", ")}) VALUES (${vals.join(", ")});`;
                }
            } else if (type === "D") {
                const firstDelta = rowDeltas[0];
                const pkValues = firstDelta?.pkValues;
                let whereClause = "";

                if (pkValues && Object.keys(pkValues).length > 0) {
                    whereClause = Object.entries(pkValues)
                        .map(([pk, val]) => `"${pk}" = ${formatSqlValue(val)}`)
                        .join(" AND ");
                } else if (primaryKeyColumns.length > 0) {
                    whereClause = primaryKeyColumns
                        .map((pk) => `"${pk}" = ${formatSqlValue(rowId)}`)
                        .join(" AND ");
                } else {
                    whereClause = `/* row ${rowId} - no primary key */`;
                }
                sql = `DELETE FROM ${fullTableName} WHERE ${whereClause};`;
            } else {
                // UPDATE
                const setClause = rowDeltas
                    .map(
                        (d) =>
                            `"${d.columnId}" = ${formatSqlValue(d.newValue)}`,
                    )
                    .join(", ");

                let whereClause = "";
                const firstDelta = rowDeltas[0];
                const pkValues = firstDelta?.pkValues;

                if (pkValues && Object.keys(pkValues).length > 0) {
                    whereClause = Object.entries(pkValues)
                        .map(([pk, val]) => `"${pk}" = ${formatSqlValue(val)}`)
                        .join(" AND ");
                } else if (primaryKeyColumns.length > 0) {
                    whereClause = primaryKeyColumns
                        .map((pk) => `"${pk}" = ${formatSqlValue(rowId)}`)
                        .join(" AND ");
                } else {
                    whereClause = `/* row ${rowId} - no primary key */`;
                }
                sql = `UPDATE ${fullTableName}\nSET ${setClause}\nWHERE ${whereClause};`;
            }

            if (sql) statements.push(sql);
        }

        return statements.join("\n\n");
    });

    function formatSqlValue(val: any): string {
        if (val === null || val === undefined || val === NULL_TOKEN)
            return "NULL";
        if (val === DEFAULT_TOKEN) return "DEFAULT";
        if (typeof val === "number") return String(val);
        if (typeof val === "boolean") return val ? "TRUE" : "FALSE";
        if (typeof val === "string") return `'${val.replace(/'/g, "''")}'`;
        return `'${JSON.stringify(val).replace(/'/g, "''")}'`;
    }

    function formatDisplayValue(val: any): string {
        if (val === null || val === NULL_TOKEN) return "NULL";
        if (val === DEFAULT_TOKEN) return "DEFAULT";
        if (val === undefined) return "undefined";
        if (typeof val === "object") return JSON.stringify(val);
        return String(val);
    }

    async function copyToClipboard() {
        try {
            await navigator.clipboard.writeText(generatedSql());
        } catch (e) {
            console.error("Failed to copy", e);
        }
    }

    async function copyRowToClipboard(rowId: any) {
        try {
            const rowDeltas = groupedDeltas().get(rowId);
            if (!rowDeltas || rowDeltas.length === 0) return;

            const type = getOperationType(rowDeltas);
            const firstColDef = columns.find((c) => c.id === rowDeltas[0]?.columnId);
            const rowTable = (firstColDef as any)?.sourceTable ?? tableName;
            const rowSchema = (firstColDef as any)?.sourceSchema ?? tableSchema;
            const fullTableName = rowSchema ? `"${rowSchema}"."${rowTable}"` : `"${rowTable}"`;

            let sql = "";
            if (type === "I") {
                const cols = rowDeltas.map((d) => `"${d.columnId}"`);
                const vals = rowDeltas.map((d) => formatSqlValue(d.newValue));
                sql = `INSERT INTO ${fullTableName} (${cols.join(", ")}) VALUES (${vals.join(", ")});`;
            } else if (type === "D") {
                const pkValues = rowDeltas[0].pkValues;
                let whereClause = "";
                if (pkValues) {
                    whereClause = Object.entries(pkValues)
                        .map(([pk, val]) => `"${pk}" = ${formatSqlValue(val)}`)
                        .join(" AND ");
                } else {
                    whereClause = `/* row ${rowId} */`;
                }
                sql = `DELETE FROM ${fullTableName} WHERE ${whereClause};`;
            } else {
                const setClause = rowDeltas
                    .map(
                        (d) =>
                            `"${d.columnId}" = ${formatSqlValue(d.newValue)}`,
                    )
                    .join(", ");
                const pkValues = rowDeltas[0].pkValues;
                let whereClause = "";
                if (pkValues) {
                    whereClause = Object.entries(pkValues)
                        .map(([pk, val]) => `"${pk}" = ${formatSqlValue(val)}`)
                        .join(" AND ");
                } else {
                    whereClause = `/* row ${rowId} */`;
                }
                sql = `UPDATE ${fullTableName}\nSET ${setClause}\nWHERE ${whereClause};`;
            }

            await navigator.clipboard.writeText(sql);
        } catch (e) {
            console.error("Failed to copy row", e);
        }
    }

    function revertRow(rowId: any) {
        pendingChangesStore.onRevertRow?.(rowId);
    }

    function revertAll() {
        pendingChangesStore.onRevertAll?.();
    }

    function getColumnLabel(colId: string): string {
        const col = columns.find((c) => c.id === colId);
        return col?.label || colId;
    }

    function formatPkDescription(pkValues?: Record<string, any>): string {
        if (!pkValues || Object.keys(pkValues).length === 0) return "";
        return Object.entries(pkValues)
            .map(([k, v]) => `${k}=${v}`)
            .join(", ");
    }

    // Monaco Editor Integration
    // svelte-ignore non_reactive_update
    let editorContainer: HTMLElement;
    let editorHandle = $state<EditorHandle | null>(null);

    $effect(() => {
        if (activeTab === "sql" && editorContainer) {
            useMonacoEditor(
                {
                    contextId: `pending-changes-sql-${windowState.label}`,
                    windowId: windowState.label,
                    kind: "sql",
                    modelUri: `file:///pending-changes-${windowState.label}.sql`,
                    container: () => editorContainer,
                    options: {
                        theme: MONACO_THEME_NAME,
                        minimap: { enabled: false },
                        automaticLayout: true,
                        readOnly: true,
                        renderLineHighlight: "none",
                        lineNumbers: "off", // Cleaner look for preview
                        padding: { top: 10, bottom: 10 },
                        fontSize: settingsStore.editorFontSize,
                        fontFamily: settingsStore.editorFontFamily,
                    },
                },
                (handle) => {
                    editorHandle = handle;
                    handle.editor.setValue(generatedSql());
                },
            );
        }
    });

    // Reactive font settings
    $effect(() => {
        if (editorHandle?.editor) {
            editorHandle.editor.updateOptions({
                fontFamily: settingsStore.editorFontFamily,
                fontSize: settingsStore.editorFontSize,
            });
        }
    });

    // Update editor content when SQL changes
    $effect(() => {
        if (editorHandle && activeTab === "sql") {
            const currentVal = editorHandle.editor.getValue();
            const newVal = generatedSql();
            if (currentVal !== newVal) {
                editorHandle.editor.setValue(newVal);
            }
        }
    });
</script>

<div class="flex h-full w-full flex-col bg-background">
    <!-- Header & Tabs -->
    <div
        class="h-8 flex items-center justify-between px-2 border-b border-border bg-muted/30"
    >
        <div class="flex items-center gap-0.5 h-full">
            <button
                type="button"
                class={cn(
                    "h-6 px-3 rounded text-[10px] font-bold transition-all flex items-center gap-1.5",
                    activeTab === "visual"
                        ? "bg-accent/15 text-accent"
                        : "text-muted-foreground hover:text-foreground hover:bg-muted/30",
                )}
                onclick={() => (activeTab = "visual")}
            >
                <IconEye class="size-3.5" />
                Visual
            </button>
            <button
                type="button"
                class={cn(
                    "h-6 px-3 rounded text-[10px] font-bold transition-all flex items-center gap-1.5",
                    activeTab === "sql"
                        ? "bg-accent/15 text-accent"
                        : "text-muted-foreground hover:text-foreground hover:bg-muted/30",
                )}
                onclick={() => (activeTab = "sql")}
            >
                <IconCode class="size-3.5" />
                SQL
            </button>
        </div>

        <div class="flex items-center gap-1">
            {#if deltas.length > 0}
                <button
                    type="button"
                    class="h-6 w-6 flex items-center justify-center hover:bg-accent rounded text-muted-foreground transition-colors"
                    onclick={copyToClipboard}
                    title="Copy all SQL"
                >
                    <IconCopy class="size-3.5" />
                </button>
                <div class="w-px h-3 bg-border mx-0.5"></div>
                <Popover.Root>
                    <Popover.Trigger
                        class="h-6 px-2 flex items-center gap-1.5 hover:bg-red-500/10 text-red-500/80 hover:text-red-500 rounded text-[10px] font-bold transition-colors"
                        title="Discard all changes"
                    >
                        <IconTrash class="size-3" />
                        Discard All
                    </Popover.Trigger>
                    <Popover.Content
                        class="w-48 p-3 bg-(--theme-bg-secondary) border border-(--theme-border-default) shadow-xl rounded-lg anim-pop"
                        align="end"
                        sideOffset={10}
                    >
                        <div class="space-y-3">
                            <p
                                class="text-[11px] font-medium text-foreground/90 leading-relaxed"
                            >
                                Are you sure you want to discard all pending
                                changes?
                            </p>
                            <div class="flex justify-end gap-2">
                                <Popover.Close
                                    class="h-6 px-2 rounded text-[10px] font-bold hover:bg-muted transition-colors text-muted-foreground"
                                >
                                    Cancel
                                </Popover.Close>
                                <button
                                    type="button"
                                    class="h-6 px-2 rounded text-[10px] font-bold bg-red-500 text-white hover:bg-red-600 transition-colors"
                                    onclick={() => {
                                        revertAll();
                                    }}
                                >
                                    Discard All
                                </button>
                            </div>
                        </div>
                    </Popover.Content>
                </Popover.Root>
                <div class="w-px h-3 bg-border mx-0.5"></div>
                <button
                    type="button"
                    class="h-6 px-3 flex items-center gap-1.5 bg-green-600 hover:bg-green-700 text-white rounded text-[10px] font-bold transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                    onclick={saveChanges}
                    disabled={isSaving}
                    title="Save all changes to database"
                >
                    {#if isSaving}
                        <IconLoader2 class="size-3 animate-spin" />
                        Saving...
                    {:else}
                        <IconDeviceFloppy class="size-3" />
                        Save
                    {/if}
                </button>
                <div class="w-px h-3 bg-border mx-0.5"></div>
            {/if}
            <button
                type="button"
                class="h-6 w-6 flex items-center justify-center hover:bg-accent rounded text-muted-foreground transition-colors"
                onclick={() => windowState.closeRightPanel()}
            >
                <IconX class="size-4" />
            </button>
        </div>
    </div>

    <!-- Content -->
    <div class="flex-1 overflow-auto bg-muted/5">
        {#if activeTab === "visual"}
            <div class="p-3 space-y-4">
                {#each [...groupedDeltas()] as [rowId, rowDeltas]}
                    {@const opType = getOperationType(rowDeltas)}
                    <div
                        class="group rounded-xl border border-border bg-surface shadow-sm overflow-hidden transition-all hover:border-border/80 hover:shadow-md"
                    >
                        <!-- Row Header -->
                        <div
                            class="flex items-center justify-between px-3 py-2.5 border-b border-border/40 bg-muted/10"
                        >
                            <div class="flex items-center gap-2.5 min-w-0">
                                <span
                                    class={cn(
                                        "px-1.5 py-0.5 rounded text-[9px] font-black uppercase tracking-tighter",
                                        opType === "U" &&
                                            "bg-amber-500/15 text-amber-600 border border-amber-500/20",
                                        opType === "I" &&
                                            "bg-green-500/15 text-green-600 border border-green-500/20",
                                        opType === "D" &&
                                            "bg-red-500/15 text-red-600 border border-red-500/20",
                                    )}
                                >
                                    {opType === "U"
                                        ? "Update"
                                        : opType === "I"
                                          ? "Insert"
                                          : "Delete"}
                                </span>
                                <div class="flex flex-col min-w-0">
                                    <span
                                        class="text-[10px] font-bold text-foreground/90 truncate leading-tight"
                                    >
                                        {formatPkDescription(
                                            rowDeltas[0]?.pkValues,
                                        ) || `Row ${rowId}`}
                                    </span>
                                    <span
                                        class="text-[9px] text-muted-foreground truncate leading-tight"
                                    >
                                        {(() => {
                                            const fd = columns.find((c) => c.id === rowDeltas[0]?.columnId);
                                            const t = (fd as any)?.sourceTable ?? tableName;
                                            const s = (fd as any)?.sourceSchema ?? tableSchema;
                                            return s ? `${s}.${t}` : t;
                                        })()}
                                    </span>
                                </div>
                            </div>

                            <div
                                class="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity"
                            >
                                <button
                                    type="button"
                                    class="size-7 flex items-center justify-center rounded-lg hover:bg-accent text-muted-foreground hover:text-foreground transition-colors"
                                    onclick={() => copyRowToClipboard(rowId)}
                                    title="Copy row SQL"
                                >
                                    <IconCopy class="size-3.5" />
                                </button>
                                <button
                                    type="button"
                                    class="size-7 flex items-center justify-center rounded-lg hover:bg-red-500/10 text-muted-foreground hover:text-red-500 transition-colors"
                                    onclick={() => revertRow(rowId)}
                                    title="Revert this row"
                                >
                                    <IconArrowBackUp class="size-4" />
                                </button>
                            </div>
                        </div>

                        <!-- Column Diffs -->
                        <div class="divide-y divide-border/20">
                            {#if opType === "D"}
                                {@const rowData = rowDeltas[0].oldValue}
                                <div class="px-3 py-2">
                                    <table
                                        class="w-full text-[10px] border-collapse"
                                    >
                                        <tbody>
                                            {#each columns as column}
                                                <tr
                                                    class="border-b border-border/10 last:border-0 hover:bg-red-500/5 transition-colors"
                                                >
                                                    <td
                                                        class="py-1.5 pr-3 font-bold text-muted-foreground/80 truncate max-w-[100px] align-top"
                                                    >
                                                        {column.label ||
                                                            column.id}
                                                    </td>
                                                    <td
                                                        class="py-1.5 pl-1 font-mono text-red-500/90 truncate align-top break-all whitespace-pre-wrap"
                                                        title={formatDisplayValue(
                                                            rowData?.[
                                                                column.id
                                                            ],
                                                        )}
                                                    >
                                                        {formatDisplayValue(
                                                            rowData?.[
                                                                column.id
                                                            ],
                                                        )}
                                                    </td>
                                                </tr>
                                            {/each}
                                        </tbody>
                                    </table>
                                </div>
                            {:else}
                                {#each rowDeltas as delta}
                                    <div class="px-3 py-2.5 space-y-2">
                                        <div
                                            class="text-[9px] uppercase tracking-widest text-muted-foreground/70 font-bold"
                                        >
                                            {getColumnLabel(delta.columnId)}
                                        </div>
                                        <div
                                            class="flex items-center gap-2 text-[11px] font-mono leading-relaxed"
                                        >
                                            {#if opType !== "I"}
                                                <div
                                                    class="flex-1 min-w-0 px-2 py-1 rounded bg-red-500/5 text-red-500/90 border border-red-500/10 truncate"
                                                    title={formatDisplayValue(
                                                        delta.oldValue,
                                                    )}
                                                >
                                                    {formatDisplayValue(
                                                        delta.oldValue,
                                                    )}
                                                </div>
                                                <div
                                                    class="text-muted-foreground/30 font-sans font-bold"
                                                >
                                                    →
                                                </div>
                                            {/if}
                                            <div
                                                class="flex-1 min-w-0 px-2 py-1 rounded bg-green-500/5 text-green-500 border border-green-500/10 truncate font-bold"
                                                title={formatDisplayValue(
                                                    delta.newValue,
                                                )}
                                            >
                                                {formatDisplayValue(
                                                    delta.newValue,
                                                )}
                                            </div>
                                        </div>
                                    </div>
                                {/each}
                            {/if}
                        </div>
                    </div>
                {/each}

                {#if deltas.length === 0}
                    <div
                        class="flex flex-col items-center justify-center py-12 px-6 text-center space-y-3"
                    >
                        <div
                            class="size-12 rounded-full bg-muted/20 flex items-center justify-center text-muted-foreground/30"
                        >
                            <IconEye class="size-6" />
                        </div>
                        <div class="space-y-1">
                            <h3 class="text-xs font-bold text-foreground/80">
                                No pending changes
                            </h3>
                            <p
                                class="text-[10px] text-muted-foreground max-w-[180px]"
                            >
                                Your edits will appear here as a visual diff
                                before you commit them.
                            </p>
                        </div>
                    </div>
                {/if}
            </div>
        {:else}
            <!-- SQL Tab -->
            <div class="flex flex-col h-full">
                <div
                    class="flex-1 relative bg-background border-t border-border/50"
                >
                    <div
                        bind:this={editorContainer}
                        class="absolute inset-0 w-full h-full"
                    ></div>
                </div>
            </div>
        {/if}
    </div>
</div>
