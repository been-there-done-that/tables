<script lang="ts">
    import { onMount } from "svelte";
    import { Grid, WillowDark } from "wx-svelte-grid";
    import type {
        Column,
        DataFetcher,
        OnApplyEdits,
        SortState,
        TableQueryContext,
    } from "./types";

    interface Props {
        columns: Column[];
        dataFetcher: DataFetcher;
        onApplyEdits?: OnApplyEdits;
        class?: string;
        tableName?: string;
        tableSchema?: string;
        onOpenInQueryEditor?: (ctx: TableQueryContext) => void;
        onOpenNewQueryTab?: (ctx: TableQueryContext) => void;
        viewState?: Record<string, any>;
        onViewStateChange?: (state: any) => void;
        isLoading?: boolean;
        limit?: number;
        offset?: number;
    }

    let {
        columns,
        dataFetcher,
        onApplyEdits,
        class: className,
        tableName,
        tableSchema,
        onOpenInQueryEditor,
        onOpenNewQueryTab,
        viewState = $bindable(),
        onViewStateChange,
        isLoading = $bindable(false),
        limit = $bindable(500),
        offset = $bindable(0),
    }: Props = $props();

    // Internal state
    let gridData = $state<any[]>([]);
    let gridColumns = $state<any[]>([]);
    let totalRows = $state(0);
    let loading = $state(false);
    let sortState = $state<SortState[]>([]);
    let gridApi: any = null;

    // Convert our Column type to SVAR Grid column format
    function mapColumnsToSvar(cols: Column[]): any[] {
        return cols.map((col) => ({
            id: col.id,
            header: col.label || col.id,
            width: col.width || 150,
            resize: true,
            sort: col.sortable,
            editor: col.editable ? getEditorType(col) : undefined,
            template: getCellTemplate(col),
        }));
    }

    function getEditorType(col: Column): string | undefined {
        switch (col.type) {
            case "boolean":
                return "checkbox";
            case "int":
            case "float":
                return "text";
            case "date":
            case "datetime":
                return "datepicker";
            case "enum":
                return "select";
            default:
                return "text";
        }
    }

    function getCellTemplate(
        col: Column,
    ): ((value: any) => string) | undefined {
        switch (col.type) {
            case "json":
            case "jsonb":
            case "JSON":
                return (value: any) => {
                    if (value === null || value === undefined) return "";
                    try {
                        const str =
                            typeof value === "string"
                                ? value
                                : JSON.stringify(value);
                        return str.length > 50 ? str.slice(0, 50) + "..." : str;
                    } catch {
                        return String(value);
                    }
                };
            case "boolean":
                return (value: any) => {
                    if (value === null || value === undefined) return "";
                    return value ? "✓" : "✗";
                };
            case "datetime":
                return (value: any) => {
                    if (value === null || value === undefined) return "";
                    try {
                        const d = new Date(value);
                        return d.toLocaleString();
                    } catch {
                        return String(value);
                    }
                };
            default:
                return undefined;
        }
    }

    // Fetch data using the provided dataFetcher
    async function loadData() {
        if (loading) return;
        loading = true;
        isLoading = true;

        try {
            const result = await dataFetcher({
                offset,
                limit,
                sort: sortState,
                filters: {},
            });

            // Add id for SVAR Grid row identification
            gridData = result.rows.map((row, idx) => ({
                ...row,
                id: row._rowId ?? row.id ?? offset + idx,
            }));

            totalRows = result.total;

            // Update columns if provided by fetch
            if (result.columns?.length) {
                gridColumns = mapColumnsToSvar(result.columns);
            }
        } catch (e) {
            console.error("[SvarTable] loadData error:", e);
        } finally {
            loading = false;
            isLoading = false;
        }
    }

    // Sync columns prop to grid columns
    $effect(() => {
        if (columns && columns.length > 0) {
            gridColumns = mapColumnsToSvar(columns);
        }
    });

    // Load on mount only - parent components should call refresh() for reloads
    onMount(() => {
        loadData();
    });

    // Initialize Grid API and register event handlers
    function initGrid(api: any) {
        gridApi = api;

        // Listen for cell updates
        api.on("update-cell", (ev: { id: any; column: string; value: any }) => {
            console.log("[SvarTable] Cell updated:", ev);

            if (onApplyEdits) {
                const row = gridData.find((r) => r.id === ev.id);
                if (row) {
                    onApplyEdits([
                        {
                            rowId: ev.id,
                            originalRow: { ...row },
                            changes: { [ev.column]: ev.value },
                        },
                    ]);
                }
            }
        });

        // Listen for sort changes
        api.on("sort", (ev: { column: string; order: string }) => {
            console.log("[SvarTable] Sort:", ev);

            if (ev.order === "none") {
                sortState = [];
            } else {
                sortState = [
                    {
                        columnId: ev.column,
                        direction: ev.order as "asc" | "desc",
                    },
                ];
            }

            loadData();
        });
    }

    // Export refresh method for parent components
    export function refresh() {
        loadData();
    }

    export function getContainer() {
        return null;
    }
</script>

<div class={className} style="height: 100%; width: 100%;">
    <WillowDark>
        <Grid data={gridData} columns={gridColumns} init={initGrid} />
    </WillowDark>
</div>

<style>
    div {
        display: flex;
        flex-direction: column;
    }
</style>
