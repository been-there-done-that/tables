<script lang="ts">
  import SearchInput from "$lib/components/SearchInput.svelte";
  import { cn } from "$lib/utils";

  export type CommandItem = {
    id: string;
    label: string;
    section?: string;
    shortcut?: string;
    hint?: string;
    action?: () => void;
    keywords?: string;
    icon?: string;
  };

  let {
    open = $bindable(false),
    items = [] as CommandItem[],
    placeholder = "Type a command or search...",
    onSelect = (item: CommandItem) => item.action?.(),
  } = $props<{
    open?: boolean;
    items?: CommandItem[];
    placeholder?: string;
    onSelect?: (item: CommandItem) => void;
  }>();

  let query = $state("");
  let activeIndex = $state(0);
  let inputEl = $state<HTMLInputElement | null>(null);
  let normalized = $state<CommandItem[]>([]);
  let grouped = $state<[string, CommandItem[]][]>([]);
  let rendered = $state(false);

  $effect(() => {
    const q = query.trim().toLowerCase();
    const filtered: CommandItem[] = (items ?? []).filter((item: CommandItem) => {
      if (!q) return true;
      const hay = `${item.label} ${item.section ?? ""} ${item.keywords ?? ""}`.toLowerCase();
      return hay.includes(q);
    });
    normalized = filtered;

    const bySection = new Map<string, CommandItem[]>();
    filtered.forEach((item) => {
      const key = item.section ?? "Commands";
      if (!bySection.has(key)) bySection.set(key, []);
      bySection.get(key)!.push(item);
    });
    const sections: [string, CommandItem[]][] = Array.from(bySection.entries());
    grouped = sections;
  });

  function handleSelect(item: CommandItem) {
    onSelect(item);
    open = false;
    query = "";
    activeIndex = 0;
  }

  function onKeyDown(event: KeyboardEvent) {
    if (!open) return;
    const flat = normalized;
    if (event.key === "ArrowDown") {
      event.preventDefault();
      activeIndex = (activeIndex + 1) % Math.max(flat.length, 1);
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      activeIndex = activeIndex - 1 < 0 ? Math.max(flat.length - 1, 0) : activeIndex - 1;
    } else if (event.key === "Enter") {
      event.preventDefault();
      if (flat[activeIndex]) handleSelect(flat[activeIndex]);
    } else if (event.key === "Escape") {
      event.preventDefault();
      open = false;
      query = "";
      activeIndex = 0;
    }
  }

  $effect(() => {
    if (open) {
      rendered = true;
      activeIndex = 0;
      queueMicrotask(() => inputEl?.focus());
    } else {
      // keep rendered until CSS transition ends; see on:animationend
    }
  });
</script>

{#if rendered}
  <div
    class="fixed inset-0 z-50 flex items-start justify-center bg-black/40 p-4 pt-[24vh] palette-overlay"
    role="presentation"
    tabindex="-1"
    data-state={open ? "open" : "closed"}
    onkeydown={(e) => {
      if (e.key === "Escape") {
        e.preventDefault();
        open = false;
        return;
      }
      if (e.target === e.currentTarget && (e.key === "Enter" || e.key === " ")) {
        e.preventDefault();
        open = false;
        return;
      }
      onKeyDown(e);
    }}
    onclick={(e) => {
      if (e.target === e.currentTarget) {
        open = false;
      }
    }}
  >
    <div
      class="w-full max-w-xl rounded-xl border border-(--theme-border-default) bg-(--theme-bg-secondary) shadow-2xl overflow-hidden palette-anim"
      role="dialog"
      aria-modal="true"
      aria-label="Command palette"
      tabindex="-1"
      onclick={(e) => e.stopPropagation()}
      onkeydown={onKeyDown}
      data-state={open ? "open" : "closed"}
      onanimationend={(e) => {
        if (!open && e.target === e.currentTarget) {
          rendered = false;
        }
      }}
    >
      <div class="border-b border-(--theme-border-subtle) p-2.5">
        <SearchInput
          bind:inputRef={inputEl}
          bind:value={query}
          placeholder={placeholder}
          class="h-9 bg-(--theme-bg-tertiary) border border-(--theme-border-subtle)"
        />
      </div>
      <div class="max-h-[60vh] overflow-auto">
        {#if normalized.length === 0}
          <div class="p-4 text-sm text-(--theme-fg-secondary)">No results</div>
        {:else}
          {#each grouped as [section, sectionItems], i}
            <div class="px-3 py-2 space-y-2">
              <div class="text-[11px] font-semibold uppercase tracking-wide text-(--theme-fg-secondary)">{section}</div>
              <div class="space-y-1">
                {#each sectionItems as item}
                  {#if item}
                    <button
                      type="button"
                      class={cn(
                        "w-full text-left rounded-md border border-transparent px-3 py-2 flex items-center justify-between gap-3 transition text-sm",
                        activeIndex === normalized.findIndex((n) => n.id === item.id)
                          ? "bg-(--theme-bg-hover) border-(--theme-border-default)"
                          : "hover:bg-(--theme-bg-hover)/70"
                      )}
                      role="option"
                      aria-selected={activeIndex === normalized.findIndex((n) => n.id === item.id)}
                      onclick={() => handleSelect(item)}
                    >
                      <div class="flex items-center gap-2 min-w-0">
                        <div class="h-6 w-6 rounded-md border border-(--theme-border-subtle) flex items-center justify-center text-sm opacity-80">
                          {item.icon ?? "⌘"}
                        </div>
                        <div class="min-w-0">
                          <div class="text-sm font-medium truncate">{item.label}</div>
                          {#if item.hint}
                            <div class="text-xs text-(--theme-fg-secondary) truncate">{item.hint}</div>
                          {/if}
                        </div>
                      </div>
                      {#if item.shortcut}
                        <span class="text-[11px] text-(--theme-fg-secondary) border border-(--theme-border-subtle) rounded px-1.5 py-0.5">
                          {item.shortcut}
                        </span>
                      {/if}
                    </button>
                  {/if}
                {/each}
              </div>
            </div>
            {#if i < grouped.length - 1}
              <div class="border-t border-(--theme-border-subtle) mx-3"></div>
            {/if}
          {/each}
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .palette-overlay {
    animation: overlayFadeIn 240ms ease-out;
  }
  .palette-overlay[data-state="closed"] {
    animation: overlayFadeOut 180ms ease-in forwards;
  }
  .palette-anim {
    animation: fadeScaleIn 240ms cubic-bezier(0.22, 0.61, 0.36, 1);
    transform-origin: center;
  }
  .palette-anim[data-state="closed"] {
    animation: fadeScaleOut 180ms ease-in forwards;
  }
  @keyframes fadeScaleIn {
    from {
      opacity: 0;
      transform: translateY(10px) scale(0.96);
    }
    to {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
  }
  @keyframes fadeScaleOut {
    from {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
    to {
      opacity: 0;
      transform: translateY(6px) scale(0.98);
    }
  }
  @keyframes overlayFadeIn {
    from {
      opacity: 0;
    }
    to {
      opacity: 1;
    }
  }
  @keyframes overlayFadeOut {
    from {
      opacity: 1;
    }
    to {
      opacity: 0;
    }
  }
</style>
