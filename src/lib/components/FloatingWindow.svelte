<script lang="ts">
  import { cn } from "$lib/utils";
  import X from "@tabler/icons-svelte/icons/x";
  import type { Snippet } from "svelte";
  import { scale } from "svelte/transition";

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
    showMaximizeButton = false,
    onClose,
    onMaximize,
    openShortcut,
    closeShortcut,
    modal = false,
    class: className = "",
    headerClass = "",
    contentClass = "",
    titleClass = "",
    overlayClass = "",
    initialPosition = "center" as Position | "center",
    style: customStyle = "",
    children,
    headerActions,
  } = $props();

  let position = $state<Position>({ x: 0, y: 0 });
  let isDragging = $state(false);
  let dragStart = $state<Position>({ x: 0, y: 0 });
  let windowElement = $state<HTMLDivElement | null>(null);

  const normalizeShortcut = (shortcut?: KeyboardShortcut | string): KeyboardShortcut | undefined => {
    if (!shortcut) return undefined;
    if (typeof shortcut === "string") {
      const parts = shortcut.toLowerCase().split("+").map((p) => p.trim());
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

  function handleClose() {
    if (onClose) onClose();
    else open = false;
  }

  function handleMaximize() {
    if (onMaximize) onMaximize();
  }

  function handleMouseDown(e: MouseEvent) {
    if (e.target instanceof HTMLElement && e.target.closest("[data-drag-handle]")) {
      isDragging = true;
      dragStart = { x: e.clientX - position.x, y: e.clientY - position.y };
      e.preventDefault();
    }
  }

  function handleMouseMove(e: MouseEvent) {
    if (isDragging) {
      position = { x: e.clientX - dragStart.x, y: e.clientY - dragStart.y };
    }
  }

  function handleMouseUp() {
    isDragging = false;
  }

  $effect(() => {
    if (open && windowElement) {
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
</script>

<svelte:window onmousemove={handleMouseMove} onmouseup={handleMouseUp} onkeydown={handleKeyDown} />

{#if open}
  {#if modal}
    <button
      type="button"
      class={cn("fixed inset-0 bg-black/50 z-40 border-0 cursor-pointer", overlayClass)}
      onclick={handleClose}
      aria-label="Close dialog"
    ></button>
  {/if}

  <div
    bind:this={windowElement}
    role="dialog"
    aria-modal={modal}
    aria-labelledby="dialog-title"
    tabindex="-1"
    in:scale={{ duration: 150, start: 0.95, opacity: 0 }}
    out:scale={{ duration: 120, start: 1, opacity: 0 }}
    class={cn(
      "fixed z-50 max-h-[75vh] max-w-3xl w-full rounded-lg border border-[color-mix(in_srgb,var(--theme-border-default)_75%,transparent)] bg-[var(--theme-bg-secondary)] text-[var(--theme-fg-primary)] shadow-2xl flex flex-col",
      className,
    )}
    style={`left: ${position.x}px; top: ${position.y}px; ${customStyle}`}
    onmousedown={handleMouseDown}
    onclick={(e) => e.stopPropagation()}
    onkeydown={swallowKeyDown}
  >
    <div
      data-drag-handle
      class={cn(
        "flex items-center justify-between pl-4 pr-2 py-1 rounded-t-lg cursor-move select-none border-b border-[var(--theme-border-default)] bg-[color-mix(in_srgb,var(--theme-bg-secondary)_90%,transparent)]",
        headerClass,
      )}
    >
      <h2 id="dialog-title" class={cn("font-semibold text-xs w-full", titleClass)}>{title}</h2>
      <div class="flex items-center gap-2">
        {@render headerActions?.()}
        {#if showMaximizeButton}
          <button
            class="hover:opacity-80 transition-colors p-1"
            aria-label="Maximize"
            onclick={handleMaximize}
          >
            ☐
          </button>
        {/if}
        {#if showCloseButton}
          <button
            onclick={handleClose}
            class="hover:opacity-80 text-red-600 font-bold transition-colors p-1"
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
</style>
