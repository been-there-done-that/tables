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
    import { invoke } from "@tauri-apps/api/core";

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
                StarterKit.configure({}),
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
                        void handleSend();
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
                    const DROPDOWN_WIDTH = 280; // max-w from ComposerDropdown
                    const DROPDOWN_HEIGHT = 200; // approximate max height
                    const vw = window.innerWidth;
                    const vh = window.innerHeight;

                    let x = coords.left;
                    let y = coords.bottom + 4;

                    // Clamp horizontally
                    if (x + DROPDOWN_WIDTH > vw) {
                        x = Math.max(0, vw - DROPDOWN_WIDTH - 8);
                    }
                    // If near bottom, show above cursor instead
                    if (y + DROPDOWN_HEIGHT > vh) {
                        y = coords.top - DROPDOWN_HEIGHT - 4;
                    }

                    dropdownPos = { x, y };
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

    $effect(() => {
        const chip = composerStore.pendingChip;
        if (!chip || !editor) return;
        editor.commands.insertContent({
            type: "fileChip",
            attrs: { path: chip.path, lineStart: chip.lineStart, lineEnd: chip.lineEnd },
        });
        editor.commands.focus();
        composerStore.pendingChip = null;
    });

    function escapeXml(str: string): string {
        return str
            .replace(/&/g, "&amp;")
            .replace(/</g, "&lt;")
            .replace(/>/g, "&gt;")
            .replace(/"/g, "&quot;");
    }

    async function resolveChips(doc: ReturnType<Editor["getJSON"]>): Promise<string> {
        const collected: Array<{ type: string; attrs: Record<string, unknown> }> = [];

        function collect(nodes: unknown[]): void {
            for (const node of nodes ?? []) {
                const n = node as Record<string, unknown>;
                if (["fileChip", "tableChip", "resultChip"].includes(n.type as string)) {
                    collected.push(n as { type: string; attrs: Record<string, unknown> });
                }
                if (n.content) collect(n.content as unknown[]);
            }
        }
        collect(doc.content ?? []);

        if (collected.length === 0) return "";

        const xmlParts: string[] = [];

        for (const node of collected) {
            if (node.type === "fileChip") {
                const { path, lineStart, lineEnd } = node.attrs as {
                    path: string;
                    lineStart: number | null;
                    lineEnd: number | null;
                };
                const session = windowState.activeSession;
                const view = session?.views.find((v: { title: string }) => v.title === path);
                if (!view) {
                    xmlParts.push(`  <!-- file "${escapeXml(path)}" is not open in any tab -->`);
                    continue;
                }
                const fullContent: string = (view?.data as Record<string, unknown>)?.content as string ?? "";
                let content = fullContent;
                if (lineStart != null && lineEnd != null) {
                    const lines = fullContent.split("\n");
                    content = lines.slice(lineStart - 1, lineEnd).join("\n");
                }
                const attrs = lineStart != null && lineEnd != null ? ` lines="${lineStart}-${lineEnd}"` : "";
                xmlParts.push(`  <file path="${escapeXml(path)}"${attrs}><![CDATA[\n${content}\n]]></file>`);
            } else if (node.type === "tableChip") {
                const { tableName } = node.attrs as { tableName: string };
                const conn = schemaStore.activeConnection;
                try {
                    const details = await invoke<unknown>("get_schema_table_details", {
                        connectionId: conn?.id ?? "",
                        database: schemaStore.selectedDatabase ?? "",
                        schema: schemaStore.activeSchema ?? "public",
                        tableName,
                    });
                    const schema = JSON.stringify(details, null, 2);
                    xmlParts.push(`  <table_schema name="${escapeXml(tableName)}">\n${schema}\n  </table_schema>`);
                } catch {
                    xmlParts.push(`  <table_schema name="${escapeXml(tableName)}">unavailable</table_schema>`);
                }
            } else if (node.type === "resultChip") {
                const { toolId, label } = node.attrs as { toolId: string; label: string };
                const tagged = composerStore.taggedResults.get(toolId);
                if (tagged) {
                    const truncAttr = tagged.truncated
                        ? ` truncated="true" total_rows="${tagged.totalRows}"`
                        : ` truncated="false"`;
                    xmlParts.push(
                        `  <query_result tool="${escapeXml(tagged.toolName)}"${truncAttr}><![CDATA[\n${tagged.output}\n]]></query_result>`,
                    );
                }
            }
        }

        return `<context>\n${xmlParts.join("\n")}\n</context>`;
    }

    async function handleSend() {
        if (!editor || running || disabled) return;
        const prose = editor.getText({ blockSeparator: "\n" }).trim();
        if (!prose) return;
        const doc = editor.getJSON();

        const contextXml = await resolveChips(doc);
        editor.commands.clearContent();

        // Re-check state after async resolution
        if (!editor || running) return;
        const fullText = contextXml ? `${contextXml}\n\n${prose}` : prose;
        onSend(fullText, doc);
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
                onclick={() => void handleSend()}
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
