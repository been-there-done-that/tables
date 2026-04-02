<script lang="ts">
    import * as ContextMenu from "$lib/components/ui/context-menu";
    import type { TreeNode } from "./FileTree.svelte";
    import IconTable       from "@tabler/icons-svelte/icons/table";
    import IconCode        from "@tabler/icons-svelte/icons/code";
    import IconCopy        from "@tabler/icons-svelte/icons/copy";
    import IconRefresh     from "@tabler/icons-svelte/icons/refresh";
    import IconTerminal    from "@tabler/icons-svelte/icons/terminal";
    import IconTrash       from "@tabler/icons-svelte/icons/trash";
    import IconCut         from "@tabler/icons-svelte/icons/cut";
    import IconPlayerPlay  from "@tabler/icons-svelte/icons/player-play";
    import IconPlayerStop  from "@tabler/icons-svelte/icons/player-stop";
    import IconBolt        from "@tabler/icons-svelte/icons/bolt";
    import IconList        from "@tabler/icons-svelte/icons/list";

    let {
        node,
        onAction = (_action: string, _node: TreeNode) => {},
    }: {
        node: TreeNode;
        onAction?: (action: string, node: TreeNode) => void;
    } = $props();

    function act(a: string) { onAction(a, node); }

    const t            = $derived(node.type);
    const isTable      = $derived(t === "table");
    const isView       = $derived(t === "view");
    const isMatView    = $derived(t === "materialized_view");
    const isFunction   = $derived(t === "function");
    const isProcedure  = $derived(t === "procedure");
    const isSequence   = $derived(t === "sequence");
    const isIndex      = $derived(t === "index");
    const isTrigger    = $derived(t === "trigger");
    const isConstraint = $derived(t === "constraint");
    const isSchema     = $derived(t === "schema");
    const isColumn     = $derived(t === "column" || t === "primary_key");

    const isDataObject  = $derived(isTable || isView || isMatView);
    const isCodeObject  = $derived(isFunction || isProcedure);
    const hasDefinition = $derived(isDataObject || isCodeObject || isSequence || isIndex || isTrigger || isSchema);
    const isDroppable   = $derived(isTable || isView || isMatView || isFunction || isProcedure || isSequence || isIndex || isTrigger);

    const dropLabel = $derived(
        isTable ? "Drop Table" : isView ? "Drop View" : isMatView ? "Drop Materialized View" :
        isFunction ? "Drop Function" : isProcedure ? "Drop Procedure" :
        isSequence ? "Drop Sequence" : isIndex ? "Drop Index" :
        isTrigger ? "Drop Trigger" : "Drop"
    );
    const ddlLabel = $derived(
        isCodeObject ? "Open Function DDL" : isSequence ? "Open Sequence DDL" :
        isIndex ? "Open Index DDL" : isTrigger ? "Open Trigger DDL" :
        isSchema ? "Open Schema DDL" : "Open DDL"
    );
</script>

<ContextMenu.Content class="w-56">

    <!-- ── DATA ─────────────────────────────────────────────── -->
    {#if isDataObject}
        <ContextMenu.Item onclick={() => act("view_data")}>
            <IconTable class="mr-2 size-3.5 opacity-60" />
            View Data
        </ContextMenu.Item>
        {#if isMatView}
            <ContextMenu.Item onclick={() => act("refresh_matview")}>
                <IconRefresh class="mr-2 size-3.5 opacity-60" />
                Refresh View
            </ContextMenu.Item>
        {/if}
        <ContextMenu.Separator />
    {/if}

    <!-- ── QUERY EDITOR ─────────────────────────────────────── -->
    {#if isDataObject || isCodeObject || isSequence || isSchema}
        <ContextMenu.Item onclick={() => act("query_console")}>
            <IconTerminal class="mr-2 size-3.5 opacity-60" />
            Open in Query Editor
            <ContextMenu.Shortcut>⇧⌘L</ContextMenu.Shortcut>
        </ContextMenu.Item>
        <ContextMenu.Separator />
    {/if}

    <!-- ── DEFINITION ───────────────────────────────────────── -->
    {#if hasDefinition}
        <ContextMenu.Label class="text-[10px] uppercase tracking-wide opacity-40 px-2 py-1">Definition</ContextMenu.Label>
        <ContextMenu.Item onclick={() => act("open_ddl")}>
            <IconCode class="mr-2 size-3.5 opacity-60" />
            {ddlLabel}
        </ContextMenu.Item>
        <ContextMenu.Item onclick={() => act("copy_ddl")}>
            <IconCopy class="mr-2 size-3.5 opacity-60" />
            Copy DDL
        </ContextMenu.Item>
        <ContextMenu.Separator />
    {/if}

    {#if isConstraint}
        <ContextMenu.Item onclick={() => act("copy_ddl")}>
            <IconCopy class="mr-2 size-3.5 opacity-60" />
            Copy DDL
        </ContextMenu.Item>
        <ContextMenu.Separator />
    {/if}

    <!-- ── COPY ─────────────────────────────────────────────── -->
    <ContextMenu.Label class="text-[10px] uppercase tracking-wide opacity-40 px-2 py-1">Copy</ContextMenu.Label>
    <ContextMenu.Item onclick={() => act("copy_name")}>
        <IconCopy class="mr-2 size-3.5 opacity-60" />
        Copy Name
    </ContextMenu.Item>
    {#if isDataObject}
        <ContextMenu.Item onclick={() => act("copy_select")}>
            <IconList class="mr-2 size-3.5 opacity-60" />
            Copy as SELECT *
        </ContextMenu.Item>
        <ContextMenu.Item onclick={() => act("copy_insert_template")}>
            <IconList class="mr-2 size-3.5 opacity-60" />
            Copy INSERT template
        </ContextMenu.Item>
    {/if}
    {#if isColumn}
        <ContextMenu.Item onclick={() => act("copy_column_type")}>
            <IconList class="mr-2 size-3.5 opacity-60" />
            Copy Data Type
        </ContextMenu.Item>
    {/if}

    <!-- ── MAINTENANCE ──────────────────────────────────────── -->
    {#if isTable}
        <ContextMenu.Separator />
        <ContextMenu.Label class="text-[10px] uppercase tracking-wide opacity-40 px-2 py-1">Maintenance</ContextMenu.Label>
        <ContextMenu.Item onclick={() => act("truncate")}>
            <IconCut class="mr-2 size-3.5 opacity-60" />
            Truncate
        </ContextMenu.Item>
        <ContextMenu.Item onclick={() => act("truncate_cascade")}>
            <IconCut class="mr-2 size-3.5 opacity-60" />
            Truncate Cascade
        </ContextMenu.Item>
        <ContextMenu.Item onclick={() => act("vacuum_analyze")}>
            <IconBolt class="mr-2 size-3.5 opacity-60" />
            Vacuum Analyze
        </ContextMenu.Item>
        <ContextMenu.Item onclick={() => act("analyze")}>
            <IconBolt class="mr-2 size-3.5 opacity-60" />
            Analyze
        </ContextMenu.Item>
    {/if}

    {#if isIndex}
        <ContextMenu.Separator />
        <ContextMenu.Label class="text-[10px] uppercase tracking-wide opacity-40 px-2 py-1">Maintenance</ContextMenu.Label>
        <ContextMenu.Item onclick={() => act("reindex")}>
            <IconRefresh class="mr-2 size-3.5 opacity-60" />
            Reindex
        </ContextMenu.Item>
    {/if}

    <!-- ── TRIGGER ───────────────────────────────────────────── -->
    {#if isTrigger}
        <ContextMenu.Separator />
        <ContextMenu.Label class="text-[10px] uppercase tracking-wide opacity-40 px-2 py-1">Actions</ContextMenu.Label>
        <ContextMenu.Item onclick={() => act("enable_trigger")}>
            <IconPlayerPlay class="mr-2 size-3.5 opacity-60" />
            Enable Trigger
        </ContextMenu.Item>
        <ContextMenu.Item onclick={() => act("disable_trigger")}>
            <IconPlayerStop class="mr-2 size-3.5 opacity-60" />
            Disable Trigger
        </ContextMenu.Item>
    {/if}

    <!-- ── SCHEMA ────────────────────────────────────────────── -->
    {#if isSchema}
        <ContextMenu.Separator />
        <ContextMenu.Item onclick={() => act("refresh_schema")}>
            <IconRefresh class="mr-2 size-3.5 opacity-60" />
            Refresh Schema
        </ContextMenu.Item>
    {/if}

    <!-- ── DESTRUCTIVE ───────────────────────────────────────── -->
    {#if isDroppable}
        <ContextMenu.Separator />
        <ContextMenu.Item
            onclick={() => act("drop")}
            class="text-destructive focus:text-destructive focus:bg-destructive/10"
        >
            <IconTrash class="mr-2 size-3.5" />
            {dropLabel}
        </ContextMenu.Item>
    {/if}

</ContextMenu.Content>
