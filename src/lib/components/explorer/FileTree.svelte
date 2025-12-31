<script lang="ts" context="module">
    import ChevronRight from "@tabler/icons-svelte/icons/chevron-right";
    import Folder from "@tabler/icons-svelte/icons/folder";
    import FolderOpen from "@tabler/icons-svelte/icons/folder-open";
    import FileText from "@tabler/icons-svelte/icons/file-text";
    import Database from "@tabler/icons-svelte/icons/database";
    import Key from "@tabler/icons-svelte/icons/key";

    export type NodeType = "folder" | "file" | "database" | "key";

    export type TreeNode = {
        name: string;
        type?: NodeType;
        children?: TreeNode[];
        id?: string;
    };
</script>

<script lang="ts">
    import { cn } from "$lib/utils";

    let { items = [] as TreeNode[], class: className = "" } = $props();

    let expanded = $state<Set<string>>(new Set());

    // Helper to generate a unique key if id is missing
    const getKey = (node: TreeNode, index: number) =>
        node.id || `${node.name}-${index}`;

    const typeIcon: Partial<Record<NodeType, any>> = {
        folder: Folder,
        file: FileText,
        database: Database,
        key: Key,
    };

    function toggle(key: string) {
        const next = new Set(expanded);
        if (next.has(key)) next.delete(key);
        else next.add(key);
        expanded = next;
    }

    // Auto-expand all initially (optional, but good for demo)
    // $effect(() => {
    //   const allKeys = new Set<string>();
    //   const traverse = (nodes: TreeNode[], prefix = "") => {
    //      nodes.forEach((n, i) => {
    //         if (n.children?.length) {
    //             const key = getKey(n, i);
    //             allKeys.add(key);
    //             traverse(n.children, key);
    //         }
    //      });
    //   };
    //   traverse(items);
    //   expanded = allKeys;
    // });
</script>

<div class={cn("font-mono text-sm select-none", className)}>
    <ul class="flex flex-col gap-1">
        {#each items as item, i (getKey(item, i))}
            {@render TreeItem({ node: item, index: i })}
        {/each}
    </ul>
</div>

{#snippet TreeItem({ node, index }: { node: TreeNode; index: number })}
    {@const key = getKey(node, index)}
    {@const isFolder = !!node.children?.length || node.type === "folder"}
    {@const isOpen = expanded.has(key)}

    <li class="relative">
        <div
            class={cn(
                "group flex items-center gap-1.5 py-1 px-2 rounded-md cursor-pointer transition-colors",
                "hover:bg-accent/50 text-foreground/80 hover:text-foreground",
            )}
            onclick={(e) => {
                if (isFolder) {
                    e.stopPropagation();
                    toggle(key);
                }
            }}
            onkeydown={(e) =>
                (e.key === "Enter" || e.key === " ") && isFolder && toggle(key)}
            role="button"
            tabindex="0"
        >
            <!-- Arrow / Spacer -->
            <span
                class="flex items-center justify-center size-4 shrink-0 text-muted-foreground/70"
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
            <span class="truncate leading-none">{node.name}</span>
        </div>

        <!-- Children -->
        {#if isFolder && isOpen && node.children}
            <div class="relative pl-4 ml-3 border-l border-border/60">
                <ul class="flex flex-col gap-0.5 pt-0.5">
                    {#each node.children as child, childIndex (getKey(child, childIndex))}
                        {@render TreeItem({ node: child, index: childIndex })}
                    {/each}
                </ul>
            </div>
        {/if}
    </li>
{/snippet}
