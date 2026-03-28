<!-- src/lib/components/agent/AgentComposer.svelte -->
<script lang="ts">
    import { Editor } from "@tiptap/core";
    import StarterKit from "@tiptap/starter-kit";
    import { Placeholder } from "@tiptap/extension-placeholder";
    import { FileChipNode, TableChipNode, ResultChipNode } from "$lib/agent/composer-nodes";
    import IconArrowUp from "@tabler/icons-svelte/icons/arrow-up";
    import IconSquare from "@tabler/icons-svelte/icons/square";
    import IconCpu from "@tabler/icons-svelte/icons/cpu";
    import IconBrain from "@tabler/icons-svelte/icons/brain";
    import IconCheck from "@tabler/icons-svelte/icons/check";
    import IconChevronDown from "@tabler/icons-svelte/icons/chevron-down";
    import ComposerDropdown from "./ComposerDropdown.svelte";
    import type { DropdownItem } from "./ComposerDropdown.svelte";
    import * as Menu from "$lib/components/ui/dropdown-menu";
    import { composerStore } from "$lib/stores/composer.svelte";
    import { schemaStore } from "$lib/stores/schema.svelte";
    import { windowState } from "$lib/stores/window.svelte";
    import { settingsStore } from "$lib/stores/settings.svelte";
    import { invoke } from "@tauri-apps/api/core";

    interface Props {
        onSend: (displayText: string, fullText: string, rawDoc: unknown) => void;
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

    const MODELS = [
        { id: "claude-haiku-4-5-20251001", label: "Haiku 4.5" },
        { id: "claude-sonnet-4-6",         label: "Sonnet 4.6" },
        { id: "claude-opus-4-6",           label: "Opus 4.6" },
    ] as const;

    const EFFORTS = [
        { id: "auto",   label: "Auto" },
        { id: "low",    label: "Low" },
        { id: "medium", label: "Medium" },
        { id: "high",   label: "High" },
        { id: "max",    label: "Max" },
    ] as const;

    const currentModelLabel = $derived(
        MODELS.find((m) => m.id === settingsStore.aiModel)?.label ?? "Sonnet 4.6"
    );
    const currentEffortLabel = $derived(
        EFFORTS.find((e) => e.id === settingsStore.aiEffort)?.label ?? "Auto"
    );

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
                Placeholder.configure({ placeholder: "Ask anything… @mention a table, file, or query result" }),
                FileChipNode,
                TableChipNode,
                ResultChipNode,
            ],
            editorProps: {
                attributes: {
                    class: "focus:outline-none min-h-[32px] max-h-[130px] overflow-y-auto text-[11.5px] text-foreground/90 leading-relaxed",
                    autocorrect: "off",
                    autocapitalize: "off",
                    autocomplete: "off",
                    spellcheck: "false",
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
                    dropdownVisible = dropdownItems.length > 0;
                    const coords = e.view.coordsAtPos(from);
                    const DROPDOWN_WIDTH = 300;
                    const DROPDOWN_HEIGHT = 220;
                    const vw = window.innerWidth;

                    let x = coords.left;
                    // Always anchor above the cursor — composer is at the bottom so
                    // showing below would cover whatever the user is typing.
                    let y = coords.top - DROPDOWN_HEIGHT - 4;
                    // Clamp: if no room above, fall back to below
                    if (y < 4) {
                        y = coords.bottom + 4;
                    }
                    // Clamp horizontally
                    if (x + DROPDOWN_WIDTH > vw) {
                        x = Math.max(0, vw - DROPDOWN_WIDTH - 8);
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

    /** Build display text from the doc, preserving chip labels as @name inline. */
    function docToDisplayText(doc: ReturnType<Editor["getJSON"]>): string {
        const parts: string[] = [];
        function walk(nodes: unknown[]): void {
            for (const node of nodes ?? []) {
                const n = node as Record<string, unknown>;
                if (n.type === "text") {
                    parts.push(n.text as string);
                } else if (n.type === "fileChip") {
                    parts.push(`@${(n.attrs as Record<string, unknown>).path}`);
                } else if (n.type === "tableChip") {
                    parts.push(`@${(n.attrs as Record<string, unknown>).tableName}`);
                } else if (n.type === "resultChip") {
                    parts.push(`@${(n.attrs as Record<string, unknown>).label}`);
                } else if (n.content) {
                    walk(n.content as unknown[]);
                    if (["paragraph", "heading"].includes(n.type as string)) parts.push("\n");
                }
            }
        }
        walk(doc.content ?? []);
        return parts.join("").trim();
    }

    async function handleSend() {
        if (!editor || running || disabled) return;
        const doc = editor.getJSON();
        const displayText = docToDisplayText(doc);  // includes @chip labels
        const prose = editor.getText({ blockSeparator: "\n" }).trim(); // plain text for agent body
        if (!displayText) return;

        const contextXml = await resolveChips(doc);
        editor.commands.clearContent();

        // Re-check state after async resolution
        if (!editor || running) return;
        const agentBody = prose || displayText;
        const fullText = contextXml ? `${contextXml}\n\n${agentBody}` : agentBody;
        onSend(displayText, fullText, doc);
    }

    export function insertContent(content: unknown) {
        editor?.commands.insertContent(content as any);
        editor?.commands.focus();
    }
</script>

<div class="px-2.5 py-2">
    <!-- Unified composer container -->
    <div class="rounded-lg border border-border/60 bg-background/80 transition-colors focus-within:border-border">
        <!-- Editor area -->
        <div
            bind:this={editorEl}
            class="px-2.5 pt-2 pb-0.5"
        ></div>
        <!-- Bottom bar -->
        <div class="flex items-center justify-between px-2 pb-2 pt-1">
            <div class="flex items-center gap-0.5">
                <!-- Model picker -->
                <Menu.Root>
                    <Menu.Trigger>
                        <button
                            class="flex items-center gap-1 rounded px-1.5 py-1 text-[10.5px] text-muted-foreground/50 transition-colors hover:bg-foreground/5 hover:text-muted-foreground"
                            title="Switch model"
                        >
                            <IconCpu size={11} />
                            <span class="font-mono">{currentModelLabel}</span>
                            <IconChevronDown size={9} class="opacity-50" />
                        </button>
                    </Menu.Trigger>
                    <Menu.Content
                        class="w-40 border border-border bg-background shadow-md p-1"
                        align="start"
                        side="top"
                    >
                        {#each MODELS as m}
                            <Menu.Item
                                class="flex items-center justify-between gap-2 px-2 py-1.5 text-[11px] font-mono rounded cursor-pointer"
                                onclick={() => { settingsStore.aiModel = m.id; }}
                            >
                                {m.label}
                                {#if settingsStore.aiModel === m.id}
                                    <IconCheck size={11} class="shrink-0 text-accent" />
                                {/if}
                            </Menu.Item>
                        {/each}
                    </Menu.Content>
                </Menu.Root>

                <!-- Effort picker -->
                <Menu.Root>
                    <Menu.Trigger>
                        <button
                            class="flex items-center gap-1 rounded px-1.5 py-1 text-[10.5px] transition-colors hover:bg-foreground/5 {settingsStore.aiEffort !== 'auto' && settingsStore.aiEffort !== 'low' ? 'text-accent/70 hover:text-accent' : 'text-muted-foreground/50 hover:text-muted-foreground'}"
                            title="Thinking effort"
                        >
                            <IconBrain size={11} />
                            <span class="font-mono">{currentEffortLabel}</span>
                            <IconChevronDown size={9} class="opacity-50" />
                        </button>
                    </Menu.Trigger>
                    <Menu.Content
                        class="w-36 border border-border bg-background shadow-md p-1"
                        align="start"
                        side="top"
                    >
                        {#each EFFORTS as ef}
                            <Menu.Item
                                class="flex items-center justify-between gap-2 px-2 py-1.5 text-[11px] font-mono rounded cursor-pointer"
                                onclick={() => { settingsStore.aiEffort = ef.id; }}
                            >
                                {ef.label}
                                {#if settingsStore.aiEffort === ef.id}
                                    <IconCheck size={11} class="shrink-0 text-accent" />
                                {/if}
                            </Menu.Item>
                        {/each}
                    </Menu.Content>
                </Menu.Root>
            </div>

            <div class="flex items-center gap-1.5">
                {#if running}
                    <button
                        onclick={onStop}
                        title="Stop"
                        class="flex h-6 w-6 items-center justify-center rounded-full bg-destructive/80 text-white transition-opacity hover:bg-destructive"
                    >
                        <IconSquare size={10} fill="currentColor" />
                    </button>
                {:else}
                    <button
                        onclick={() => void handleSend()}
                        disabled={disabled}
                        title="Send (↵)"
                        class="flex h-6 w-6 items-center justify-center rounded-full bg-foreground text-background transition-opacity hover:opacity-80 disabled:opacity-25"
                    >
                        <IconArrowUp size={13} stroke-width={2.5} />
                    </button>
                {/if}
            </div>
        </div>
    </div>
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
