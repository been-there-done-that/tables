<!-- src/lib/components/agent/AgentComposer.svelte -->
<script lang="ts">
    import { Editor } from "@tiptap/core";
    import StarterKit from "@tiptap/starter-kit";
    import { Placeholder } from "@tiptap/extension-placeholder";

    interface Props {
        onSend: (text: string, rawDoc: unknown) => void;
        onStop: () => void;
        running: boolean;
        disabled: boolean;
    }

    let { onSend, onStop, running, disabled }: Props = $props();

    let editorEl: HTMLDivElement;
    let editor: Editor | null = $state(null);

    $effect(() => {
        if (!editorEl) return;
        const e = new Editor({
            element: editorEl,
            extensions: [
                StarterKit.configure({ history: true }),
                Placeholder.configure({ placeholder: "Ask Claude about your database..." }),
            ],
            editorProps: {
                attributes: {
                    class: "focus:outline-none min-h-[36px] max-h-[150px] overflow-y-auto text-sm text-foreground leading-relaxed",
                },
                handleKeyDown(_view, event) {
                    if (event.key === "Enter" && !event.shiftKey) {
                        event.preventDefault();
                        handleSend();
                        return true;
                    }
                    return false;
                },
            },
        });
        editor = e;
        return () => {
            e.destroy();
            editor = null;
        };
    });

    $effect(() => {
        if (!editor) return;
        editor.setEditable(!disabled && !running);
    });

    function handleSend() {
        if (!editor || running || disabled) return;
        const text = editor.getText({ blockSeparator: "\n" }).trim();
        if (!text) return;
        const doc = editor.getJSON();
        onSend(text, doc);
        editor.commands.clearContent();
    }

    export function insertContent(content: unknown) {
        editor?.commands.insertContent(content as any);
        editor?.commands.focus();
    }
</script>

<div class="border-t border-border/50 p-2 flex flex-col gap-1.5">
    <div
        bind:this={editorEl}
        class="rounded-md border border-input bg-background px-3 py-2 text-sm"
    ></div>
    <div class="flex justify-end">
        {#if running}
            <button
                onclick={onStop}
                class="rounded px-3 py-1 text-xs bg-destructive/10 text-destructive hover:bg-destructive/20"
            >
                Stop
            </button>
        {:else}
            <button
                onclick={handleSend}
                disabled={disabled}
                class="rounded px-3 py-1 text-xs bg-primary text-primary-foreground hover:bg-primary/90 disabled:opacity-50"
            >
                Send
            </button>
        {/if}
    </div>
</div>
