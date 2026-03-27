<!-- src/lib/components/agent/AgentComposer.svelte -->
<script lang="ts">
    import { Editor } from "@tiptap/core";
    import StarterKit from "@tiptap/starter-kit";
    import { Placeholder } from "@tiptap/extension-placeholder";
    import { FileChipNode, TableChipNode, ResultChipNode } from "$lib/agent/composer-nodes";
    import ComposerDropdown from "./ComposerDropdown.svelte";
    import type { DropdownItem } from "./ComposerDropdown.svelte";
    import { composerStore } from "$lib/stores/composer.svelte";
    import { schemaStore } from "$lib/stores/schema.svelte";
    import { windowState } from "$lib/stores/window.svelte";

    interface Props {
        onSend: (text: string, rawDoc: unknown) => void;
        onStop: () => void;
        running: boolean;
        disabled: boolean;
    }

    let { onSend, onStop, running, disabled }: Props = $props();

    let editorEl: HTMLDivElement;
    let editor: Editor | null = $state(null);

    let dropdownVisible = $state(false);
    let dropdownItems = $state<DropdownItem[]>([]);
    let dropdownPos = $state({ x: 0, y: 0 });
    let triggerRange = $state<{ from: number; to: number } | null>(null);
    let dropdownRef: ComposerDropdown | null = $state(null);

    function buildDropdownItems(query: string): DropdownItem[] {
        const q = query.toLowerCase();
        const items: DropdownItem[] = [];

        // Open editor tabs
        const session = windowState.activeSession;
        if (session) {
            for (const view of session.views) {
                if (view.type === "editor" && view.title.toLowerCase().includes(q)) {
                    items.push({ type: "file", label: view.title, sublabel: "file", path: view.title });
                }
            }
        }

        // Tables from schema — iterate databases → schemas → tables
        for (const db of schemaStore.databases) {
            for (const schema of db.schemas) {
                for (const table of schema.tables) {
                    if (table.table_name.toLowerCase().includes(q)) {
                        items.push({
                            type: "table",
                            label: table.table_name,
                            sublabel: schema.name !== "public" ? schema.name : "table",
                            tableName: table.table_name,
                        });
                    }
                }
            }
        }

        // Recent tagged results
        for (const result of composerStore.recentResults()) {
            if (result.label.toLowerCase().includes(q)) {
                items.push({ type: "result", label: result.label, sublabel: "result", toolId: result.toolId });
            }
        }

        return items.slice(0, 12);
    }

    function selectDropdownItem(item: DropdownItem) {
        if (!editor || !triggerRange) return;
        editor.chain()
            .focus()
            .deleteRange(triggerRange)
            .run();

        if (item.type === "file") {
            editor.commands.insertContent({
                type: "fileChip",
                attrs: { path: item.path },
            });
        } else if (item.type === "table") {
            editor.commands.insertContent({
                type: "tableChip",
                attrs: { tableName: item.tableName },
            });
        } else if (item.type === "result") {
            editor.commands.insertContent({
                type: "resultChip",
                attrs: { toolId: item.toolId, label: item.label },
            });
        }
        editor.commands.insertContent(" ");
        dropdownVisible = false;
        triggerRange = null;
    }

    $effect(() => {
        if (!editorEl) return;
        const e = new Editor({
            element: editorEl,
            extensions: [
                StarterKit.configure({ history: true }),
                Placeholder.configure({ placeholder: "Ask Claude about your database..." }),
                FileChipNode,
                TableChipNode,
                ResultChipNode,
            ],
            editorProps: {
                attributes: {
                    class: "focus:outline-none min-h-[36px] max-h-[150px] overflow-y-auto text-sm text-foreground leading-relaxed",
                },
                handleKeyDown(_view, event) {
                    if (dropdownVisible && dropdownRef) {
                        if (dropdownRef.handleKey(event)) return true;
                    }
                    if (event.key === "Enter" && !event.shiftKey) {
                        event.preventDefault();
                        handleSend();
                        return true;
                    }
                    return false;
                },
            },
            onUpdate({ editor: e }) {
                const { from } = e.state.selection;
                const textBefore = e.state.doc.textBetween(
                    Math.max(0, from - 50),
                    from,
                    "\n",
                    "\0",
                );
                const match = /@([^\s@]*)$/.exec(textBefore);
                if (match) {
                    const query = match[1];
                    const start = from - match[0].length;
                    triggerRange = { from: start, to: from };
                    dropdownItems = buildDropdownItems(query);
                    dropdownVisible = true;
                    const coords = e.view.coordsAtPos(from);
                    dropdownPos = { x: coords.left, y: coords.bottom + 4 };
                } else {
                    dropdownVisible = false;
                    triggerRange = null;
                }
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
    {#if dropdownVisible}
        <div
            class="fixed z-50"
            style="left:{dropdownPos.x}px; top:{dropdownPos.y}px"
        >
            <ComposerDropdown
                bind:this={dropdownRef}
                items={dropdownItems}
                onSelect={selectDropdownItem}
                onClose={() => { dropdownVisible = false; }}
            />
        </div>
    {/if}
</div>
