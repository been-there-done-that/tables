<script lang="ts">
    import Table from "$lib/components/table/Table.svelte";
    import TableToolbar from "$lib/components/table/TableToolbar.svelte";
    import type { ViewState } from "$lib/stores/session.svelte";
    import IconSql from "@tabler/icons-svelte/icons/sql";
    import * as Popover from "$lib/components/ui/popover";
    import { Button } from "$lib/components/ui/button";

    let { view }: { view: ViewState } = $props();

    const results = $derived(view.data?.results);
    const controller = $derived(view.data?.controller);
    const primaryKeyColumns = $derived(
        results?.columns
            ?.filter((c: any) => c.isPrimaryKey)
            .map((c: any) => c.id) || [],
    );

    let tableRef: any = $state(null);

    // Bind table methods to the shared controller so SqlTestingEditor can reach them
    $effect(() => {
        if (controller && tableRef) {
            controller.refreshTable = () => tableRef.refresh();
            controller.getEditDeltas = () => tableRef.getEditDeltas?.() ?? [];
            controller.revertRow = (rid: any) => tableRef.revertRow?.(rid);
            controller.revertAll = () => tableRef.revertAll?.();
        }
    });
</script>

{#if results && results.visible}
    <div class="h-full flex flex-col bg-background">
        <TableToolbar
            bind:tableRef
            columns={results.columns}
            currentOffset={results.offset}
            totalRows={results.total}
            pageSize={results.pageSize}
            whereClause={results.whereClause}
            orderByClause={results.orderByClause}
            onExecute={controller.execute}
            onRefresh={controller.refresh}
            onPageChange={controller.pageChange}
            onPageSizeChange={controller.pageSizeChange}
            onExport={controller.export}
            onShowDdl={controller.showDdl}
            onWhereChange={controller.whereChange}
            onOrderByChange={controller.orderByChange}
            onCancel={controller.cancel}
            onCountUpdate={controller.countUpdate}
            currentBatchSize={results.currentBatchSize}
            isExactTotal={results.isExactTotal}
            isCountLoading={results.isCountLoading}
            isLoading={results.loading}
            executionTime={results.executionTime}
            pendingChangesCount={results.pendingDeltas?.length || 0}
            onShowChanges={controller.showChanges}
            onAddRow={controller.addRow}
            onSaveChanges={controller.saveChanges}
            isSaving={results.isSaving}
            hideFilters={true}
            hideExecute={true}
            hidePagination={true}
            fetchedAt={results.fetchedAt}
        >
            {#snippet extraActions()}
                <Popover.Root>
                    <Popover.Trigger>
                        <Button
                            variant="ghost"
                            size="sm"
                            class="gap-2 h-7 px-2"
                        >
                            <IconSql class="size-4" />
                            <span class="text-xs">SQL</span>
                        </Button>
                    </Popover.Trigger>
                    <Popover.Content class="w-[500px] p-0 overflow-hidden">
                        <div class="p-3 border-b border-border bg-muted/30">
                            <h4
                                class="text-xs font-semibold uppercase tracking-wider text-muted-foreground"
                            >
                                Executed Query
                            </h4>
                        </div>
                        <div
                            class="p-4 bg-background max-h-[300px] overflow-auto"
                        >
                            <pre
                                class="text-xs font-mono whitespace-pre-wrap leading-relaxed">{results.executedQueryText}</pre>
                        </div>
                    </Popover.Content>
                </Popover.Root>
            {/snippet}
        </TableToolbar>

        <div class="flex-1 relative overflow-hidden">
            <Table
                bind:this={tableRef}
                columns={results.columns}
                dataFetcher={controller.dataFetcher}
                isLoading={results.loading}
                onEditChange={controller.editChange}
                onApplyEdits={controller.applyEdits}
                {primaryKeyColumns}
                viewState={{ rows: results.rows, totalRows: results.total, tableColumns: results.columns }}
            />
        </div>
    </div>
{:else if results && results.loading}
    <div class="h-full flex items-center justify-center bg-background/50">
        <div class="flex flex-col items-center gap-2">
            <div
                class="size-6 border-2 border-primary border-t-transparent animate-spin rounded-full"
            ></div>
            <p class="text-xs text-muted-foreground">Executing query...</p>
        </div>
    </div>
{:else}
    <div
        class="h-full flex items-center justify-center bg-muted/5 text-muted-foreground italic text-sm"
    >
        No results to display. Execute a query to see data here.
    </div>
{/if}
