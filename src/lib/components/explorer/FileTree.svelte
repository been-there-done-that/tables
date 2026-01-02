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
        | "group"
        | "file"
        | "database"
        | "key"
        | "primary_key"
        | "schema"
        | "table"
        | "column"
        | "index"
        | "trigger"
        | "foreign_key";

    export type TreeNode = {
        name: string;
        type?: NodeType;
        children?: TreeNode[];
        id?: string;
        detail?: string; // For type info like "VARCHAR(45)"
        icon?: any; // Allow overriding icon
        count?: number; // Added for displaying item counts
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
        expanded = $bindable(new Set()),
    } = $props();

    // Helper to generate a unique key if id is missing
    // NOTE: For streaming data, ensure each node has a stable unique 'id'.
    // Relying on name+index fallback may cause issues if items are inserted/reordered.
    const getKey = (node: TreeNode, index: number) =>
        node.id || `${node.name}-${index}`;

    const typeIcon: Partial<Record<NodeType, any>> = {
        folder: Folder,
        group: Folder,
        file: FileText,
        database: Database,
        key: Key,
        schema: Cube,
        table: Table,
        column: ColumnIcon,
        primary_key: PrimaryKeyIcon,
        index: ListSearch,
        trigger: Bolt,
        foreign_key: Key,
    };

    function toggle(key: string) {
        const next = new Set(expanded);
        if (next.has(key)) next.delete(key);
        else next.add(key);
        expanded = next;
    }

    export function expandAll() {
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

    export function collapseAll() {
        expanded = new Set();
    }
</script>

<div
    class={cn("font-mono text-sm select-none flex flex-col h-full", className)}
>
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
    {@const isFolder =
        !!node.children?.length ||
        node.type === "folder" ||
        node.type === "group"}
    {@const isOpen = expanded.has(key)}
    {@const isGroup = node.type === "group"}
    {@const isCompact =
        node.type === "column" ||
        node.type === "primary_key" ||
        node.type === "index" ||
        node.type === "foreign_key"}

    <li class="relative">
        <div
            class={cn(
                "group flex items-center gap-2 rounded-md cursor-default transition-colors border border-transparent",
                isCompact
                    ? "py-0 text-xs h-5 hover:bg-accent/50 text-foreground/80 hover:text-foreground" // Compact with hover
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
                {#if isFolder && node.children && node.children.length > 0}
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
                {#if node.type === "folder" || node.type === "group"}
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
            <span class="truncate leading-none opacity-90">
                {node.name}
            </span>

            <!-- Count (New) -->
            {#if node.count !== undefined}
                <span class="ml-1 text-[10px] text-muted-foreground/70"
                    >({node.count})</span
                >
            {/if}

            <!-- Detail (Type info, etc) -->
            {#if node.detail}
                <span class="ml-2 text-xs text-muted-foreground truncate"
                    >{node.detail}</span
                >
            {/if}
        </div>

        <!-- Children -->
        {#if isFolder && isOpen && node.children}
            <div class="relative" transition:slide={{ duration: 200 }}>
                <!-- Vertical Line -->
                <div
                    class="absolute top-0 bottom-0 w-px bg-border/60"
                    style="left: calc({indent}px * {depth} + 12px);"
                ></div>

                <ul class={cn("flex flex-col", isGroup ? "gap-0" : "gap-0.5")}>
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
