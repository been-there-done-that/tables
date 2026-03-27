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
        selectedNodeId?: string | null;
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
        selectedNodeId = $bindable<string | null>(null),
    }: Props = $props();

    // Helper to generate a unique key if id is missing
    const getKey = (node: TreeNode, index: number) =>
        node.id || `${node.name}-${index}`;

    const typeIcon: Partial<Record<NodeType, any>> = {
        folder: Folder,
        group: Folder,
        file: FileText,
        database: Database,
        key: Key,
        schema: Cube,
        table: TableIcon,
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

    // --- Keyboard Navigation ---

    type FlatNode = {
        id: string;
        node: TreeNode;
        parentId: string | null;
        depth: number;
    };

    function getVisibleNodes(): FlatNode[] {
        const result: FlatNode[] = [];

        const traverse = (
            nodes: TreeNode[],
            depth: number,
            parentId: string | null,
        ) => {
            for (let i = 0; i < nodes.length; i++) {
                const node = nodes[i];
                const id = getKey(node, i); // This index reuse might be buggy for global uniqueness if names clash.
                // Assuming getKey uses node.id which is globally unique.

                result.push({ id, node, parentId, depth });

                if (expanded.has(id) && node.children?.length) {
                    traverse(node.children, depth + 1, id); // Recurse
                }
            }
        };

        traverse(items, 0, null);
        return result;
    }

    let treeContainer: HTMLDivElement;
    let interactionMode = $state<"mouse" | "keyboard">("mouse");

    function handleKeyDown(e: KeyboardEvent) {
        // Only handle if focused within the tree container
        // Using currentTarget ensures we caught it on the div
        if (!items.length) return;

        interactionMode = "keyboard";

        const visible = getVisibleNodes();
        const currentIndex = visible.findIndex((n) => n.id === selectedNodeId);

        switch (e.key) {
            case "ArrowDown": {
                e.preventDefault();
                const nextIndex =
                    currentIndex < visible.length - 1
                        ? currentIndex + 1
                        : currentIndex;
                if (nextIndex !== -1) selectNode(visible[nextIndex].id);
                else if (visible.length > 0) selectNode(visible[0].id);
                break;
            }
            case "ArrowUp": {
                e.preventDefault();
                const prevIndex = currentIndex > 0 ? currentIndex - 1 : 0;
                if (prevIndex !== -1) selectNode(visible[prevIndex].id);
                else if (visible.length > 0) selectNode(visible[0].id);
                break;
            }
            case "ArrowRight": {
                e.preventDefault();
                if (currentIndex === -1) return;
                const current = visible[currentIndex];
                const hasChildren =
                    current.node.children && current.node.children.length > 0;

                if (hasChildren) {
                    if (expanded.has(current.id)) {
                        // Move to first child
                        const nextIndex = currentIndex + 1;
                        if (nextIndex < visible.length)
                            selectNode(visible[nextIndex].id);
                    } else {
                        // Expand
                        toggle(current.id, current.node);
                    }
                }
                break;
            }
            case "ArrowLeft": {
                e.preventDefault();
                if (currentIndex === -1) return;
                const current = visible[currentIndex];

                if (expanded.has(current.id)) {
                    // Collapse
                    toggle(current.id, current.node);
                } else if (current.parentId) {
                    // Move to parent
                    selectNode(current.parentId);
                }
                break;
            }
            case "Enter":
            case " ": {
                // Space key - same behavior as Enter
                e.preventDefault();
                if (currentIndex === -1) return;
                const current = visible[currentIndex];
                // Tables/views trigger action, other expandables toggle
                const isActionable =
                    current.node.type === "table" ||
                    current.node.type === "view";
                if (isActionable) {
                    onAction(current.node);
                } else if (
                    current.node.children &&
                    current.node.children.length > 0
                ) {
                    toggle(current.id, current.node);
                } else {
                    onAction(current.node);
                }
                break;
            }
        }
    }

    function selectNode(id: string) {
        selectedNodeId = id;
        setTimeout(() => {
            const el = document.querySelector(
                `[data-node-id="${CSS.escape(id)}"]`,
            ) as HTMLElement;
            if (el) {
                el.scrollIntoView({ block: "nearest", inline: "nearest" });
                // If tree container has focus or is a parent of active element, move focus to the node
                if (
                    document.activeElement === treeContainer ||
                    treeContainer?.contains(document.activeElement)
                ) {
                    el.focus();
                }
            }
        }, 0);
    }
</script>

<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<div
    bind:this={treeContainer}
    class={cn(
        "font-mono text-[13px] select-none flex flex-col h-full outline-none",
        className,
    )}
    tabindex="0"
    onkeydown={handleKeyDown}
    onfocus={(e) => {
        if (e.target === treeContainer) {
            const selected = treeContainer.querySelector(
                '[data-selected="true"]',
            ) as HTMLElement;
            if (selected) {
                selected.focus();
            } else if (items.length > 0) {
                const visible = getVisibleNodes();
                if (visible.length > 0) {
                    selectNode(visible[0].id);
                }
            }
        }
    }}
    onmousemove={() => (interactionMode = "mouse")}
    role="tree"
>
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

    {@const isSelected = selectedNodeId === key}
    <li class="relative">
        <ContextMenu.Root>
            <ContextMenu.Trigger>
                <!-- svelte-ignore a11y_click_events_have_key_events -->
                <div
                    class={cn(
                        "group flex items-center gap-1.5 rounded-sm cursor-default transition-colors border border-transparent h-6 outline-none ring-0 focus:outline-none focus:ring-0",
                        isSelected
                            ? "bg-primary/20 text-foreground"
                            : interactionMode === "mouse"
                              ? isCompact
                                  ? "hover:bg-accent/40 text-foreground/90 hover:text-foreground"
                                  : "hover:bg-accent/40 text-foreground hover:text-foreground"
                              : isCompact
                                ? "text-foreground/90"
                                : "text-foreground",
                    )}
                    data-node-id={key}
                    data-selected={isSelected}
                    tabindex="-1"
                    style="padding-left: calc({indent}px * {depth} + 4px);"
                    onclick={(e) => {
                        e.stopPropagation();
                        // Focus the clicked node directly — not the container — so the
                        // onfocus auto-select fallback doesn't fire before the DOM updates.
                        selectNode(key);
                        (e.currentTarget as HTMLElement).focus();
                        // Tables and views trigger action on click, other folders toggle
                        const isActionable =
                            node.type === "table" || node.type === "view";
                        if (isActionable) {
                            onAction(node);
                        } else if (isFolder) {
                            toggle(key, node);
                        }
                        onNodeClick(node);
                    }}
                    role="none"
                >
                    <!-- svelte-ignore a11y_click_events_have_key_events -->
                    <span
                        class="flex items-center justify-center w-4 shrink-0 text-muted-foreground/60 cursor-pointer"
                        onclick={(e) => {
                            e.stopPropagation();
                            if (node.children && node.children.length > 0) {
                                toggle(key, node);
                            }
                        }}
                        role="button"
                        tabindex="-1"
                    >
                        {#if isFolder && (node.type === "database" || (node.children && node.children.length > 0))}
                            <ChevronRight
                                class={cn(
                                    "size-4 transition-transform duration-200",
                                    isOpen && "rotate-90",
                                )}
                            />
                        {/if}
                    </span>

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
                    {#if node.isLoading && (node.type === "database" || node.type === "schema")}
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
                    style="left: calc({indent}px * {depth} + 12px);"
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
