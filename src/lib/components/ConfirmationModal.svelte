<script lang="ts">
    import DraggableWindow from "./DraggableWindow.svelte";
    import Button from "./Button.svelte";
    import { cn } from "$lib/utils";

    let {
        open = $bindable(false),
        title = "Confirm Action",
        message = "Are you sure you want to proceed?",
        confirmText = "Confirm",
        cancelText = "Cancel",
        variant = "danger" as "danger" | "warning" | "info",
        isLoading = false,
        onConfirm = () => {},
        onCancel = () => {},
    } = $props();

    function handleConfirm() {
        if (onConfirm) onConfirm();
    }

    function handleCancel() {
        open = false;
        if (onCancel) onCancel();
    }

    function handleKeyDown(e: KeyboardEvent) {
        if (!open) return;

        if (e.key === "Escape") {
            e.preventDefault();
            e.stopPropagation();
            handleCancel();
        } else if (e.key === "Enter") {
            e.preventDefault();
            e.stopPropagation();
            handleConfirm();
        }
    }
</script>

<svelte:window onkeydown={handleKeyDown} />

<DraggableWindow
    bind:open
    {title}
    modal={true}
    initialPosition="center"
    class="max-w-sm! w-full! h-auto! max-h-none"
    headerClass={cn(
        variant === "danger" && "bg-red-500/10 border-red-500/20 text-red-500",
        variant === "warning" &&
            "bg-amber-500/10 border-amber-500/20 text-amber-500",
    )}
    showCloseButton={true}
    onClose={handleCancel}
>
    <div class="p-4 flex flex-col gap-6">
        <p class="text-sm text-[--theme-fg-secondary] leading-relaxed">
            {message}
        </p>

        <div class="flex justify-end gap-3">
            <Button variant="ghost" onClick={handleCancel}>
                {cancelText}
            </Button>
            <Button
                variant="solid"
                class={cn(
                    variant === "danger" &&
                        "bg-red-600 hover:bg-red-700 text-white",
                    variant === "warning" &&
                        "bg-amber-600 hover:bg-amber-700 text-white",
                )}
                onClick={handleConfirm}
                disabled={isLoading}
            >
                {isLoading ? "Processing..." : confirmText}
            </Button>
        </div>
    </div>
</DraggableWindow>
