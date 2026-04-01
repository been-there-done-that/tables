<script lang="ts">
    import { Dialog } from 'bits-ui';
    import type { MetaTable } from '$lib/commands/types';
    import IconX from '@tabler/icons-svelte/icons/x';
    import IconSearch from '@tabler/icons-svelte/icons/search';
    import IconVectorTriangle from '@tabler/icons-svelte/icons/vector-triangle';
    import IconAlertTriangle from '@tabler/icons-svelte/icons/alert-triangle';
    import IconChevronRight from '@tabler/icons-svelte/icons/chevron-right';
    import IconChevronDown from '@tabler/icons-svelte/icons/chevron-down';

    interface Props {
        open: boolean;
        tables: MetaTable[];
        onConfirm: (selected: MetaTable[]) => void;
        onCancel: () => void;
    }
    let { open = $bindable(), tables, onConfirm, onCancel }: Props = $props();

    let search = $state('');
    let selected = $state<Set<string>>(new Set());
    let collapsedSchemas = $state<Set<string>>(new Set());

    // Key = "schema.table_name"
    function tableKey(t: MetaTable) { return `${t.schema}.${t.table_name}`; }

    // FK count per table (outgoing only)
    const fkCount = $derived.by(() => {
        const counts = new Map<string, number>();
        for (const t of tables) {
            const key = tableKey(t);
            counts.set(key, (counts.get(key) ?? 0) + t.foreign_keys.length);
        }
        return counts;
    });

    const filtered = $derived(
        tables.filter(t =>
            t.table_name.toLowerCase().includes(search.toLowerCase()) ||
            t.schema.toLowerCase().includes(search.toLowerCase())
        )
    );

    // Group by schema
    const grouped = $derived.by(() => {
        const map = new Map<string, MetaTable[]>();
        for (const t of filtered) {
            const list = map.get(t.schema) ?? [];
            list.push(t);
            map.set(t.schema, list);
        }
        return map;
    });

    const selectedTables = $derived(tables.filter(t => selected.has(tableKey(t))));

    function toggle(t: MetaTable) {
        const key = tableKey(t);
        const next = new Set(selected);
        if (next.has(key)) next.delete(key);
        else next.add(key);
        selected = next;
    }

    function toggleSchema(schema: string) {
        const next = new Set(collapsedSchemas);
        if (next.has(schema)) next.delete(schema);
        else next.add(schema);
        collapsedSchemas = next;
    }

    // Schema checkbox state: 'all' | 'none' | 'indeterminate'
    function schemaCheckState(schema: string): 'all' | 'none' | 'indeterminate' {
        const schemaTables = grouped.get(schema) ?? [];
        const selectedCount = schemaTables.filter(t => selected.has(tableKey(t))).length;
        if (selectedCount === 0) return 'none';
        if (selectedCount === schemaTables.length) return 'all';
        return 'indeterminate';
    }

    function toggleSchemaSelection(schema: string) {
        const schemaTables = grouped.get(schema) ?? [];
        const state = schemaCheckState(schema);
        const next = new Set(selected);
        if (state === 'all') {
            // deselect all in schema
            for (const t of schemaTables) next.delete(tableKey(t));
        } else {
            // select all in schema
            for (const t of schemaTables) next.add(tableKey(t));
        }
        selected = next;
    }

    function selectAll() {
        selected = new Set(filtered.map(tableKey));
    }

    function clearAll() {
        selected = new Set();
    }

    function addRelated() {
        const allKeys = new Set(tables.map(tableKey));
        const next = new Set(selected);
        let changed = true;
        while (changed) {
            changed = false;
            for (const t of tables) {
                const key = tableKey(t);
                if (!next.has(key)) continue;
                for (const fk of t.foreign_keys) {
                    const refSchema = fk.ref_schema ?? fk.schema;
                    const refKey = `${refSchema}.${fk.ref_table}`;
                    if (allKeys.has(refKey) && !next.has(refKey)) {
                        next.add(refKey);
                        changed = true;
                    }
                }
            }
            for (const t of tables) {
                const key = tableKey(t);
                if (next.has(key)) continue;
                for (const fk of t.foreign_keys) {
                    const refSchema = fk.ref_schema ?? fk.schema;
                    const refKey = `${refSchema}.${fk.ref_table}`;
                    if (next.has(refKey) && !next.has(key)) {
                        next.add(key);
                        changed = true;
                    }
                }
            }
        }
        selected = next;
    }

    function confirm() {
        onConfirm(selectedTables);
        open = false;
    }

    const LARGE_THRESHOLD = 50;
</script>

<Dialog.Root bind:open>
    <Dialog.Portal>
        <Dialog.Overlay class="fixed inset-0 z-50 bg-black/60" />
        <Dialog.Content
            class="fixed left-1/2 top-1/2 z-50 -translate-x-1/2 -translate-y-1/2 w-[520px] max-h-[80vh] flex flex-col rounded-lg border border-border bg-background shadow-xl"
        >
            <!-- Header -->
            <div class="flex items-center gap-2 border-b border-border px-4 py-3">
                <IconVectorTriangle class="h-4 w-4 text-muted-foreground shrink-0" />
                <h2 class="font-semibold text-sm">Select tables for ERD</h2>
                <button
                    class="ml-auto p-1 rounded hover:bg-accent text-muted-foreground"
                    onclick={() => { open = false; onCancel(); }}
                >
                    <IconX class="h-4 w-4" />
                </button>
            </div>

            <!-- Search -->
            <div class="px-3 pt-3 pb-2">
                <div class="flex items-center gap-2 rounded-md border border-border bg-muted/30 px-2 py-1.5">
                    <IconSearch class="h-3.5 w-3.5 text-muted-foreground shrink-0" />
                    <input
                        bind:value={search}
                        placeholder="Search tables…"
                        class="flex-1 bg-transparent text-sm outline-none placeholder:text-muted-foreground"
                    />
                </div>
            </div>

            <!-- Actions row -->
            <div class="flex items-center gap-2 px-3 pb-2 text-xs">
                <button class="text-muted-foreground hover:text-foreground underline-offset-2 hover:underline" onclick={selectAll}>
                    Select all ({filtered.length})
                </button>
                <span class="text-muted-foreground/50">·</span>
                <button class="text-muted-foreground hover:text-foreground underline-offset-2 hover:underline" onclick={clearAll}>
                    Clear
                </button>
                {#if selected.size > 0}
                    <span class="text-muted-foreground/50">·</span>
                    <button class="text-blue-400 hover:text-blue-300 underline-offset-2 hover:underline" onclick={addRelated}>
                        + Add related tables
                    </button>
                {/if}
                <span class="ml-auto text-muted-foreground">{selected.size} selected</span>
            </div>

            <!-- Tree list -->
            <div class="flex-1 overflow-y-auto px-2 pb-2">
                {#each [...grouped.entries()] as [schema, schemaTables]}
                    {@const checkState = schemaCheckState(schema)}
                    {@const isCollapsed = collapsedSchemas.has(schema)}

                    <!-- Schema row -->
                    <div class="flex items-center gap-1 rounded-md px-1 py-1 hover:bg-muted/30 group">
                        <!-- Chevron toggle -->
                        <button
                            class="flex h-5 w-5 shrink-0 items-center justify-center rounded text-muted-foreground hover:text-foreground"
                            onclick={() => toggleSchema(schema)}
                            aria-label={isCollapsed ? 'Expand' : 'Collapse'}
                        >
                            {#if isCollapsed}
                                <IconChevronRight class="h-3.5 w-3.5" />
                            {:else}
                                <IconChevronDown class="h-3.5 w-3.5" />
                            {/if}
                        </button>

                        <!-- Schema checkbox -->
                        <button
                            class="flex h-4 w-4 shrink-0 items-center justify-center rounded border {checkState === 'none' ? 'border-border' : 'border-primary'} {checkState === 'all' ? 'bg-primary' : checkState === 'indeterminate' ? 'bg-primary/40' : ''}"
                            onclick={() => toggleSchemaSelection(schema)}
                            aria-label="Select all in {schema}"
                        >
                            {#if checkState === 'all'}
                                <span class="text-primary-foreground text-[10px] font-bold leading-none">✓</span>
                            {:else if checkState === 'indeterminate'}
                                <span class="text-primary-foreground text-[10px] font-bold leading-none">−</span>
                            {/if}
                        </button>

                        <!-- Schema name + count -->
                        <button
                            class="flex flex-1 items-center gap-2 text-left"
                            onclick={() => toggleSchema(schema)}
                        >
                            <span class="text-xs font-semibold text-foreground uppercase tracking-wide">{schema}</span>
                            <span class="text-xs text-muted-foreground bg-muted rounded px-1.5 py-0.5">{schemaTables.length}</span>
                        </button>
                    </div>

                    <!-- Tables (children) -->
                    {#if !isCollapsed}
                        {#each schemaTables as t (tableKey(t))}
                            {@const key = tableKey(t)}
                            {@const isSelected = selected.has(key)}
                            {@const fks = fkCount.get(key) ?? 0}
                            <button
                                class="w-full flex items-center gap-2 rounded-md pl-8 pr-2 py-1.5 text-sm hover:bg-muted/50 text-left"
                                class:bg-muted={isSelected}
                                onclick={() => toggle(t)}
                            >
                                <!-- Checkbox -->
                                <span class="flex h-4 w-4 shrink-0 items-center justify-center rounded border border-border"
                                    class:bg-primary={isSelected}
                                    class:border-primary={isSelected}
                                >
                                    {#if isSelected}
                                        <span class="text-primary-foreground text-[10px] font-bold leading-none">✓</span>
                                    {/if}
                                </span>
                                <span class="flex-1 truncate">{t.table_name}</span>
                                {#if fks > 0}
                                    <span class="text-xs text-muted-foreground bg-muted rounded px-1">{fks} FK</span>
                                {/if}
                            </button>
                        {/each}
                    {/if}
                {/each}
                {#if filtered.length === 0}
                    <div class="py-8 text-center text-sm text-muted-foreground">No tables match "{search}"</div>
                {/if}
            </div>

            <!-- Footer -->
            <div class="flex flex-col gap-2 border-t border-border px-4 py-3">
                {#if selected.size > LARGE_THRESHOLD}
                    <div class="flex items-center gap-2 rounded-md border border-amber-500/30 bg-amber-500/10 px-3 py-2 text-xs text-amber-600 dark:text-amber-400">
                        <IconAlertTriangle class="h-3.5 w-3.5 shrink-0" />
                        <span>{selected.size} tables selected — large schemas take longer to load.</span>
                    </div>
                {/if}
                <div class="flex items-center justify-end gap-2">
                    <button
                        class="rounded-md border border-border px-3 py-1.5 text-sm hover:bg-muted"
                        onclick={() => { open = false; onCancel(); }}
                    >
                        Cancel
                    </button>
                    <button
                        class="rounded-md bg-primary px-3 py-1.5 text-sm text-primary-foreground hover:bg-primary/90 disabled:opacity-50"
                        disabled={selected.size === 0}
                        onclick={confirm}
                    >
                        {selected.size > LARGE_THRESHOLD ? 'Open anyway' : 'Open ERD'} ({selected.size} tables)
                    </button>
                </div>
            </div>
        </Dialog.Content>
    </Dialog.Portal>
</Dialog.Root>
