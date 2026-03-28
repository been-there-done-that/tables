<script lang="ts">
    import * as ContextMenu from "$lib/components/ui/context-menu";
    import IconFileDatabase from "@tabler/icons-svelte/icons/file-database";
    import IconCopy from "@tabler/icons-svelte/icons/copy";
    import IconCode from "@tabler/icons-svelte/icons/code";
    import IconTable from "@tabler/icons-svelte/icons/table";
    import IconRefresh from "@tabler/icons-svelte/icons/refresh";
    import type { TreeNode } from "./FileTree.svelte";

    let {
        node,
        onAction = (action: string, node: TreeNode) => {},
    }: {
        node: TreeNode;
        onAction?: (action: string, node: TreeNode) => void;
    } = $props();

    function act(action: string) {
        onAction(action, node);
    }

    const isTable = $derived(node.type === "table");
    const isView = $derived(node.type === "view");
    const isMatView = $derived(node.type === "materialized_view");
    const isFunction = $derived(node.type === "function" || node.type === "procedure");
    const isSequence = $derived(node.type === "sequence");
    const isIndex = $derived(node.type === "index");
    const isTrigger = $derived(node.type === "trigger");
    const isConstraint = $derived(node.type === "constraint");
    const isSchema = $derived(node.type === "schema");
</script>

<ContextMenu.Content class="w-52">
    <!-- Always available -->
    <ContextMenu.Item onclick={() => act("copy_name")}>
        <IconCopy class="mr-2 size-4 opacity-60" />
        <span>Copy Name</span>
    </ContextMenu.Item>

    {#if isTable || isView || isMatView}
        <ContextMenu.Item onclick={() => act("view_data")}>
            <IconTable class="mr-2 size-4 opacity-60" />
            <span>View Data</span>
        </ContextMenu.Item>
        <ContextMenu.Item onclick={() => act("copy_select")}>
            <IconCopy class="mr-2 size-4 opacity-60" />
            <span>Copy as SELECT *</span>
        </ContextMenu.Item>
    {/if}

    {#if isTable}
        <ContextMenu.Item onclick={() => act("open_ddl")}>
            <IconCode class="mr-2 size-4 opacity-60" />
            <span>Open DDL</span>
        </ContextMenu.Item>
    {/if}

    {#if isView || isMatView}
        <ContextMenu.Item onclick={() => act("open_definition")}>
            <IconCode class="mr-2 size-4 opacity-60" />
            <span>Open Definition</span>
        </ContextMenu.Item>
    {/if}

    {#if isFunction}
        <ContextMenu.Item onclick={() => act("open_definition")}>
            <IconCode class="mr-2 size-4 opacity-60" />
            <span>Open Definition</span>
        </ContextMenu.Item>
        <ContextMenu.Item onclick={() => act("copy_function_ddl")}>
            <IconCopy class="mr-2 size-4 opacity-60" />
            <span>{node.type === "procedure" ? "Copy as CREATE PROCEDURE" : "Copy as CREATE FUNCTION"}</span>
        </ContextMenu.Item>
    {/if}

    {#if isSequence}
        <ContextMenu.Item onclick={() => act("open_ddl")}>
            <IconCode class="mr-2 size-4 opacity-60" />
            <span>Open DDL</span>
        </ContextMenu.Item>
        <ContextMenu.Item onclick={() => act("copy_sequence_ddl")}>
            <IconCopy class="mr-2 size-4 opacity-60" />
            <span>Copy as CREATE SEQUENCE</span>
        </ContextMenu.Item>
    {/if}

    {#if isIndex}
        <ContextMenu.Item onclick={() => act("open_definition")}>
            <IconCode class="mr-2 size-4 opacity-60" />
            <span>Open Definition</span>
        </ContextMenu.Item>
        <ContextMenu.Item onclick={() => act("copy_index_ddl")}>
            <IconCopy class="mr-2 size-4 opacity-60" />
            <span>Copy DDL</span>
        </ContextMenu.Item>
    {/if}

    {#if isTrigger}
        <ContextMenu.Item onclick={() => act("open_definition")}>
            <IconCode class="mr-2 size-4 opacity-60" />
            <span>Open Definition</span>
        </ContextMenu.Item>
        <ContextMenu.Item onclick={() => act("copy_trigger_ddl")}>
            <IconCopy class="mr-2 size-4 opacity-60" />
            <span>Copy DDL</span>
        </ContextMenu.Item>
    {/if}

    {#if isConstraint}
        <ContextMenu.Item onclick={() => act("copy_constraint_ddl")}>
            <IconCopy class="mr-2 size-4 opacity-60" />
            <span>Copy DDL</span>
        </ContextMenu.Item>
    {/if}

    {#if isSchema}
        <ContextMenu.Item onclick={() => act("open_schema_ddl")}>
            <IconCode class="mr-2 size-4 opacity-60" />
            <span>Open DDL</span>
        </ContextMenu.Item>
        <ContextMenu.Separator />
        <ContextMenu.Item onclick={() => act("refresh_schema")}>
            <IconRefresh class="mr-2 size-4 opacity-60" />
            <span>Refresh Schema</span>
        </ContextMenu.Item>
    {/if}

    {#if isTable || isView || isMatView}
        <ContextMenu.Separator />
    {/if}

    <ContextMenu.Item onclick={() => act("query_console")}>
        <IconFileDatabase class="mr-2 size-4 text-primary" />
        <span>Query Console</span>
        <ContextMenu.Shortcut>⇧⌘L</ContextMenu.Shortcut>
    </ContextMenu.Item>
</ContextMenu.Content>
