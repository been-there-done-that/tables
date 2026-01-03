<script lang="ts" context="module">
    import ChevronRight from "@tabler/icons-svelte/icons/chevron-right";
    import Folder from "@tabler/icons-svelte/icons/folder";
    import FolderOpen from "@tabler/icons-svelte/icons/folder-open";
    import FileText from "@tabler/icons-svelte/icons/file-text";
    import Database from "@tabler/icons-svelte/icons/server"; // Creative change
    import Key from "@tabler/icons-svelte/icons/key";
    import Columns from "@tabler/icons-svelte/icons/columns";
    import Bolt from "@tabler/icons-svelte/icons/bolt";
    import ListSearch from "@tabler/icons-svelte/icons/list-search";
    import Cube from "@tabler/icons-svelte/icons/cube"; // Revert to Cube, or use Box
    import ViewIcon from "@tabler/icons-svelte/icons/eye"; // For Views
    import SqlIcon from "@tabler/icons-svelte/icons/file-database"; // Action icon
    import LoaderIcon from "@tabler/icons-svelte/icons/loader-2"; // Spinner
    import ColumnIcon from "$lib/components/icons/ColumnIcon.svelte";
    import TableIcon from "$lib/components/icons/TableIcon.svelte";
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
        | "view"
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
        isConnected?: boolean; // For database connection status
        isLoading?: boolean; // For on-demand loading state
        metadata?: Record<string, any>; // For lazy loading metadata
    };
</script>

<script lang="ts">
    import { slide } from "svelte/transition";
    import * as ContextMenu from "$lib/components/ui/context-menu";
    import ExplorerContextMenu from "./ExplorerContextMenu.svelte";
    import { cn } from "$lib/utils";

    interface Props {
        items?: TreeNode[];
        class?: string;
        isCompact?: boolean;
        indent?: number;
        onNodeClick?: (node: TreeNode) => void;
        onAction?: (node: TreeNode) => void;
        onContextMenuAction?: (action: string, node: TreeNode) => void;
        onExpand?: (node: TreeNode, isOpen: boolean) => void;
        expanded?: Set<string>;
    }

    let {
        items = [] as TreeNode[],
        class: className = "",
        isCompact = false,
        indent = 16,
        onNodeClick = (node: TreeNode) => {},
        onAction = (node: TreeNode) => {},
        onContextMenuAction = (action: string, node: TreeNode) => {},
        onExpand = (node: TreeNode, isOpen: boolean) => {},
        expanded = $bindable(new Set()),
    }: Props = $props();

    // Helper to generate a unique key if id is missing
    // NOTE: For streaming data, ensure each node has a stable unique 'id'.
    // Relying on name+index fallback may cause issues if items are inserted/reordered.
    const getKey = (node: TreeNode, index: number) =>
        node.id || `${node.name}-${index}`;

    const typeIcon: Partial<Record<NodeType, any>> = {
        folder: Folder,
        group: Folder,
        file: FileText,
        database: Database, // uses Server now
        key: Key,
        schema: Cube, // uses BoxSeam now
        table: TableIcon, // Custom SVG!
        view: ViewIcon, // New!
        column: ColumnIcon,
        primary_key: PrimaryKeyIcon,
        index: ListSearch,
        trigger: Bolt,
        foreign_key: Key,
    };

    function toggle(key: string, node: TreeNode) {
        const next = new Set(expanded);
        const willOpen = !next.has(key);
        if (next.has(key)) next.delete(key);
        else next.add(key);
        expanded = next;
        if (onExpand) onExpand(node, willOpen);
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
    class={cn(
        "font-mono text-[13px] select-none flex flex-col h-full",
        className,
    )}
>
    <!-- Tree -->
    <ul class="flex flex-col gap-0 overflow-auto flex-1">
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
        node.type === "group" ||
        node.type === "database" ||
        node.type === "schema"}
    {@const isOpen = expanded.has(key)}
    {@const isGroup = node.type === "group"}
    {@const isCompact =
        node.type === "column" ||
        node.type === "primary_key" ||
        node.type === "index" ||
        node.type === "foreign_key" ||
        node.type === "table" ||
        node.type === "view"}

    <li class="relative">
        <ContextMenu.Root>
            <ContextMenu.Trigger>
                <div
                    class={cn(
                        "group flex items-center gap-1.5 rounded-sm cursor-default transition-colors border border-transparent",
                        isCompact
                            ? "h-6 hover:bg-accent/40 text-foreground/90 hover:text-foreground"
                            : "h-6 hover:bg-accent/40 text-foreground hover:text-foreground",
                    )}
                    style="padding-left: calc({indent}px * {depth} + 4px);"
                    onclick={(e) => {
                        e.stopPropagation();
                        if (isFolder) {
                            toggle(key, node);
                        }
                        onNodeClick(node);
                    }}
                    onkeydown={(e) =>
                        (e.key === "Enter" || e.key === " ") &&
                        isFolder &&
                        toggle(key, node)}
                    role="button"
                    tabindex="0"
                >
                    <!-- Arrow -->
                    <span
                        class="flex items-center justify-center w-4 shrink-0 text-muted-foreground/60"
                    >
                        {#if isFolder && (node.type === "database" || (node.children && node.children.length > 0))}
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
                                <FolderOpen class="size-3.5" />
                            {:else}
                                <Folder class="size-3.5" />
                            {/if}
                        {:else}
                            {@const Icon =
                                typeIcon[node.type || "file"] || FileText}
                            <Icon class="size-3.5" />
                        {/if}
                    </span>

                    <!-- Label -->
                    <span
                        class="flex items-center gap-1.5 truncate leading-none overflow-hidden"
                    >
                        <span class="truncate">{node.name}</span>
                        {#if node.isConnected && node.type === "database"}
                            <span
                                class="size-1.5 rounded-full bg-primary shrink-0"
                                title="Connected"
                            ></span>
                        {/if}
                    </span>

                    <!-- Loader (New) -->
                    {#if node.isLoading && node.type === "database"}
                        <LoaderIcon
                            class="size-2.5 ml-1 animate-spin text-muted-foreground/70 shrink-0"
                        />
                    {/if}

                    <!-- Count (New) -->
                    {#if node.count !== undefined}
                        <span class="ml-1 text-[11px] text-muted-foreground/60"
                            >{node.count}</span
                        >
                    {/if}

                    <!-- Detail (Type info, etc) -->
                    {#if node.detail}
                        <span
                            class="ml-2 text-xs text-muted-foreground truncate"
                            >{node.detail}</span
                        >
                    {/if}

                    <!-- Action Icon (Visible on Hover) -->
                    {#if node.type === "table" || node.type === "column" || node.type === "primary_key" || node.type === "foreign_key"}
                        <button
                            class="ml-auto p-1 rounded-sm opacity-0 group-hover:opacity-100 hover:bg-accent text-muted-foreground hover:text-foreground transition-all duration-200"
                            onclick={(e) => {
                                e.stopPropagation();
                                onAction(node);
                            }}
                            title="Open in new tab"
                        >
                            <SqlIcon class="size-4" />
                        </button>
                    {/if}
                </div>
            </ContextMenu.Trigger>
            <ExplorerContextMenu {node} onAction={onContextMenuAction} />
        </ContextMenu.Root>

        <!-- Children -->
        {#if isFolder && isOpen && node.children}
            <div class="relative" transition:slide={{ duration: 200 }}>
                <!-- Vertical Line -->
                <div
                    class="absolute top-0 bottom-0 w-px bg-border/40"
                    style="left: calc({indent}px * {depth} + 10px);"
                ></div>

                <ul class="flex flex-col gap-0">
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
