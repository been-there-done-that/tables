<script lang="ts">
    import {
        IconEdit,
        IconCopy,
        IconClipboard,
        IconBan,
        IconRestore,
        IconPlayerStop,
    } from "@tabler/icons-svelte";
    import { COPY_FORMAT_LABELS, type CopyFormat } from "./copyFormats";

    import { tick } from "svelte";

    interface Props {
        x: number;
        y: number;
        onEdit: () => void;
        onCopy: () => void;
        onCopyAs: (format: CopyFormat) => void;
        onPaste: () => void;
        onSetNull: () => void;
        onSetDefault: () => void;
        onRevertCell: () => void;
        onDeleteRow: () => void;
        onClose: () => void;
        isEditable?: boolean;
        hasPendingEdit?: boolean;
        isSingleColumn?: boolean;
    }

    let {
        x, y,
        onEdit, onCopy, onCopyAs, onPaste,
        onSetNull, onSetDefault, onRevertCell, onDeleteRow,
        onClose,
        isEditable = false,
        hasPendingEdit = false,
        isSingleColumn = false,
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
    class="fixed z-50 min-w-[200px] rounded-xl border border-border/50 bg-surface/95 p-1 text-foreground shadow-2xl backdrop-blur-xl animate-in fade-in zoom-in-95 duration-200"
    style="left: {x}px; top: {y}px;"
    role="menu"
    tabindex="-1"
    oncontextmenu={(e) => { e.preventDefault(); e.stopPropagation(); }}
    onkeydown={handleKeyDown}
>
    <!-- Copy (uses current format) -->
    <button use:focusAction class="menu-item" onclick={() => { onCopy(); onClose(); }}>
        <IconCopy class="mr-2 size-4 opacity-70" />
        <span class="flex-1 text-left">Copy</span>
        <span class="ml-auto text-[10px] text-muted-foreground opacity-60">⌘C</span>
    </button>

    <!-- Copy as submenu (hover reveals) -->
    <div class="relative group/copyAs">
        <button class="menu-item">
            <IconCopy class="mr-2 size-4 opacity-70" />
            <span class="flex-1 text-left">Copy as</span>
            <span class="ml-auto text-[10px] text-muted-foreground">▶</span>
        </button>
        <div class="absolute left-full top-0 hidden group-hover/copyAs:flex flex-col min-w-[160px] rounded-xl border border-border/50 bg-surface/95 p-1 shadow-2xl backdrop-blur-xl z-50">
            {#each Object.entries(COPY_FORMAT_LABELS) as [fmt, label]}
                {#if fmt !== "sql_in" || isSingleColumn}
                    <button
                        class="menu-item text-xs"
                        onclick={() => { onCopyAs(fmt as CopyFormat); onClose(); }}
                    >
                        {label}
                    </button>
                {/if}
            {/each}
        </div>
    </div>

    <div class="my-1 h-px bg-border/40"></div>

    <!-- Paste -->
    <button class="menu-item" onclick={() => { onPaste(); onClose(); }}>
        <IconClipboard class="mr-2 size-4 opacity-70" />
        <span class="flex-1 text-left">Paste</span>
        <span class="ml-auto text-[10px] text-muted-foreground opacity-60">⌘V</span>
    </button>

    {#if isEditable}
        <div class="my-1 h-px bg-border/40"></div>

        <button class="menu-item" onclick={() => { onSetNull(); onClose(); }}>
            <IconBan class="mr-2 size-4 opacity-70" />
            <span class="flex-1 text-left">Set NULL</span>
        </button>

        <button class="menu-item" onclick={() => { onSetDefault(); onClose(); }}>
            <IconRestore class="mr-2 size-4 opacity-70" />
            <span class="flex-1 text-left">Set Default</span>
        </button>

        {#if hasPendingEdit}
            <button class="menu-item" onclick={() => { onRevertCell(); onClose(); }}>
                <IconRestore class="mr-2 size-4 opacity-70" />
                <span class="flex-1 text-left">Revert cell</span>
            </button>
        {/if}

        <div class="my-1 h-px bg-border/40"></div>

        <button
            class="relative flex w-full cursor-pointer select-none items-center rounded-lg px-2.5 py-2 text-xs outline-none transition-colors hover:bg-red-500/10 text-red-500 focus:bg-red-500/10"
            onclick={() => { onDeleteRow(); onClose(); }}
        >
            <IconPlayerStop class="mr-2 size-4" />
            <span class="flex-1 text-left font-medium">Delete Row</span>
        </button>
    {/if}
</div>

<style>
    @reference "../../../app.css";

    .menu-item {
        @apply relative flex w-full cursor-pointer select-none items-center rounded-lg px-2.5 py-2 text-xs outline-none transition-colors hover:bg-accent/10 hover:text-accent focus:bg-accent/10 focus:text-accent;
    }
</style>
