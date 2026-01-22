<script lang="ts">
    import {
        IconEdit,
        IconCopy,
        IconClipboard,
        IconBan,
        IconRestore,
        IconPlayerStop,
    } from "@tabler/icons-svelte";
    import * as ContextMenuPrimitive from "$lib/components/ui/context-menu";

    import { onMount, tick } from "svelte";

    interface Props {
        x: number;
        y: number;
        onEdit: () => void;
        onCopy: () => void;
        onPaste: () => void;
        onSetNull: () => void;
        onSetDefault: () => void;
        onDeleteRow: () => void;
        onClose: () => void;
    }

    let {
        x,
        y,
        onEdit,
        onCopy,
        onPaste,
        onSetNull,
        onSetDefault,
        onDeleteRow,
        onClose,
    }: Props = $props();

    let menuEl: HTMLDivElement;

    function handleClickOutside(node: HTMLElement) {
        const handleClick = (e: MouseEvent) => {
            if (node && !node.contains(e.target as Node)) {
                onClose();
            }
        };

        // Use mousedown to catch clicks before they might trigger other things
        document.addEventListener("mousedown", handleClick, true);

        return {
            destroy() {
                document.removeEventListener("mousedown", handleClick, true);
            },
        };
    }

    function focusAction(node: HTMLElement) {
        tick().then(() => {
            node.focus();
            // Second pass for robustness in case layout shift happened
            if (document.activeElement !== node && document.contains(node)) {
                requestAnimationFrame(() => node.focus());
            }
        });
    }

    function handleKeyDown(e: KeyboardEvent) {
        // Important: Stop propagation so global listeners (like table or window) don't intercept
        e.stopPropagation();

        if (e.key === "Escape") {
            e.preventDefault();
            onClose();
            return;
        }

        const buttons = Array.from(menuEl?.querySelectorAll("button") || []);
        const index = buttons.indexOf(
            document.activeElement as HTMLButtonElement,
        );

        if (e.key === "ArrowDown") {
            e.preventDefault();
            const nextIndex = (index + 1) % buttons.length;
            buttons[nextIndex]?.focus();
        } else if (e.key === "ArrowUp") {
            e.preventDefault();
            const prevIndex = (index - 1 + buttons.length) % buttons.length;
            buttons[prevIndex]?.focus();
        } else if (e.key === "Tab") {
            // Trap focus roughly
            e.preventDefault();
            if (e.shiftKey) {
                const prevIndex = (index - 1 + buttons.length) % buttons.length;
                buttons[prevIndex]?.focus();
            } else {
                const nextIndex = (index + 1) % buttons.length;
                buttons[nextIndex]?.focus();
            }
        }
    }
</script>

<div
    bind:this={menuEl}
    use:handleClickOutside
    class="fixed z-50 min-w-32 overflow-hidden rounded-md border border-(--theme-border-default) bg-(--theme-bg-secondary) p-1 text-(--theme-fg-default) shadow-md w-48"
    style="top: {y}px; left: {x}px;"
    role="menu"
    tabindex="-1"
    oncontextmenu={(e) => {
        e.preventDefault();
        e.stopPropagation();
    }}
    onkeydown={handleKeyDown}
>
    <button
        use:focusAction
        class="relative flex w-full cursor-pointer select-none items-center rounded-sm px-2 py-1.5 text-xs outline-none hover:bg-(--theme-bg-hover) hover:text-(--theme-fg-default) focus:bg-(--theme-bg-hover) focus:text-(--theme-fg-default) data-disabled:pointer-events-none data-disabled:opacity-50"
        onclick={onEdit}
    >
        <IconEdit class="mr-2 size-3.5" />
        Edit
    </button>
    <button
        class="relative flex w-full cursor-pointer select-none items-center rounded-sm px-2 py-1.5 text-xs outline-none hover:bg-(--theme-bg-hover) hover:text-(--theme-fg-default) focus:bg-(--theme-bg-hover) focus:text-(--theme-fg-default) data-disabled:pointer-events-none data-disabled:opacity-50"
        onclick={onCopy}
    >
        <IconCopy class="mr-2 size-3.5" />
        Copy
    </button>
    <button
        class="relative flex w-full cursor-pointer select-none items-center rounded-sm px-2 py-1.5 text-xs outline-none hover:bg-(--theme-bg-hover) hover:text-(--theme-fg-default) focus:bg-(--theme-bg-hover) focus:text-(--theme-fg-default) data-disabled:pointer-events-none data-disabled:opacity-50"
        onclick={onPaste}
    >
        <IconClipboard class="mr-2 size-3.5" />
        Paste
    </button>

    <div class="-mx-1 my-1 h-px bg-(--theme-border-default)"></div>

    <button
        class="relative flex w-full cursor-pointer select-none items-center rounded-sm px-2 py-1.5 text-xs outline-none hover:bg-(--theme-bg-hover) hover:text-(--theme-fg-default) focus:bg-(--theme-bg-hover) focus:text-(--theme-fg-default) data-disabled:pointer-events-none data-disabled:opacity-50"
        onclick={onSetNull}
    >
        <IconBan class="mr-2 size-3.5" />
        Set NULL
    </button>
    <button
        class="relative flex w-full cursor-pointer select-none items-center rounded-sm px-2 py-1.5 text-xs outline-none hover:bg-(--theme-bg-hover) hover:text-(--theme-fg-default) focus:bg-(--theme-bg-hover) focus:text-(--theme-fg-default) data-disabled:pointer-events-none data-disabled:opacity-50"
        onclick={onSetDefault}
    >
        <IconRestore class="mr-2 size-3.5" />
        Set DEFAULT
    </button>
    <div class="-mx-1 my-1 h-px bg-(--theme-border-default)"></div>
    <button
        class="relative flex w-full cursor-pointer select-none items-center rounded-sm px-2 py-1.5 text-xs outline-none hover:bg-red-500/10 text-red-500 focus:bg-red-500/10 focus:text-red-500 data-disabled:pointer-events-none data-disabled:opacity-50"
        onclick={onDeleteRow}
    >
        <svg
            xmlns="http://www.w3.org/2000/svg"
            width="14"
            height="14"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
            class="mr-2"
        >
            <path stroke="none" d="M0 0h24v24H0z" fill="none" />
            <path d="M4 7l16 0" />
            <path d="M10 11l0 6" />
            <path d="M14 11l0 6" />
            <path d="M5 7l1 12a2 2 0 0 0 2 2h8a2 2 0 0 0 2 -2l1 -12" />
            <path d="M9 7v-3a1 1 0 0 1 1 -1h4a1 1 0 0 1 1 1v3" />
        </svg>
        Delete Row
    </button>
</div>
