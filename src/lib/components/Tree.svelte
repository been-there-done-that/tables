<script module lang="ts">
  export type NodeType = "folder" | "file" | "database" | "key";
  export type TreeNode = {
    id: string;
    label: string;
    type?: NodeType;
    children?: TreeNode[];
  };
</script>

<script lang="ts">
  import { cn } from "$lib/utils";
  import ChevronRight from "@tabler/icons-svelte/icons/chevron-right";
  import ChevronDown from "@tabler/icons-svelte/icons/chevron-down";
  import Folder from "@tabler/icons-svelte/icons/folder";
  import FileText from "@tabler/icons-svelte/icons/file-text";
  import Database from "@tabler/icons-svelte/icons/database";
  import Key from "@tabler/icons-svelte/icons/key";

  let { items = [] as TreeNode[], class: className = "" } = $props();

  const typeIcon: Partial<Record<NodeType, typeof Folder>> = {
    folder: Folder,
    file: FileText,
    database: Database,
    key: Key,
  };

  let expanded = $state<Set<string>>(new Set());

  const iconFor = (type: NodeType | undefined, fallback: typeof Folder) =>
    type ? typeIcon[type] ?? fallback : fallback;

  const isExpanded = (id: string) => expanded.has(id);

  const collectExpandable = (nodes: TreeNode[]): string[] =>
    nodes.flatMap((n) => (n.children?.length ? [n.id, ...collectExpandable(n.children)] : []));

  $effect(() => {
    expanded = new Set(collectExpandable(items));
  });

  function toggle(node: TreeNode) {
    if (!node.children?.length) return;
    const next = new Set(expanded);
    if (next.has(node.id)) next.delete(node.id);
    else next.add(node.id);
    expanded = next;
  }
</script>

<div class={cn("text-sm text-(--theme-fg-primary)", className)}>
  <ul class="space-y-0.5">
    {#each items as node (node.id)}
      {@render TreeItem({ node, depth: 0 })}
    {/each}
  </ul>
</div>

{#snippet TreeItem({ node, depth }: { node: TreeNode; depth: number })}
  {#if node.children?.length}
    {@const Icon = iconFor(node.type, Folder)}
    <li class="group relative">
      <div
        role="button"
        tabindex="0"
        class={cn(
          "w-full flex items-center gap-0.5 rounded-md px-1.5 hover:bg-[color-mix(in_srgb,var(--theme-bg-hover)_80%,transparent)]",
          "transition-colors text-left cursor-pointer"
        )}
        style={`padding-left:${depth * 1}px`}
        onclick={() => toggle(node)}
        onkeydown={(e) => (e.key === "Enter" || e.key === " ") && toggle(node)}
      >
        <button
          type="button"
          class="p-0.5 rounded hover:bg-[color-mix(in_srgb,var(--theme-bg-hover)_90%,transparent)] transition-colors"
          aria-label={isExpanded(node.id) ? "Collapse" : "Expand"}
          onclick={(e) => {
            e.stopPropagation();
            toggle(node);
          }}
        >
          <ChevronRight class="size-4 transition-transform duration-150" style={`transform: rotate(${isExpanded(node.id) ? 90 : 0}deg);`} />
        </button>
        <Icon class="size-4 opacity-80" />
        <span class="truncate">{node.label}</span>
      </div>
      {#if isExpanded(node.id)}
        <div class="pointer-events-none absolute left-[10px] top-[15px] bottom-[6px] border-l border-(--theme-border-subtle)"></div>
      {/if}
      {#if isExpanded(node.id)}
        <ul class="child-list space-y-0.5 mt-1 border-l border-(--theme-border-subtle) ml-[10px] relative">
          {#each node.children as child (child.id)}
            {@render TreeItem({ node: child, depth: depth + 1 })}
          {/each}
        </ul>
      {/if}
    </li>
  {:else}
    {@const Icon = iconFor(node.type, FileText)}
    <li>
      <div
        class={cn(
          "flex items-center gap-2 rounded-md px-1.5 hover:bg-[color-mix(in_srgb,var(--theme-bg-hover)_80%,transparent)]",
          "transition-colors"
        )}
        style={`padding-left:${depth * 8}px`}
      >
        <Icon class="size-4 opacity-80" />
        <span class="truncate">{node.label}</span>
      </div>
    </li>
  {/if}
{/snippet}

<style>
  :global(.size-4) {
    width: 16px;
    height: 16px;
    stroke-width: 1.5;
  }
</style>
