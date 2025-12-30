<script lang="ts">
    import { onMount, onDestroy, tick } from "svelte";
    import { computePosition, flip, shift, offset } from "@floating-ui/dom";

    interface Props {
        value: any;
        onCommit: (newValue: any) => void;
        onCancel: () => void;
    }

    let { value, onCommit, onCancel }: Props = $props();

    let triggerRef = $state<HTMLDivElement | null>(null);
    let floatingRef = $state<HTMLDivElement | null>(null);
    let textareaRef = $state<HTMLTextAreaElement | null>(null);

    let textValue = $state("");

    onMount(() => {
        textValue = value || "";
        updatePosition();

        tick().then(() => {
            if (textareaRef) {
                textareaRef.focus();
                textareaRef.setSelectionRange(0, textareaRef.value.length);
            }
        });

        const handleUpdate = () => updatePosition();

        window.addEventListener("scroll", handleUpdate, true);
        window.addEventListener("resize", handleUpdate);
        window.addEventListener("keydown", handleKeydown);

        setTimeout(() => {
            document.addEventListener("mousedown", handleClickOutside);
        }, 100);

        return () => {
            window.removeEventListener("scroll", handleUpdate, true);
            window.removeEventListener("resize", handleUpdate);
            window.removeEventListener("keydown", handleKeydown);
        };
    });

    onDestroy(() => {
        document.removeEventListener("mousedown", handleClickOutside);
    });

    function handleClickOutside(event: MouseEvent) {
        if (
            triggerRef &&
            !triggerRef.contains(event.target as Node) &&
            floatingRef &&
            !floatingRef.contains(event.target as Node)
        ) {
            if (textValue !== (value || "")) {
                onCommit(textValue);
            } else {
                onCancel();
            }
        }
    }

    function handleKeydown(e: KeyboardEvent) {
        if (e.key === "Escape") {
            e.preventDefault();
            onCancel();
        } else if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) {
            e.preventDefault();
            if (textValue !== (value || "")) {
                onCommit(textValue);
            } else {
                onCancel();
            }
        }
    }

    async function updatePosition() {
        if (!triggerRef || !floatingRef) return;

        const { x, y } = await computePosition(triggerRef, floatingRef, {
            placement: "bottom-start",
            middleware: [offset(4), flip(), shift({ padding: 8 })],
        });

        floatingRef.style.left = `${x}px`;
        floatingRef.style.top = `${y}px`;
    }
</script>

<div
    bind:this={triggerRef}
    class="w-full h-full px-2 py-1 cursor-pointer truncate"
>
    {value || ""}
</div>

<div
    bind:this={floatingRef}
    class="fixed bg-popover border rounded-md shadow-lg p-2 flex flex-col gap-2"
    style="z-index: 9999; min-width: 400px; max-width: 600px; top: 0; left: 0;"
>
    <textarea
        bind:this={textareaRef}
        bind:value={textValue}
        class="flex min-h-[150px] w-full rounded-md border border-input bg-transparent px-3 py-2 text-sm shadow-sm placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring font-mono resize"
        placeholder="Enter text..."
        autocomplete="off"
        autocapitalize="off"
        spellcheck="false"
    ></textarea>

    <div class="text-[10px] text-muted-foreground px-1 flex justify-between">
        <span>{textValue.length} characters</span>
        <span>Cmd+Enter to save, Esc to cancel</span>
    </div>
</div>

