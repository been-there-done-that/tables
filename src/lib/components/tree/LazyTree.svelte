<!--
  LazyTree.svelte - Pure Tree Logic Component
  
  This component handles tree state (expansion, selection, keyboard navigation)
  without knowing anything about the data being displayed.
  
  It uses Svelte 5 snippets to delegate rendering to the parent.
-->
<script lang="ts" module>
    /**
     * A generic tree node interface.
     * The data type is generic to support any kind of tree structure.
     */
    export interface TreeNode<T = unknown> {
        id: string;
        data: T;
        hasChildren: boolean;
        level: number;
    }

    /**
     * Render context passed to the node snippet.
     */
    export interface NodeContext<T> {
        node: TreeNode<T>;
        isExpanded: boolean;
        isSelected: boolean;
        isLoading: boolean;
        toggle: () => void;
        select: () => void;
    }
</script>

<script lang="ts" generics="T">
    import type { Snippet } from "svelte";
    import { cn } from "$lib/utils";

    // Props
    let {
        nodes = [],
        renderNode,
        expanded = $bindable(new Set<string>()),
        selected = $bindable<string | null>(null),
        loadingNodes = new Set<string>(),
        onExpand,
        onCollapse,
        onSelect,
        class: className = "",
    }: {
        nodes: TreeNode<T>[];
        renderNode: Snippet<[NodeContext<T>]>;
        expanded?: Set<string>;
        selected?: string | null;
        loadingNodes?: Set<string>;
        onExpand?: (node: TreeNode<T>) => void;
        onCollapse?: (node: TreeNode<T>) => void;
        onSelect?: (node: TreeNode<T>) => void;
        class?: string;
    } = $props();

    // Toggle expansion
    function toggle(node: TreeNode<T>) {
        if (expanded.has(node.id)) {
            expanded.delete(node.id);
            expanded = new Set(expanded); // Trigger reactivity
            onCollapse?.(node);
        } else {
            expanded.add(node.id);
            expanded = new Set(expanded);
            onExpand?.(node);
        }
    }

    // Select node
    function select(node: TreeNode<T>) {
        selected = node.id;
        onSelect?.(node);
    }

    // Build context for each node
    function buildContext(node: TreeNode<T>): NodeContext<T> {
        return {
            node,
            isExpanded: expanded.has(node.id),
            isSelected: selected === node.id,
            isLoading: loadingNodes.has(node.id),
            toggle: () => toggle(node),
            select: () => select(node),
        };
    }

    // Keyboard navigation
    function handleKeydown(event: KeyboardEvent) {
        if (!nodes.length) return;

        const currentIndex = nodes.findIndex((n) => n.id === selected);

        switch (event.key) {
            case "ArrowDown":
                event.preventDefault();
                if (currentIndex < nodes.length - 1) {
                    select(nodes[currentIndex + 1]);
                }
                break;
            case "ArrowUp":
                event.preventDefault();
                if (currentIndex > 0) {
                    select(nodes[currentIndex - 1]);
                }
                break;
            case "ArrowRight":
                event.preventDefault();
                if (currentIndex >= 0) {
                    const node = nodes[currentIndex];
                    if (node.hasChildren && !expanded.has(node.id)) {
                        toggle(node);
                    }
                }
                break;
            case "ArrowLeft":
                event.preventDefault();
                if (currentIndex >= 0) {
                    const node = nodes[currentIndex];
                    if (expanded.has(node.id)) {
                        toggle(node);
                    }
                }
                break;
            case "Enter":
            case " ":
                event.preventDefault();
                if (currentIndex >= 0) {
                    toggle(nodes[currentIndex]);
                }
                break;
        }
    }
</script>

<div
    class={cn(
        "outline-none focus-visible:ring-2 focus-visible:ring-accent focus-visible:ring-inset",
        className,
    )}
    role="tree"
    tabindex="0"
    onkeydown={handleKeydown}
>
    {#each nodes as node (node.id)}
        {@render renderNode(buildContext(node))}
    {/each}
</div>
