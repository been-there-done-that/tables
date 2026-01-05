<!-- src/lib/components/explorer/ExplorerRow.svelte -->
<script lang="ts">
    import { cn } from "$lib/utils";
    import { type ExplorerNode } from "$lib/explorer/types";

    // Icons
    import ChevronRight from "@tabler/icons-svelte/icons/chevron-right";
    import Folder from "@tabler/icons-svelte/icons/folder";
    import FolderOpen from "@tabler/icons-svelte/icons/folder-open";
    import FileText from "@tabler/icons-svelte/icons/file-text";
    import Database from "@tabler/icons-svelte/icons/server";
    import Package from "@tabler/icons-svelte/icons/package";
    import Table from "@tabler/icons-svelte/icons/table";
    import Eye from "@tabler/icons-svelte/icons/eye";
    import Columns from "@tabler/icons-svelte/icons/columns";
    import Key from "@tabler/icons-svelte/icons/key";
    import Bolt from "@tabler/icons-svelte/icons/bolt";
    import Loader2 from "@tabler/icons-svelte/icons/loader-2";
    import Refresh from "@tabler/icons-svelte/icons/refresh";

    import * as ContextMenu from "$lib/components/ui/context-menu";
    import ExplorerContextMenu from "./ExplorerContextMenu.svelte";

    interface Props {
        node: ExplorerNode;
        depth: number;
        expanded: boolean;
        onToggle: () => void;
        onRefresh: () => void;
        onAction: (action: string, node: ExplorerNode) => void;
    }

    let { node, depth, expanded, onToggle, onRefresh, onAction }: Props =
        $props();

    // Icon Mapping
    const icons: Record<string, any> = {
        connection: Database,
        database: Database,
        schema: Package,
        group: Folder,
        table: Table,
        view: Eye,
        column: Columns,
        index: FileText,
        foreign_key: Key,
        trigger: Bolt,
    };

    const Icon = $derived(node.icon || icons[node.kind] || FileText);
    const isSpinner = $derived(node.loadState === "loading");
    const isExpandable = $derived(
        node.kind === "connection" ||
            node.kind === "database" ||
            node.kind === "schema" ||
            node.kind === "group" ||
            (node.kind === "table" && false), // Tables have children (columns) but we might want them to "open" as a file first?
        // Logic: If I click a table, I want to VIEW DATA (action). The twistie expands.
        // So clicking LABEL -> Action. Clicking Twistie -> Expand.
        // Current Code: Whole row click -> if expandable then toggle.
        // Adjustment: Separating click targets or logic.
        // Let's stick to standard behavior: Click = Select/Open. Twistie = Expand.
    );

    // Adjusted logic:
    // Twistie is separate click target.
    // Row body is Action target.
    // But for folders (Group, Schema, Database), Click usually expands too.

    function handleRowClick(e: MouseEvent) {
        e.stopPropagation();
        if (
            node.kind === "table" ||
            node.kind === "view" ||
            node.kind === "column"
        ) {
            // It's a "file"
            onAction("open", node);
        } else {
            // It's a "folder"
            onToggle();
        }
    }
</script>

<ContextMenu.Root>
    <ContextMenu.Trigger>
        <div
            class={cn(
                "flex items-center h-6 select-none cursor-default hover:bg-accent/50 text-foreground/80 group",
                expanded && "text-foreground",
                node.disabled && "opacity-50 pointer-events-none",
            )}
            style="padding-left: {depth * 16 + 4}px"
            onclick={handleRowClick}
            role="button"
            tabindex="0"
            onkeydown={(e) => {
                if (e.key === "Enter" || e.key === " ") {
                    e.preventDefault();
                    handleRowClick(e as unknown as MouseEvent);
                }
            }}
        >
            <!-- Twistie / Chevron -->
            <span
                class="w-4 h-4 flex items-center justify-center shrink-0 mr-1 text-muted-foreground/50 hover:text-foreground cursor-pointer"
                onclick={(e) => {
                    e.stopPropagation();
                    onToggle();
                }}
            >
                {#if isSpinner}
                    <Loader2 class="w-3 h-3 animate-spin text-primary" />
                {:else if isExpandable || node.kind === "table"}
                    <!-- Always show twistie for expandable things, including tables if they have columns -->
                    <ChevronRight
                        class={cn(
                            "w-3.5 h-3.5 transition-transform",
                            expanded && "rotate-90",
                        )}
                    />
                {/if}
            </span>

            <!-- Type Icon -->
            <span
                class="w-4 h-4 flex items-center justify-center shrink-0 mr-1.5 text-muted-foreground/80"
            >
                <Icon class="w-3.5 h-3.5" />
            </span>

            <!-- Label -->
            <span class="truncate text-[13px] leading-none font-medium">
                {node.label}
            </span>

            <!-- Count Badge (optional) -->
            {#if node.childCount !== undefined}
                <span class="ml-1.5 text-[11px] text-muted-foreground/60"
                    >{node.childCount}</span
                >
            {/if}

            <!-- Hover Actions -->
            <div
                class="ml-auto flex items-center pr-2 opacity-0 group-hover:opacity-100 bg-gradient-to-l from-background to-transparent pl-4"
            >
                {#if node.kind === "database" || node.kind === "schema" || node.kind === "table"}
                    <button
                        class="p-0.5 hover:bg-accent rounded text-muted-foreground hover:text-foreground transition-colors"
                        title="Refresh"
                        onclick={(e) => {
                            e.stopPropagation();
                            onRefresh();
                        }}
                    >
                        <Refresh class="w-3 h-3" />
                    </button>
                {/if}
            </div>
        </div>
    </ContextMenu.Trigger>

    <ExplorerContextMenu {node} {onAction} />
</ContextMenu.Root>
