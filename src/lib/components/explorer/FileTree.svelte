<script lang="ts" context="module">
    import ChevronRight from "@tabler/icons-svelte/icons/chevron-right";
    import Folder from "@tabler/icons-svelte/icons/folder";
    import FolderOpen from "@tabler/icons-svelte/icons/folder-open";
    import FileText from "@tabler/icons-svelte/icons/file-text";
    import Database from "@tabler/icons-svelte/icons/database";
    import Key from "@tabler/icons-svelte/icons/key";
    import Table from "@tabler/icons-svelte/icons/table";
    import Columns from "@tabler/icons-svelte/icons/columns";
    import Bolt from "@tabler/icons-svelte/icons/bolt";
    import ListSearch from "@tabler/icons-svelte/icons/list-search";
    import Cube from "@tabler/icons-svelte/icons/cube"; // For Schema
    import ColumnIcon from "$lib/components/icons/ColumnIcon.svelte";
    import PrimaryKeyIcon from "$lib/components/icons/PrimaryKeyIcon.svelte";

    export type NodeType =
        | "folder"
        | "file"
        | "database"
        | "key"
        | "primary_key"
        | "schema"
        | "table"
        | "column"
        | "index"
        | "trigger";

    export type TreeNode = {
        name: string;
        type?: NodeType;
        children?: TreeNode[];
        id?: string;
        detail?: string; // For type info like "VARCHAR(45)"
        icon?: any; // Allow overriding icon
    };
</script>

<script lang="ts">
    import { slide } from "svelte/transition";
    import { cn } from "$lib/utils";

    let {
        items = [] as TreeNode[],
        class: className = "",
        indent = 24,
        onNodeClick = (node: TreeNode) => {},
    } = $props();

    let expanded = $state<Set<string>>(new Set());

    // Helper to generate a unique key if id is missing
    // NOTE: For streaming data, ensure each node has a stable unique 'id'.
    // Relying on name+index fallback may cause issues if items are inserted/reordered.
    const getKey = (node: TreeNode, index: number) =>
        node.id || `${node.name}-${index}`;

    const typeIcon: Partial<Record<NodeType, any>> = {
        folder: Folder,
        file: FileText,
        database: Database,
        key: Key,
        schema: Cube,
        table: Table,
        column: ColumnIcon,
        primary_key: PrimaryKeyIcon,
        index: ListSearch,
        trigger: Bolt,
    };

    function toggle(key: string) {
        const next = new Set(expanded);
        if (next.has(key)) next.delete(key);
        else next.add(key);
        expanded = next;
    }

    function expandAll() {
        const all = new Set<string>();
        const traverse = (nodes: TreeNode[], iPrefix = "") => {
            nodes.forEach((node, index) => {
                const key = getKey(node, index);
                if (node.children?.length) {
                    all.add(key);
                    traverse(node.children, key);
                }
            });
        };
        traverse(items);
        expanded = all;
    }

    function collapseAll() {
        expanded = new Set();
    }
</script>

<div
    class={cn("font-mono text-sm select-none flex flex-col h-full", className)}
>
    <!-- Toolbar -->
    <div
        class="flex items-center gap-2 mb-2 px-1 text-xs text-muted-foreground/60"
    >
        <button
            onclick={expandAll}
            class="hover:text-foreground transition-colors">Expand All</button
        >
        <span>/</span>
        <button
            onclick={collapseAll}
            class="hover:text-foreground transition-colors">Collapse All</button
        >
    </div>

    <!-- Tree -->
    <ul class="flex flex-col gap-0.5 overflow-auto flex-1">
        {#each items as item, i (getKey(item, i))}
            {@render TreeItem({ node: item, index: i, depth: 0 })}
        {/each}
    </ul>
</div>

{#snippet TreeItem({
    node,
    index,
    depth,
}: {
    node: TreeNode;
    index: number;
    depth: number;
})}
    {@const key = getKey(node, index)}
    {@const isFolder = !!node.children?.length || node.type === "folder"}
    {@const isOpen = expanded.has(key)}

    <li class="relative">
        <div
            class={cn(
                "group flex items-center gap-2 rounded-md cursor-pointer transition-colors border border-transparent",
                node.type === "column" || node.type === "primary_key"
                    ? "py-0.5 text-xs"
                    : "py-1 text-sm hover:bg-accent/50 text-foreground/80 hover:text-foreground",
            )}
            style="padding-left: calc({indent}px * {depth} + 4px);"
            onclick={(e) => {
                e.stopPropagation();
                if (isFolder) {
                    toggle(key);
                }
                onNodeClick(node);
            }}
            onkeydown={(e) =>
                (e.key === "Enter" || e.key === " ") && isFolder && toggle(key)}
            role="button"
            tabindex="0"
        >
            <!-- Arrow -->
            <span
                class="flex items-center justify-center size-4 shrink-0 text-muted-foreground/50"
            >
                {#if isFolder}
                    <ChevronRight
                        class={cn(
                            "size-3.5 transition-transform duration-200",
                            isOpen && "rotate-90",
                        )}
                    />
                {/if}
            </span>

            <!-- Icon -->
            <span
                class="flex items-center justify-center size-4 shrink-0 text-muted-foreground"
            >
                {#if node.type === "folder"}
                    {#if isOpen}
                        <FolderOpen class="size-4" />
                    {:else}
                        <Folder class="size-4" />
                    {/if}
                {:else}
                    {@const Icon = typeIcon[node.type || "file"] || FileText}
                    <Icon class="size-4" />
                {/if}
            </span>

            <!-- Label -->
            <span class="truncate leading-none opacity-90">{node.name}</span>

            <!-- Detail (Type info, etc) -->
            {#if node.detail}
                <span class="ml-2 text-xs text-muted-foreground truncate"
                    >{node.detail}</span
                >
            {/if}
        </div>

        <!-- Children -->
        {#if isFolder && isOpen && node.children}
            <div class="relative" transition:slide={{ duration: 500 }}>
                <!-- Vertical Line -->
                <!-- 
                   The line needs to align with the arrow center.
                   Arrow center x = (depth * indent) + padding-left (4px) + arrow width/2 (8px) = depth * indent + 12px.
                   We render children with depth + 1. 
                   The line should effectively sit at `left: (depth * indent) + 12px`.
                -->
                <div
                    class="absolute top-0 bottom-0 w-px bg-border/60"
                    style="left: calc({indent}px * {depth} + 12px);"
                ></div>

                <ul class="flex flex-col gap-0.5">
                    {#each node.children as child, childIndex (getKey(child, childIndex))}
                        {@render TreeItem({
                            node: child,
                            index: childIndex,
                            depth: depth + 1,
                        })}
                    {/each}
                </ul>
            </div>
        {/if}
    </li>
{/snippet}
