<script lang="ts">
    import * as ContextMenu from "$lib/components/ui/context-menu";
    import IconFileDatabase from "@tabler/icons-svelte/icons/file-database";
    import IconTable from "@tabler/icons-svelte/icons/table";
    import IconListSearch from "@tabler/icons-svelte/icons/list-search";
    import IconInfoCircle from "@tabler/icons-svelte/icons/info-circle";
    import IconDatabase from "@tabler/icons-svelte/icons/database";
    import IconCloud from "@tabler/icons-svelte/icons/cloud";
    import IconFolder from "@tabler/icons-svelte/icons/folder";
    import IconAdjustments from "@tabler/icons-svelte/icons/adjustments";
    import ColumnIcon from "$lib/components/icons/ColumnIcon.svelte";
    import type { TreeNode } from "./FileTree.svelte";

    let {
        node,
        onAction = (action: string, node: TreeNode) => {},
    }: {
        node: TreeNode;
        onAction?: (action: string, node: TreeNode) => void;
    } = $props();

    const isTable = $derived(node.type === "table");
    const isColumn = $derived(
        node.type === "column" ||
            node.type === "primary_key" ||
            node.type === "foreign_key",
    );
    const isGroup = $derived(node.type === "group");
    const isSchema = $derived(node.type === "schema");
</script>

<ContextMenu.Content class="w-64">
    <ContextMenu.Item onclick={() => onAction("query_console", node)}>
        <IconFileDatabase class="mr-2 size-4 text-primary" />
        <span>Query Console</span>
        <ContextMenu.Shortcut>⇧⌘L</ContextMenu.Shortcut>
    </ContextMenu.Item>

    <ContextMenu.Separator />

    <ContextMenu.Item
        onclick={() => onAction("new_column", node)}
        disabled={!isTable}
    >
        <ColumnIcon class="mr-2 size-4" />
        <span>Column</span>
    </ContextMenu.Item>
    <ContextMenu.Item
        onclick={() => onAction("new_index", node)}
        disabled={!isTable}
    >
        <IconInfoCircle class="mr-2 size-4 text-blue-500" />
        <span>Index</span>
    </ContextMenu.Item>
    <ContextMenu.Item
        onclick={() => onAction("new_table", node)}
        disabled={!isSchema && !isGroup}
    >
        <IconTable class="mr-2 size-4" />
        <span>Table</span>
    </ContextMenu.Item>
    <ContextMenu.Item
        onclick={() => onAction("new_view", node)}
        disabled={!isSchema && !isGroup}
    >
        <IconListSearch class="mr-2 size-4 text-blue-400" />
        <span>View</span>
    </ContextMenu.Item>

    <ContextMenu.Separator />

    <ContextMenu.Item
        onclick={() => onAction("new_v_column", node)}
        disabled={!isTable}
    >
        <ColumnIcon class="mr-2 size-4 text-purple-500" />
        <span>Virtual Column</span>
    </ContextMenu.Item>
    <ContextMenu.Item
        onclick={() => onAction("new_v_fk", node)}
        disabled={!isTable}
    >
        <span
            class="mr-2 size-4 flex items-center justify-center font-bold text-purple-500"
            >♀</span
        >
        <span>Virtual Foreign Key</span>
    </ContextMenu.Item>
    <ContextMenu.Item
        onclick={() => onAction("new_v_view", node)}
        disabled={!isSchema && !isGroup}
    >
        <IconListSearch class="mr-2 size-4 text-purple-500" />
        <span>Virtual View</span>
    </ContextMenu.Item>

    <ContextMenu.Separator />

    <ContextMenu.Item onclick={() => onAction("new_ds_postgres", node)}>
        <IconDatabase class="mr-2 size-4" />
        <span>PostgreSQL Data Source</span>
    </ContextMenu.Item>
    <ContextMenu.Item onclick={() => onAction("new_ds_sqlite", node)}>
        <IconDatabase class="mr-2 size-4" />
        <span>SQLite Data Source</span>
    </ContextMenu.Item>
    <ContextMenu.Item onclick={() => onAction("new_ds_duckdb", node)}>
        <IconDatabase class="mr-2 size-4" />
        <span>DuckDB Data Source</span>
    </ContextMenu.Item>

    <ContextMenu.Separator />

    <ContextMenu.Item onclick={() => onAction("new_ddl_ds", node)}>
        <IconFileDatabase class="mr-2 size-4 opacity-50" />
        <span>DDL Data Source</span>
    </ContextMenu.Item>
    <ContextMenu.Item onclick={() => onAction("new_ds_url", node)}>
        <span
            class="mr-2 size-4 flex items-center justify-center text-muted-foreground text-lg"
            >🔌</span
        >
        <span>Data Source from URL</span>
    </ContextMenu.Item>
    <ContextMenu.Item onclick={() => onAction("new_ds_path", node)}>
        <IconFolder class="mr-2 size-4" />
        <span>Data Source from Path</span>
    </ContextMenu.Item>

    <ContextMenu.Separator />

    <ContextMenu.Item onclick={() => onAction("driver_settings", node)}>
        <IconAdjustments class="mr-2 size-4" />
        <span>Driver Settings</span>
    </ContextMenu.Item>
</ContextMenu.Content>
