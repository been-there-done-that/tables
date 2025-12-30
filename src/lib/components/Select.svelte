<script lang="ts">
  import { getContext, onMount } from "svelte";
  import IconChevronDown from "@tabler/icons-svelte/icons/chevron-down";
  import { cn } from "$lib/utils";

  export type Option = { value: string; label: string };

  interface Props {
    value: any;
    options: Option[] | string[];
    placeholder?: string;
    disabled?: boolean;
    height?: "sm" | "md" | "lg";
    radius?: "sm" | "md" | "lg" | "full" | "none";
    minWidth?: number;
    widthBuffer?: number;
    onCommit: (newValue: any) => void;
    onCancel?: () => void;
    class?: string;
    id?: string;
  }

  let {
    value,
    options = [],
    placeholder = "Select",
    disabled = false,
    height = "sm",
    radius = "md",
    minWidth = 140,
    widthBuffer = 0,
    onCommit,
    onCancel = () => (open = false),
    class: className = "",
    id,
  }: Props = $props();

  const normalized = $derived(
    options.map((opt) =>
      typeof opt === "string" ? { value: opt, label: opt } : opt,
    ),
  );

  const minWidthValue = $derived(minWidth);
  const widthBufferValue = $derived(widthBuffer);

  let open = $state(false);
  let selectedIndex = $state(0);
  let triggerEl: HTMLButtonElement | null = null;
  let overlayEl = $state<HTMLElement | null>(null);
  let position = $state({ top: 0, left: 0, width: 0 });
  let isVisible = $state(false);

  const GUTTER = 4;
  const originalValue = $derived(value);

  const getContainer: (() => HTMLElement | null | undefined) | undefined =
    getContext("table-container");
  let containerEl: HTMLElement | null | undefined;

  function focusButton(idx: number) {
    if (!overlayEl) return;
    const buttons = overlayEl.querySelectorAll("button");
    buttons[idx]?.focus();
  }

  function portal(node: HTMLElement) {
    if (typeof document === "undefined") return {};
    const target = document.body;
    target.appendChild(node);
    return {
      destroy() {
        if (node.parentNode === target) target.removeChild(node);
      },
    };
  }

  let lastSyncedValue: any;
  $effect(() => {
    const idx = normalized.findIndex((opt) => opt.value === value);
    if (idx === -1) return;
    if (normalized[idx].value === lastSyncedValue) return;
    lastSyncedValue = normalized[idx].value;
    selectedIndex = idx;
    if (open) queueMicrotask(() => focusButton(idx));
  });

  $effect(() => {
    if (!isVisible || !open || normalized.length === 0) return;
    queueMicrotask(() => focusButton(selectedIndex));
  });

  function updatePosition() {
    if (!triggerEl || !triggerEl.isConnected) {
      onCancel();
      open = false;
      return;
    }
    const rect = triggerEl.getBoundingClientRect();
    const width = Math.max(rect.width + widthBufferValue, minWidthValue);
    const overlayHeight = overlayEl?.offsetHeight ?? rect.height;
    const margin = GUTTER;

    let left = rect.left;
    const fitsRight = left + width + margin <= window.innerWidth;
    if (!fitsRight) {
      left = Math.max(margin, window.innerWidth - width - margin);
    }
    left = Math.max(margin, Math.min(left, window.innerWidth - width - margin));

    let top = rect.bottom + margin;
    const maxTop = window.innerHeight - overlayHeight - margin;
    top = Math.min(top, maxTop);

    position = { top, left, width };
  }

  function handleSelect(newValue: any) {
    onCommit(newValue);
    open = false;
    isVisible = false;
    triggerEl?.focus();
  }

  function handleClickOutside(event: MouseEvent) {
    const target = event.target as Node;
    if (overlayEl?.contains(target)) return;
    if (triggerEl?.contains(target)) return;
    open = false;
    onCancel();
  }

  function handleOverlayKeydown(e: KeyboardEvent) {
    e.stopPropagation();
    if (!overlayEl) return;
    if (e.key === "Escape") {
      e.preventDefault();
      open = false;
      onCancel();
      triggerEl?.focus();
    } else if (e.key === "ArrowDown") {
      if (!normalized.length) return;
      e.preventDefault();
      selectedIndex = (selectedIndex + 1) % normalized.length;
      focusButton(selectedIndex);
    } else if (e.key === "ArrowUp") {
      if (!normalized.length) return;
      e.preventDefault();
      selectedIndex =
        (selectedIndex - 1 + normalized.length) % normalized.length;
      focusButton(selectedIndex);
    } else if (e.key === "Enter") {
      if (!normalized.length) return;
      e.preventDefault();
      handleSelect(normalized[selectedIndex].value);
    } else if (e.key === "Tab") {
      if (!normalized.length) return;
      e.preventDefault();
      const dir = e.shiftKey ? -1 : 1;
      selectedIndex =
        (selectedIndex + dir + normalized.length) % normalized.length;
      focusButton(selectedIndex);
    }
  }

  function toggleOpen() {
    if (disabled) return;
    open = !open;
    if (open) {
      requestAnimationFrame(updatePosition);
      const idx = normalized.findIndex((o) => o.value === value);
      selectedIndex = idx >= 0 ? idx : 0;
      queueMicrotask(() => {
        isVisible = true;
        focusButton(selectedIndex);
      });
    } else {
      isVisible = false;
    }
  }

  function handleTriggerKeydown(e: KeyboardEvent) {
    if (disabled) return;
    if ([" ", "Enter", "ArrowDown", "ArrowUp"].includes(e.key)) {
      e.preventDefault();
      if (!open) open = true;
      requestAnimationFrame(updatePosition);
      const idx = normalized.findIndex((o) => o.value === value);
      selectedIndex =
        idx >= 0 ? idx : e.key === "ArrowUp" ? normalized.length - 1 : 0;
      queueMicrotask(() => {
        isVisible = true;
        focusButton(selectedIndex);
      });
    }
  }

  onMount(() => {
    const handleUpdate = () => requestAnimationFrame(updatePosition);
    window.addEventListener("resize", handleUpdate);
    window.addEventListener("scroll", handleUpdate, true);
    document.addEventListener("mousedown", handleClickOutside);
    containerEl = getContainer?.();
    containerEl?.addEventListener("scroll", handleUpdate, { passive: true });

    return () => {
      window.removeEventListener("resize", handleUpdate);
      window.removeEventListener("scroll", handleUpdate, true);
      document.removeEventListener("mousedown", handleClickOutside);
      containerEl?.removeEventListener("scroll", handleUpdate);
    };
  });
</script>

<div class={cn("relative inline-block", className)}>
  <button
    bind:this={triggerEl}
    type="button"
    {id}
    class={cn(
      "w-full flex items-center justify-between border px-3 text-sm transition bg-(--theme-bg-secondary) text-(--theme-fg-primary) border-(--theme-border-default) hover:border-(--theme-accent-primary) focus:outline-none focus-visible:ring-2 focus-visible:ring-(--theme-accent-primary)",
      height === "sm" && "h-8",
      height === "md" && "h-10",
      height === "lg" && "h-12",
      radius === "sm" && "rounded-sm",
      radius === "md" && "rounded-md",
      radius === "lg" && "rounded-lg",
      radius === "full" && "rounded-full",
      radius === "none" && "rounded-none",
      disabled && "opacity-50 cursor-not-allowed",
    )}
    onclick={toggleOpen}
    onkeydown={handleTriggerKeydown}
    aria-expanded={open}
    aria-haspopup="listbox"
    style={`min-width:${minWidth}px`}
  >
    <span>
      {#if normalized.find((o) => o.value === value)}
        {normalized.find((o) => o.value === value)?.label}
      {:else}
        {placeholder}
      {/if}
    </span>
    <IconChevronDown class="size-4 opacity-70" />
  </button>
</div>

{#if open}
  <div
    use:portal
    bind:this={overlayEl}
    role="menu"
    aria-label="Select value"
    tabindex="-1"
    onkeydown={handleOverlayKeydown}
    class={cn(
      "fixed z-[1000] bg-(--theme-bg-secondary) border rounded-md border-(--theme-border-default) shadow-2xl",
      isVisible ? "opacity-100" : "opacity-0 pointer-events-none",
    )}
    style={`top:${position.top}px;left:${position.left}px;min-width:${position.width}px;transform-origin:center`}
    aria-hidden={!isVisible}
  >
    <div class="flex flex-col gap-1 p-1">
      {#each normalized as option, i}
        <button
          type="button"
          role="menuitemradio"
          aria-checked={selectedIndex === i}
          tabindex={selectedIndex === i ? 0 : -1}
          class={cn(
            "pl-2 pr-2 py-1.5 text-sm rounded text-left transition-colors flex items-center gap-1",
            selectedIndex === i
              ? "bg-[color-mix(in_srgb,var(--theme-accent-primary)_82%,var(--theme-bg-primary)_18%)] text-(--theme-bg-primary) font-semibold"
              : "hover:bg-[color-mix(in_srgb,var(--theme-accent-primary)_18%,var(--theme-bg-secondary)_82%)] hover:text-(--theme-fg-primary)",
          )}
          onclick={() => handleSelect(option.value)}
          onmouseenter={() => (selectedIndex = i)}
        >
          <span
            class={cn(
              "inline-block size-1 rounded-full mr-1",
              option.value === originalValue
                ? "bg-(--theme-accent-primary)"
                : "invisible",
            )}
            aria-hidden="true"
          ></span>
          {option.label}
        </button>
      {/each}
    </div>
  </div>
{/if}
