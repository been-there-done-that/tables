<script lang="ts">
    import IconSend from "@tabler/icons-svelte/icons/send";
    import IconPlayerStop from "@tabler/icons-svelte/icons/player-stop-filled";

    interface Props {
        onSend: (text: string) => void;
        onStop: () => void;
        running: boolean;
        disabled?: boolean;
    }

    let { onSend, onStop, running, disabled = false }: Props = $props();

    let text = $state("");
    let textarea: HTMLTextAreaElement;

    function send() {
        const trimmed = text.trim();
        if (!trimmed || running || disabled) return;
        onSend(trimmed);
        text = "";
        resize();
    }

    function resize() {
        if (!textarea) return;
        textarea.style.height = "auto";
        textarea.style.height = Math.min(textarea.scrollHeight, 150) + "px";
    }

    function onKeydown(e: KeyboardEvent) {
        if (e.key === "Enter" && !e.shiftKey) {
            e.preventDefault();
            send();
        }
    }
</script>

<div class="shrink-0 border-t border-border p-3">
    <div
        class="flex items-end gap-2 rounded-2xl border border-border bg-muted/20 px-3 py-2 focus-within:border-accent/40 focus-within:ring-1 focus-within:ring-accent/20"
    >
        <textarea
            bind:this={textarea}
            bind:value={text}
            oninput={resize}
            onkeydown={onKeydown}
            placeholder="Ask anything about your database…"
            rows={1}
            {disabled}
            class="flex-1 resize-none bg-transparent text-[13px] text-foreground placeholder:text-muted-foreground focus:outline-none disabled:opacity-50"
            style="max-height: 150px; overflow-y: auto;"
        ></textarea>

        {#if running}
            <button
                onclick={onStop}
                class="shrink-0 rounded-lg bg-destructive/80 p-1.5 text-white transition-colors hover:bg-destructive"
                title="Stop generation"
            >
                <IconPlayerStop size={14} />
            </button>
        {:else}
            <button
                onclick={send}
                disabled={!text.trim() || disabled}
                class="shrink-0 rounded-lg bg-accent p-1.5 text-white transition-colors hover:bg-accent-hover disabled:cursor-not-allowed disabled:opacity-40"
                title="Send (Enter)"
            >
                <IconSend size={14} />
            </button>
        {/if}
    </div>
</div>
