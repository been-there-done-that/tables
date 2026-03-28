<script lang="ts">
    import PopoverShell from "./PopoverShell.svelte";
    import IconCheck from "@tabler/icons-svelte/icons/check";
    import IconX from "@tabler/icons-svelte/icons/x";

    interface Props {
        value: any;
        anchorEl?: HTMLElement | null;
        onCommit: (newValue: any) => void;
        onCancel: () => void;
    }

    let { value, anchorEl, onCommit, onCancel }: Props = $props();

    let inputValue = $state("");

    const isMac =
        typeof navigator !== "undefined" && navigator.userAgent.includes("Mac");

    const originalString = $derived((value ?? "").toString());

    $effect(() => {
        inputValue = (value ?? "").toString();
    });

    function handleKeydown(e: KeyboardEvent) {
        // Standard Escape handled by shell, but we need Cmd+Enter
        const isCmdEnter = (e.metaKey || e.ctrlKey) && e.key === "Enter";
        if (isCmdEnter) {
            e.preventDefault();
            commit();
        }
    }

    function commit() {
        if (inputValue === originalString) {
            onCancel();
            return;
        }
        onCommit(inputValue);
    }
</script>

<PopoverShell {anchorEl} {onCancel} minWidth={260} maxWidth={400}>
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <div class="relative flex flex-col group" role="group" onkeydown={handleKeydown}>
        <textarea
            class="w-full rounded-md border border-accent/5 text-[13px] bg-background text-foreground min-h-[110px] resize-y p-1.5 pb-6 focus:ring-1 focus:ring-accent/10 focus:border-accent/10 focus:outline-none transition-all placeholder:text-foreground-muted/20"
            bind:value={inputValue}
            rows={4}
            placeholder="Edit text..."
        ></textarea>

        <div
            class="absolute bottom-1 left-0 right-0 flex items-center justify-center gap-2 pointer-events-none"
        >
            <button
                type="button"
                class="flex items-center gap-1.5 px-2 py-0.5 rounded border border-transparent hover:border-accent/10 hover:bg-muted text-foreground-muted transition-colors active:scale-95 group/btn pointer-events-auto"
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
                class="flex items-center gap-1.5 px-2 py-0.5 rounded text-accent border border-transparent hover:border-accent/10 hover:bg-accent/10 transition-colors active:scale-95 group/btn pointer-events-auto"
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
        </div>
    </div>
</PopoverShell>
