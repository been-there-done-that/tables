<script lang="ts">
  import { cn } from "$lib/utils";
  import X from "@tabler/icons-svelte/icons/x";
  import type { Snippet } from "svelte";

  type KeyboardShortcut = {
    key: string;
    ctrl?: boolean;
    meta?: boolean;
    shift?: boolean;
    alt?: boolean;
  };

  type Position = { x: number; y: number };

  let {
    title = "Window",
    open = $bindable(true),
    showCloseButton = true,
    onClose,
    openShortcut = undefined,
    closeShortcut = undefined,
    modal = false,
    class: className = "",
    headerClass = "",
    contentClass = "",
    titleClass = "",
    overlayClass = "",
    initialPosition = "center" as Position | "center",
    style: customStyle = "",
    children,
    headerActions = undefined,
  } = $props();

  let position = $state<Position>({ x: 0, y: 0 });
  let isDragging = $state(false);
  let dragStart = $state<Position>({ x: 0, y: 0 });
  let windowElement = $state<HTMLDivElement | null>(null);
  let rendered = $state(false);
  let dragSize = $state<{ w: number; h: number }>({ w: 0, h: 0 });
  let hasCustomPosition = $state(false);

  const normalizeShortcut = (
    shortcut?: KeyboardShortcut | string,
  ): KeyboardShortcut | undefined => {
    if (!shortcut) return undefined;
    if (typeof shortcut === "string") {
      const parts = shortcut
        .toLowerCase()
        .split("+")
        .map((p) => p.trim());
      return {
        key: parts[parts.length - 1],
        ctrl: parts.includes("ctrl"),
        meta: parts.includes("meta") || parts.includes("cmd"),
        shift: parts.includes("shift"),
        alt: parts.includes("alt"),
      };
    }
    return shortcut;
  };

  const openShortcutNorm = $derived(normalizeShortcut(openShortcut));
  const closeShortcutNorm = $derived(normalizeShortcut(closeShortcut));

  function matchesShortcut(e: KeyboardEvent, shortcut?: KeyboardShortcut) {
    if (!shortcut) return false;
    const keyMatch = e.key.toLowerCase() === shortcut.key.toLowerCase();
    const wantsModifier = shortcut.ctrl || shortcut.meta;
    const hasModifier = e.ctrlKey || e.metaKey;
    const modifierMatch = wantsModifier ? hasModifier : !hasModifier;
    const shiftMatch = shortcut.shift ? e.shiftKey : !e.shiftKey;
    const altMatch = shortcut.alt ? e.altKey : !e.altKey;
    return keyMatch && modifierMatch && shiftMatch && altMatch;
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      handleClose();
      return;
    }
    if (matchesShortcut(e, closeShortcutNorm)) {
      e.preventDefault();
      handleClose();
    }
  }

  function swallowKeyDown(e: KeyboardEvent) {
    e.stopPropagation();
  }

  function handleClose(force = false) {
    if (onClose) onClose();
    else open = false;
  }

  function handleMouseDown(e: MouseEvent) {
    if (
      e.target instanceof HTMLElement &&
      e.target.closest("[data-drag-handle]")
    ) {
      isDragging = true;
      dragStart = { x: e.clientX - position.x, y: e.clientY - position.y };
      if (windowElement) {
        const rect = windowElement.getBoundingClientRect();
        dragSize = { w: rect.width, h: rect.height };
      }
      e.preventDefault();
    }
  }

  function handleMouseMove(e: MouseEvent) {
    if (isDragging) {
      const nextX = e.clientX - dragStart.x;
      const nextY = e.clientY - dragStart.y;
      const marginX = 8;
      const minY = 40; // keep below titlebar/toolbar area
      const marginY = 8;
      const maxX = Math.max(
        marginX,
        window.innerWidth - (dragSize.w || 0) - marginX,
      );
      const maxY = Math.max(
        minY,
        window.innerHeight - (dragSize.h || 0) - marginY,
      );
      position = {
        x: Math.min(Math.max(marginX, nextX), maxX),
        y: Math.min(Math.max(minY, nextY), maxY),
      };
    }
  }

  function handleMouseUp() {
    isDragging = false;
  }

  $effect(() => {
    if (open && windowElement && !hasCustomPosition) {
      requestAnimationFrame(() => {
        if (!windowElement) return;
        const rect = windowElement.getBoundingClientRect();
        if (initialPosition === "center") {
          position = {
            x: (window.innerWidth - rect.width) / 2,
            y: (window.innerHeight - rect.height) / 2,
          };
        } else {
          position = {
            x: initialPosition.x ?? (window.innerWidth - rect.width) / 2,
            y: initialPosition.y ?? (window.innerHeight - rect.height) / 2,
          };
        }
      });
    }
  });

  $effect(() => {
    if (open) {
      rendered = true;
      queueMicrotask(() => windowElement?.focus());
    }
  });
</script>

<svelte:window
  onmousemove={handleMouseMove}
  onmouseup={handleMouseUp}
  onkeydown={handleKeyDown}
/>

{#if rendered}
  {#if modal}
    <div
      class={cn(
        "fixed inset-0 bg-black/50 z-40 cursor-pointer modal-overlay",
        overlayClass,
      )}
      role="presentation"
      aria-label="Close dialog"
      data-state={open ? "open" : "closed"}
      onclick={() => handleClose()}
    ></div>
  {/if}

  <div
    bind:this={windowElement}
    role="dialog"
    aria-modal={modal}
    aria-labelledby="dialog-title"
    tabindex="-1"
    class={cn(
      "fixed z-50 max-h-[75vh] max-w-3xl w-full rounded-lg border border-[color-mix(in_srgb,var(--theme-border-default)_75%,transparent)] bg-(--theme-bg-secondary) text-(--theme-fg-primary) shadow-2xl flex flex-col window-anim",
      className,
    )}
    style={`left: ${position.x}px; top: ${position.y}px; ${customStyle}`}
    onmousedown={handleMouseDown}
    onclick={(e) => e.stopPropagation()}
    onkeydown={swallowKeyDown}
    data-state={open ? "open" : "closed"}
    onanimationend={(e) => {
      if (!open && e.target === e.currentTarget) {
        rendered = false;
      }
    }}
  >
    <div
      data-drag-handle
      class={cn(
        "flex items-center justify-between pl-4 pr-2 py-1 rounded-t-lg cursor-move select-none border-b border-(--theme-border-default) bg-[color-mix(in_srgb,var(--theme-bg-secondary)_90%,transparent)]",
        headerClass,
      )}
    >
      <h2
        id="dialog-title"
        class={cn("font-semibold text-xs w-full", titleClass)}
      >
        {title}
      </h2>
      <div class="flex items-center gap-2">
        {@render headerActions?.()}
        {#if showCloseButton}
          <button
            onclick={() => handleClose()}
            class="hover:opacity-80 text-red-600 font-bold transition-colors p-1 border border-(--theme-border-default) rounded"
            aria-label="Close"
          >
            <X class="size-4" />
          </button>
        {/if}
      </div>
    </div>

    <div class={cn("overflow-auto", contentClass)}>{@render children?.()}</div>
  </div>
{/if}

<style>
  [data-drag-handle] {
    user-select: none;
    -webkit-user-select: none;
  }

  [data-drag-handle]:active {
    cursor: grabbing;
  }

  .modal-overlay {
    animation: overlayFadeIn 220ms ease-out;
  }
  .modal-overlay[data-state="closed"] {
    animation: overlayFadeOut 160ms ease-in forwards;
  }

  .window-anim {
    animation: windowFadeIn 220ms cubic-bezier(0.22, 0.61, 0.36, 1);
    transform-origin: center;
  }
  .window-anim[data-state="closed"] {
    animation: windowFadeOut 160ms ease-in forwards;
  }

  @keyframes windowFadeIn {
    from {
      opacity: 0;
      transform: translateY(10px) scale(0.96);
    }
    to {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
  }

  @keyframes windowFadeOut {
    from {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
    to {
      opacity: 0;
      transform: translateY(8px) scale(0.98);
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
