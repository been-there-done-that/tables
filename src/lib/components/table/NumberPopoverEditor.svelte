<script lang="ts">
    import PopoverShell from "./PopoverShell.svelte";
    import IconCheck from "@tabler/icons-svelte/icons/check";
    import IconX from "@tabler/icons-svelte/icons/x";
    import IconChevronUp from "@tabler/icons-svelte/icons/chevron-up";
    import IconChevronDown from "@tabler/icons-svelte/icons/chevron-down";

    type NumberType = "int" | "float";

    interface Props {
        value: any;
        kind: NumberType;
        anchorEl?: HTMLElement | null;
        onCommit: (newValue: any) => void;
        onCancel: () => void;
    }

    let { value, kind, anchorEl, onCommit, onCancel }: Props = $props();

    let inputValue = $state("");

    $effect(() => {
        inputValue = (value ?? "").toString();
    });

    const isMac =
        typeof navigator !== "undefined" && navigator.userAgent.includes("Mac");

    const originalString = $derived((value ?? "").toString());

    function parseNumber(val: string) {
        if (kind === "int") {
            const n = parseInt(val, 10);
            return isNaN(n) ? null : n;
        }
        const n = parseFloat(val);
        return isNaN(n) ? null : n;
    }

    function handleKeydown(e: KeyboardEvent) {
        const isCmdEnter = (e.metaKey || e.ctrlKey) && e.key === "Enter";
        const isPlainEnter = e.key === "Enter";
        if (isPlainEnter || isCmdEnter) {
            e.preventDefault();
            commit();
        }
    }

    function commit() {
        const parsed = parseNumber(inputValue);
        if (inputValue.toString() === originalString) {
            onCancel();
            return;
        }
        onCommit(parsed);
    }
</script>

<PopoverShell {anchorEl} {onCancel} minWidth={220} maxWidth={280}>
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <div class="flex flex-col gap-1" role="group" onkeydown={handleKeydown}>
        <input
            type="number"
            inputmode="decimal"
            class="w-full rounded-md border border-accent/10 px-2 py-1.5 text-sm bg-background text-foreground focus:outline-none focus:ring-1 focus:ring-accent/10 focus:border-accent/10 transition-all [appearance:textfield] [&::-webkit-outer-spin-button]:appearance-none [&::-webkit-inner-spin-button]:appearance-none"
            bind:value={inputValue}
            placeholder="0"
        />

        <div class="flex items-center justify-center gap-2 px-1 pb-0.5">
            <button
                type="button"
                class="flex items-center gap-1.5 px-2 py-0.5 rounded border border-transparent hover:border-accent/10 hover:bg-muted text-foreground-muted transition-colors active:scale-95 group/btn"
                onclick={onCancel}
            >
                <span
                    class="text-[9px] font-medium px-1 rounded bg-black/5 dark:bg-white/5 border border-black/5 dark:border-white/5 text-foreground-muted/60"
                    >Esc</span
                >
                <IconX
                    class="size-3.5 opacity-60 group-hover/btn:opacity-100"
                />
            </button>

            <button
                type="button"
                class="flex items-center gap-1.5 px-2 py-0.5 rounded text-accent border border-transparent hover:border-accent/10 hover:bg-accent/10 transition-colors active:scale-95 group/btn"
                onclick={commit}
            >
                <span
                    class="text-[9px] font-medium px-1 rounded bg-accent/10 border border-accent/20 text-accent/80"
                    >{isMac ? "⌘↵" : "Ctrl↵"}</span
                >
                <IconCheck
                    class="size-3.5 opacity-80 group-hover/btn:opacity-100"
                />
            </button>

            <div class="flex items-center gap-1">
                <button
                    type="button"
                    class="p-1 rounded border border-transparent hover:border-accent/10 hover:bg-muted text-foreground-muted/60 hover:text-foreground-muted transition-colors active:scale-90"
                    onclick={() => {
                        const val = parseFloat(inputValue || "0");
                        inputValue = (val - 1).toString();
                    }}
                    title="Decrease"
                >
                    <IconChevronDown class="size-3.5" />
                </button>
                <button
                    type="button"
                    class="p-1 rounded border border-transparent hover:border-accent/10 hover:bg-muted text-foreground-muted/60 hover:text-foreground-muted transition-colors active:scale-90"
                    onclick={() => {
                        const val = parseFloat(inputValue || "0");
                        inputValue = (val + 1).toString();
                    }}
                    title="Increase"
                >
                    <IconChevronUp class="size-3.5" />
                </button>
            </div>
        </div>
    </div>
</PopoverShell>
