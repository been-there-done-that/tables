<script lang="ts">
  import { cn } from "$lib/utils";
  import ArrowsDiagonal2 from "@tabler/icons-svelte/icons/arrows-diagonal-2";
  import X from "@tabler/icons-svelte/icons/x";
  import { onMount } from "svelte";

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
    openShortcut,
    closeShortcut,
    modal = false,
    closeOnOverlayClick = true,
    class: className = "",
    headerClass = "",
    contentClass = "",
    titleClass = "",
    overlayClass = "",
    initialPosition = "center" as Position | "center",
    style: customStyle = "",
    minWidth = 260,
    minHeight = 180,
    children,
    headerActions,
    debug = false,
  } = $props();

  const titleId = crypto.randomUUID?.() ?? `dialog-title-${Math.random().toString(36).slice(2)}`;

  let position = $state<Position>({ x: 0, y: 0 });
  let isDragging = $state(false);
  let isResizing = $state(false);
  let resizeDir = $state<{ x: -1 | 0 | 1; y: -1 | 0 | 1 } | null>(null);
  let size = $state<{ w: number; h: number } | null>(null);
  let userSized = $state(false);
  let dragStart = $state<Position>({ x: 0, y: 0 });
  let windowElement = $state<HTMLDivElement | null>(null);
  let rendered = $state(false);
  let dragSize = $state<{ w: number; h: number }>({ w: 0, h: 0 });
  let hasCustomPosition = $state(false);
  let resizeStart = $state<{ x: number; y: number }>({ x: 0, y: 0 });
  let resizeStartSize = $state<{ w: number; h: number }>({ w: 0, h: 0 });
  let resizeStartPos = $state<Position>({ x: 0, y: 0 });

  const clamp = (value: number, min: number, max: number) => Math.min(Math.max(value, min), max);

  const clampPositionToViewport = () => {
    if (!windowElement) return;
    const rect = windowElement.getBoundingClientRect();
    position = {
      x: clamp(position.x, 8, Math.max(8, window.innerWidth - rect.width - 8)),
      y: clamp(position.y, 40, Math.max(40, window.innerHeight - rect.height - 8)),
    };
    if (debug) console.log("[ResizableWindow] clamp position", { position, rect });
  };

  onMount(() => {
    const handleResize = () => {
      clampPositionToViewport();
    };
    window.addEventListener("resize", handleResize);
    return () => window.removeEventListener("resize", handleResize);
  });

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
    if (open && e.key === "Escape") {
      e.preventDefault();
      handleClose();
      return;
    }
    if (matchesShortcut(e, closeShortcutNorm)) {
      e.preventDefault();
      handleClose();
      return;
    }
    if (!open && matchesShortcut(e, openShortcutNorm)) {
      e.preventDefault();
      open = true;
    }
  }

  function handleElementKeydown(e: KeyboardEvent) {
    if (e.key !== "Tab" || !windowElement) return;
    const focusable = windowElement.querySelectorAll<HTMLElement>(
      'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])',
    );
    if (!focusable.length) return;
    const first = focusable[0];
    const last = focusable[focusable.length - 1];
    if (e.shiftKey && document.activeElement === first) {
      e.preventDefault();
      last.focus();
    } else if (!e.shiftKey && document.activeElement === last) {
      e.preventDefault();
      first.focus();
    }
  }

  function handleClose(force = false) {
    if (onClose) onClose();
    else open = false;
  }

  function handleMouseDown(e: MouseEvent) {
    if (e.target instanceof HTMLElement && e.target.closest("[data-drag-handle]")) {
      isDragging = true;
      dragStart = { x: e.clientX - position.x, y: e.clientY - position.y };
      if (windowElement) {
        const rect = windowElement.getBoundingClientRect();
        dragSize = { w: rect.width, h: rect.height };
      }
      if (debug) console.log("[ResizableWindow] drag start", { dragStart, position, dragSize });
      e.preventDefault();
    }
  }

  function handleResizeMouseDown(e: MouseEvent, dir: { x: -1 | 0 | 1; y: -1 | 0 | 1 }) {
    if (!windowElement) return;
    e.preventDefault();
    e.stopPropagation();
    const rect = windowElement.getBoundingClientRect();
    size = { w: rect.width, h: rect.height };
    resizeStart = { x: e.clientX, y: e.clientY };
    resizeStartSize = { w: rect.width, h: rect.height };
    resizeStartPos = { ...position };
    resizeDir = dir;
    isResizing = true;
    hasCustomPosition = true;
    if (debug) console.log("[ResizableWindow] resize start", { size, resizeStart, resizeDir });
  }

  function handleMouseMove(e: MouseEvent) {
    if (isDragging) {
      const nextX = e.clientX - dragStart.x;
      const nextY = e.clientY - dragStart.y;
      const marginX = 8;
      const minY = 40; // keep below titlebar/toolbar area
      const marginY = 8;
      const maxX = Math.max(marginX, window.innerWidth - (dragSize.w || 0) - marginX);
      const maxY = Math.max(minY, window.innerHeight - (dragSize.h || 0) - marginY);
      position = {
        x: Math.min(Math.max(marginX, nextX), maxX),
        y: Math.min(Math.max(minY, nextY), maxY),
      };
      if (debug) console.log("[ResizableWindow] dragging", { nextX, nextY, position, dragSize });
    } else if (isResizing && resizeDir && size && windowElement) {
      const deltaX = e.clientX - resizeStart.x;
      const deltaY = e.clientY - resizeStart.y;
      let nextWidth = resizeStartSize.w;
      let nextHeight = resizeStartSize.h;
      let nextX = resizeStartPos.x;
      let nextY = resizeStartPos.y;

      if (resizeDir.x === 1) {
        const maxWidth = window.innerWidth - resizeStartPos.x - 8;
        nextWidth = clamp(resizeStartSize.w + deltaX, minWidth, maxWidth);
      } else if (resizeDir.x === -1) {
        const newLeft = clamp(resizeStartPos.x + deltaX, 8, resizeStartPos.x + resizeStartSize.w - minWidth);
        nextWidth = clamp(resizeStartPos.x + resizeStartSize.w - newLeft, minWidth, window.innerWidth - 16);
        nextX = newLeft;
      }

      if (resizeDir.y === 1) {
        const maxHeight = window.innerHeight - resizeStartPos.y - 8;
        nextHeight = clamp(resizeStartSize.h + deltaY, minHeight, maxHeight);
      } else if (resizeDir.y === -1) {
        const newTop = clamp(resizeStartPos.y + deltaY, 40, resizeStartPos.y + resizeStartSize.h - minHeight);
        nextHeight = clamp(resizeStartPos.y + resizeStartSize.h - newTop, minHeight, window.innerHeight - 48);
        nextY = newTop;
      }

      size = { w: nextWidth, h: nextHeight };
      position = { x: nextX, y: nextY };
      userSized = true;
      if (debug)
        console.log("[ResizableWindow] resizing", {
          deltaX,
          deltaY,
          resizeDir,
          size,
          position,
          nextWidth,
          nextHeight,
          nextX,
          nextY,
        });
    }
  }

  function handleMouseUp() {
    isDragging = false;
    isResizing = false;
    resizeDir = null;
    if (debug) console.log("[ResizableWindow] pointer up", { position, size });
  }

  let seedKey = $state("");

  $effect(() => {
    const nextKey = open ? `${minWidth}-${minHeight}-${open ? "open" : "closed"}` : "closed";
    if (nextKey !== seedKey) {
      seedKey = nextKey;
      size = null;
      userSized = false;
    }
  });

  $effect(() => {
    if (open && windowElement && !hasCustomPosition) {
      requestAnimationFrame(() => {
        if (!windowElement) return;
        const rect = windowElement.getBoundingClientRect();
        if (!size) {
          size = {
            w: Math.max(rect.width, minWidth),
            h: Math.max(rect.height, minHeight),
          };
          userSized = true;
        }
        const widthForPos = size?.w ?? rect.width;
        const heightForPos = size?.h ?? rect.height;
        if (initialPosition === "center") {
          position = {
            x: (window.innerWidth - widthForPos) / 2,
            y: (window.innerHeight - heightForPos) / 2,
          };
        } else {
          position = {
            x: initialPosition.x ?? (window.innerWidth - widthForPos) / 2,
            y: initialPosition.y ?? (window.innerHeight - heightForPos) / 2,
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

<svelte:window onmousemove={handleMouseMove} onmouseup={handleMouseUp} onkeydown={handleKeyDown} />

{#if rendered}
  {#if modal}
    <div
      class={cn("fixed inset-0 bg-black/50 z-40 cursor-pointer modal-overlay", overlayClass)}
      role="presentation"
      aria-label="Close dialog"
      data-state={open ? "open" : "closed"}
      onclick={() => closeOnOverlayClick && handleClose()}
    ></div>
  {/if}

  <div
    bind:this={windowElement}
    role="dialog"
    aria-modal={modal}
    aria-labelledby={titleId}
    tabindex="-1"
    class={cn(
      "fixed z-50 rounded-lg border border-[color-mix(in_srgb,var(--theme-border-default)_75%,transparent)] bg-(--theme-bg-secondary) text-(--theme-fg-primary) shadow-2xl flex flex-col window-anim outline-none focus-visible:outline-none focus-visible:ring-0 focus-visible:ring-offset-0",
      className,
    )}
    style={`left: ${position.x}px; top: ${position.y}px; min-width: ${minWidth}px; min-height: ${minHeight}px; ${
      userSized && size ? `width: ${size.w}px; height: ${size.h}px;` : ""
    } ${customStyle}`}
    onmousedown={handleMouseDown}
    onclick={(e) => e.stopPropagation()}
    onkeydown={handleElementKeydown}
    onfocus={(e) => e.stopPropagation()}
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
      <h2 id={titleId} class={cn("font-semibold text-sm text-center w-full", titleClass)}>{title}</h2>
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

    <div class={cn("overflow-auto flex-1 min-h-[120px]", contentClass)}>{@render children?.()}</div>

    <button
      class="absolute bottom-1 right-1 size-8 grid place-items-center text-(--theme-fg-muted) hover:text-(--theme-fg-primary) rounded-md border border-dashed border-(--theme-border-default) bg-[color-mix(in_srgb,var(--theme-bg-secondary)_85%,transparent)] transition-colors active:scale-[0.98] shadow-sm"
      aria-label="Resize"
      onmousedown={(e) => handleResizeMouseDown(e, { x: 1, y: 1 })}
    >
      <ArrowsDiagonal2 class="size-4" />
    </button>
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

  @media (prefers-reduced-motion: reduce) {
    .modal-overlay {
      animation: none;
    }
    .window-anim {
      animation: none;
    }
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
