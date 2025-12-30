<script lang="ts">
  import type { Column, SortState } from "./types";
  import { cn } from "$lib/utils";
  import { ArrowUp, ArrowDown, Filter, MoreHorizontal } from "lucide-svelte";
  import * as Tooltip from "$lib/components/ui/tooltip";
  import * as ContextMenu from "$lib/components/ui/context-menu";
  import * as Popover from "$lib/components/ui/popover";
  import { Button } from "$lib/components/ui/button";
  import HeaderContextMenu from "./HeaderContextMenu.svelte";
  import LocalFilter from "./LocalFilter.svelte";
  import Grid4x4Icon from "@tabler/icons-svelte/icons/grid-4x4";
  import GridGoldenRatioIcon from "@tabler/icons-svelte/icons/grid-goldenratio";
  import Hash from "lucide-svelte/icons/hash";

  interface Props {
    columns: Column[];
    sortState: SortState[];
    filters: Record<string, any>;
    onSort: (
      columnId: string,
      multi: boolean,
      direction?: "asc" | "desc",
    ) => void;
    onClearSort?: (columnId: string) => void;
    onOpenInQueryEditor?: (columnId: string) => void;
    onOpenNewQueryTab?: (columnId: string) => void;
    onFilter: (columnId: string, value: any) => void;
    onResize?: (columnId: string, newWidth: number) => void;
    onHideColumn?: (columnId: string) => void;
    onHideOtherColumns?: (columnId: string) => void;
    onShowColumnList?: () => void;
    getUniqueValues?: (columnId: string) => { value: any; count: number }[];
  }

  let {
    columns,
    sortState,
    filters,
    onSort,
    onClearSort,
    onOpenInQueryEditor,
    onOpenNewQueryTab,
    onFilter,
    onResize,
    onHideColumn,
    onHideOtherColumns,
    onShowColumnList,
    getUniqueValues,
  }: Props = $props();

  function getSortIcon(columnId: string) {
    const sort = sortState.find((s) => s.columnId === columnId);
    if (!sort) return null;
    return sort.direction === "asc" ? ArrowUp : ArrowDown;
  }

  function getSortIndex(columnId: string) {
    if (sortState.length <= 1) return null;
    const index = sortState.findIndex((s) => s.columnId === columnId);
    return index >= 0 ? index + 1 : null;
  }

  // Resizing logic
  let resizingColumnId = $state<string | null>(null);
  let startX = 0;
  let startWidth = 0;

  function handleMouseDown(e: MouseEvent, column: Column) {
    resizingColumnId = column.id;
    startX = e.clientX;
    startWidth = column.width || 150; // Default width

    window.addEventListener("mousemove", handleMouseMove);
    window.addEventListener("mouseup", handleMouseUp);
  }

  function handleMouseMove(e: MouseEvent) {
    if (!resizingColumnId) return;
    const diff = e.clientX - startX;
    const newWidth = Math.max(50, startWidth + diff); // Min width 50px
    onResize?.(resizingColumnId, newWidth);
  }

  function handleMouseUp() {
    resizingColumnId = null;
    window.removeEventListener("mousemove", handleMouseMove);
    window.removeEventListener("mouseup", handleMouseUp);
  }

  // Filter state
  let openFilterColumnId = $state<string | null>(null);

  // Context menu state
  // Keeps track of which column's header context menu is currently open so that
  // only one context menu can be displayed at any given time.
  let openContextColumnId = $state<string | null>(null);

  function handleCopyName(name: string) {
    navigator.clipboard.writeText(name);
  }
</script>

<div class="flex select-none text-sm font-medium text-muted-foreground">
  <!-- Row Number Header -->
  <div
    class="sticky left-0 z-20 flex items-center justify-center border-r bg-background py-1.5"
    style="width: 60px; flex-shrink: 0;"
  >
    <Hash class="size-4" />
  </div>

  <Tooltip.Provider>
    {#each columns as column (column.id)}
      {@const SortIcon = getSortIcon(column.id)}
      {@const sortIndex = getSortIndex(column.id)}
      {@const isFilterOpen = openFilterColumnId === column.id}

      <ContextMenu.Root
          open={openContextColumnId === column.id}
          onOpenChange={(open) => {
            if (open) {
              openContextColumnId = column.id;
            } else if (openContextColumnId === column.id) {
              openContextColumnId = null;
            }
          }}
        >
        <ContextMenu.Trigger>
          <div
            class="relative flex items-center border-r px-2 py-1.5 hover:bg-muted/80 transition-colors group"
            style="width: {column.width ||
              150}px; min-width: {column.minWidth ||
              50}px; max-width: {column.maxWidth}px; flex-shrink: 0;"
          >
            <!-- Label & Sort -->
            <Tooltip.Root>
              <!-- svelte-ignore a11y_no_static_element_interactions -->
              <Tooltip.Trigger
                class="flex flex-1 items-center gap-1 truncate text-left focus:outline-none cursor-pointer"
                onclick={(e) => onSort(column.id, e.shiftKey)}
              >
                <span class="truncate">{column.label}</span>
                {#if SortIcon}
                  <SortIcon class="size-3.5" />
                  {#if sortIndex}
                    <span class="text-[10px]">{sortIndex}</span>
                  {/if}
                {/if}
              </Tooltip.Trigger>
              <Tooltip.Content>
                <p>{column.label}</p>
              </Tooltip.Content>
            </Tooltip.Root>

            <!-- Actions (Filter) -->
            <div
              class={cn(
                "flex items-center transition-opacity",
                filters[column.id] || isFilterOpen
                  ? "opacity-100"
                  : "opacity-0 group-hover:opacity-100",
              )}
            >
              {#if column.filterable}
                <Popover.Root
                  open={isFilterOpen}
                  onOpenChange={(open) => {
                    openFilterColumnId = open ? column.id : null;
                  }}
                >
                  <Popover.Trigger>
                    <Button
                      variant="ghost"
                      size="icon"
                      class={cn(
                        "h-6 w-6",
                        filters[column.id] && "text-primary",
                      )}
                    >
                      <Filter class="size-3" />
                      {#if filters[column.id]}
                        <span class="absolute -top-1 -right-1 h-2 w-2 rounded-full bg-primary"></span>
                      {/if}
                    </Button>
                  </Popover.Trigger>
                  <Popover.Content
                    class="p-0 w-[340px] max-w-[min(90vw,900px)] max-h-[min(80vh,640px)] overflow-auto resize"
                    align="start"
                  >
                    <LocalFilter
                      {column}
                      currentFilter={filters[column.id]}
                      uniqueValues={getUniqueValues?.(column.id)}
                      onFilterChange={(val) => onFilter(column.id, val)}
                      onClose={() => (openFilterColumnId = null)}
                    />
                  </Popover.Content>
                </Popover.Root>
              {/if}
            </div>

            <!-- Resizer -->
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div
              class="absolute right-0 top-0 bottom-0 w-1 cursor-col-resize hover:bg-primary/50 active:bg-primary z-10"
              onmousedown={(e) => handleMouseDown(e, column)}
            ></div>
          </div>
        </ContextMenu.Trigger>

        <HeaderContextMenu
          {column}
          onCopyName={() => handleCopyName(column.label)}
          onSelectColumn={() => {
            /*TODO*/
          }}
          onHideColumn={() => onHideColumn?.(column.id)}
          onHideOtherColumns={() => onHideOtherColumns?.(column.id)}
          onShowColumnList={() => onShowColumnList?.()}
          onSortAsc={() => onSort(column.id, false, "asc")}
          onSortDesc={() => onSort(column.id, false, "desc")}
          onClearSort={() => onClearSort?.(column.id)}
          onOpenInQueryEditor={() => onOpenInQueryEditor?.(column.id)}
          onOpenNewQueryTab={() => onOpenNewQueryTab?.(column.id)}
          onSetLocalFilter={() => {
            openFilterColumnId = column.id;
          }}
        />
      </ContextMenu.Root>
    {/each}
  </Tooltip.Provider>
</div>
