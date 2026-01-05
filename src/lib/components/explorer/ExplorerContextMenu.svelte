<script lang="ts">
    import * as ContextMenu from "$lib/components/ui/context-menu";
    import IconFileDatabase from "@tabler/icons-svelte/icons/file-database";
    import IconTable from "@tabler/icons-svelte/icons/table";
    import IconListSearch from "@tabler/icons-svelte/icons/list-search";
    import IconInfoCircle from "@tabler/icons-svelte/icons/info-circle";
    import ColumnIcon from "$lib/components/icons/ColumnIcon.svelte";
    import type { ExplorerNode } from "$lib/explorer/types";

    let {
        node,
        onAction = (action: string, node: ExplorerNode) => {},
    }: {
        node: ExplorerNode;
        onAction?: (action: string, node: ExplorerNode) => void;
    } = $props();

    const isTable = $derived(node.kind === "table");
    const isGroup = $derived(node.kind === "group");
    const isSchema = $derived(node.kind === "schema");
    const isDatabase = $derived(node.kind === "database");
</script>

<ContextMenu.Content class="w-56">
    <ContextMenu.Item
        onclick={() => {
            console.log("[ExplorerContextMenu] Query Console clicked", {
                label: node.label,
                id: node.id,
                kind: node.kind,
            });
            onAction("query_console", node);
        }}
    >
        <IconFileDatabase class="mr-2 size-4 text-primary" />
        <span>Query Console</span>
        <ContextMenu.Shortcut>⇧⌘L</ContextMenu.Shortcut>
    </ContextMenu.Item>

    <ContextMenu.Separator />

    <ContextMenu.Item
        onclick={() => onAction("view_diagram", node)}
        disabled={!isSchema && !isTable && node.kind !== "column"}
    >
        <IconTable class="mr-2 size-4 text-purple-500" />
        <span>View Diagram</span>
    </ContextMenu.Item>

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
        onclick={() => onAction("refresh", node)}
        disabled={!isSchema && node.kind !== "database"}
    >
        <IconListSearch class="mr-2 size-4 text-green-500" />
        <span>Refresh</span>
    </ContextMenu.Item>
</ContextMenu.Content>
