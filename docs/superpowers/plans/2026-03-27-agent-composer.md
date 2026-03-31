# Agent Composer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the simple textarea in the agent panel with a Tiptap-based rich composer that supports inline context chips, @-mention dropdown, Monaco line-range selection via Cmd+L, result tagging, XML context injection, streaming file writes, and tab renaming.

**Architecture:** Tiptap editor (ProseMirror) with three custom atom inline nodes (FileChipNode, TableChipNode, ResultChipNode). At send time, the frontend resolves each chip to its content and prepends a `<context>` XML block to the user turn message — never the system prompt. The agent writes SQL via `write_file` tool which streams content into editor tabs live via `input_json_delta` SDK events.

**Tech Stack:** `@tiptap/core`, `@tiptap/pm`, `@tiptap/extension-history`, `@tiptap/extension-placeholder`, `@floating-ui/dom` (already installed), Svelte 5 runes, Monaco editor API, existing Tauri IPC, Bun harness HTTP+SSE.

---

## File Map

| File | Action | Responsibility |
|---|---|---|
| `src/lib/stores/composer.svelte.ts` | Create | Tagged results store, pending Monaco chip |
| `src/lib/components/agent/AgentComposer.svelte` | Create | Tiptap editor, chip insertion, context assembly, send |
| `src/lib/components/agent/ComposerDropdown.svelte` | Create | @ mention dropdown (files / tables / results) |
| `src/lib/components/agent/ToolCallCard.svelte` | Modify | Add "@ use as context" button |
| `src/lib/components/agent/AgentPanel.svelte` | Modify | Swap ComposerInput → AgentComposer, handle `tool.input_delta` |
| `src/lib/agent/tools.ts` | Modify | Add `write_file` to system prompt |
| `src/lib/agent/tool-executor.ts` | Modify | Handle `write_file` and `tool.input_delta` |
| `src/lib/agent/claude.ts` | Modify | Add `tool.input_delta` to `AgentEventType` |
| `src/lib/components/editor/SqlTestingEditor.svelte` | Modify | Monaco selection button + Cmd+L + watch `streamingContent` |
| `src/lib/components/editor/EditorTabs.svelte` | Modify | Double-click to rename tab |
| `src/lib/stores/session.svelte.ts` | Modify | Add `renameView()`, `streamingContent` field on view data |
| `packages/harness/src/session.ts` | Modify | Emit `tool.input_delta` from `input_json_delta` SDK events |

---

## Task 1: `composer.svelte.ts` store

**Files:**
- Create: `src/lib/stores/composer.svelte.ts`

- [ ] **Step 1: Create the store**

```ts
// src/lib/stores/composer.svelte.ts

export interface TaggedResult {
    toolId: string;
    toolName: string;
    output: string;       // already truncated to ≤50 rows
    label: string;        // e.g. "run_query result"
    truncated: boolean;
    totalRows?: number;
    timestamp: number;
}

class ComposerStore {
    taggedResults = $state<Map<string, TaggedResult>>(new Map());
    pendingChip = $state<{ path: string; lineStart: number; lineEnd: number } | null>(null);

    tagResult(toolId: string, toolName: string, rawOutput: string): void {
        let output = rawOutput;
        let truncated = false;
        let totalRows: number | undefined;
        try {
            const parsed = JSON.parse(rawOutput);
            if (Array.isArray(parsed) && parsed.length > 50) {
                totalRows = parsed.length;
                output = JSON.stringify(parsed.slice(0, 50));
                truncated = true;
            }
        } catch {
            // non-JSON output — store as-is
        }
        const newMap = new Map(this.taggedResults);
        newMap.set(toolId, {
            toolId, toolName, output,
            label: `${toolName} result`,
            truncated, totalRows,
            timestamp: Date.now(),
        });
        // keep at most 10
        if (newMap.size > 10) {
            const oldest = newMap.keys().next().value!;
            newMap.delete(oldest);
        }
        this.taggedResults = newMap;
    }

    untagResult(toolId: string): void {
        const newMap = new Map(this.taggedResults);
        newMap.delete(toolId);
        this.taggedResults = newMap;
    }

    isTagged(toolId: string): boolean {
        return this.taggedResults.has(toolId);
    }

    recentResults(): TaggedResult[] {
        return [...this.taggedResults.values()]
            .sort((a, b) => b.timestamp - a.timestamp)
            .slice(0, 10);
    }
}

export const composerStore = new ComposerStore();
```

- [ ] **Step 2: Verify TypeScript compiles**

```bash
pnpm check
```
Expected: no errors in `composer.svelte.ts`.

- [ ] **Step 3: Commit**

```bash
git add src/lib/stores/composer.svelte.ts
git commit -m "feat: add composer store for tagged results and pending chip"
```

---

## Task 2: Install Tiptap + base `AgentComposer.svelte`

**Files:**
- Modify: `package.json` (add deps)
- Create: `src/lib/components/agent/AgentComposer.svelte`
- Modify: `src/lib/components/agent/AgentPanel.svelte` (swap input)

- [ ] **Step 1: Install Tiptap packages**

```bash
pnpm add @tiptap/core @tiptap/pm @tiptap/extension-history @tiptap/extension-placeholder
```

Expected: packages added to `node_modules`, `pnpm-lock.yaml` updated.

- [ ] **Step 2: Create base `AgentComposer.svelte` (text only, no chips yet)**

```svelte
<!-- src/lib/components/agent/AgentComposer.svelte -->
<script lang="ts">
    import { Editor } from "@tiptap/core";
    import { History } from "@tiptap/extension-history";
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
                History,
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
```

- [ ] **Step 3: Add Tiptap placeholder CSS**

Tiptap's placeholder extension requires CSS to display the placeholder. Add to `src/app.css`:

```css
/* Tiptap composer placeholder */
.tiptap p.is-editor-empty:first-child::before {
    color: var(--muted-foreground);
    content: attr(data-placeholder);
    float: left;
    height: 0;
    pointer-events: none;
}
```

- [ ] **Step 4: Swap `ComposerInput` → `AgentComposer` in `AgentPanel.svelte`**

Find the import and usage of `ComposerInput` in `src/lib/components/agent/AgentPanel.svelte`.

Replace the import:
```ts
// remove:
import ComposerInput from "./ComposerInput.svelte";
// add:
import AgentComposer from "./AgentComposer.svelte";
```

Replace the component in the template (find `<ComposerInput` and replace):
```svelte
<AgentComposer
    onSend={(text, _doc) => send(text)}
    onStop={stop}
    running={agentStore.status === "running"}
    disabled={!sessionReady || !!sessionError}
/>
```

- [ ] **Step 5: Verify the app still works**

```bash
pnpm tauri dev
```
Expected: agent panel loads, typing in the composer and pressing Enter sends a message.

- [ ] **Step 6: Commit**

```bash
git add src/lib/components/agent/AgentComposer.svelte src/lib/components/agent/AgentPanel.svelte src/app.css package.json pnpm-lock.yaml
git commit -m "feat: replace ComposerInput with Tiptap-based AgentComposer"
```

---

## Task 3: Chip node types (FileChipNode, TableChipNode, ResultChipNode)

**Files:**
- Create: `src/lib/agent/composer-nodes.ts`

These are Tiptap `atom` inline nodes. `atom: true` means Tiptap treats the whole node as a single unit — backspace deletes the entire chip, and you can't place the cursor inside it.

- [ ] **Step 1: Create `composer-nodes.ts` with all three node types**

```ts
// src/lib/agent/composer-nodes.ts
import { Node, mergeAttributes } from "@tiptap/core";

// Inline SVG paths for chip icons (Tabler icon paths)
const ICON_SVG = {
    file: `<path stroke="none" d="M0 0h24v24H0z" fill="none"/><path d="M14 3v4a1 1 0 0 0 1 1h4"/><path d="M17 21h-10a2 2 0 0 1 -2 -2v-14a2 2 0 0 1 2 -2h7l5 5v11a2 2 0 0 1 -2 2z"/><path d="M9 13l6 0"/><path d="M9 17l6 0"/>`,
    table: `<path stroke="none" d="M0 0h24v24H0z" fill="none"/><path d="M3 5a2 2 0 0 1 2 -2h14a2 2 0 0 1 2 2v14a2 2 0 0 1 -2 2h-14a2 2 0 0 1 -2 -2v-14z"/><path d="M3 10h18"/><path d="M10 3v18"/>`,
    result: `<path stroke="none" d="M0 0h24v24H0z" fill="none"/><path d="M15 15m-4 0a4 4 0 1 0 8 0a4 4 0 1 0 -8 0"/><path d="M18.5 18.5l2.5 2.5"/><path d="M4 6h16"/><path d="M4 12h4"/><path d="M4 18h4"/>`,
};

function svg(type: keyof typeof ICON_SVG, color: string): string {
    return `<svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" stroke="${color}" stroke-width="1.5" fill="none" stroke-linecap="round" stroke-linejoin="round" style="flex-shrink:0">${ICON_SVG[type]}</svg>`;
}

function buildChipDom(
    iconType: keyof typeof ICON_SVG,
    label: string,
    suffix: string,
    bgClass: string,
    iconColor: string,
): HTMLElement {
    const dom = document.createElement("span");
    dom.setAttribute("contenteditable", "false");
    dom.setAttribute("data-chip", iconType);
    dom.style.cssText =
        `display:inline-flex;align-items:center;gap:3px;border-radius:4px;padding:1px 6px;font-size:11px;font-weight:500;line-height:1.4;vertical-align:middle;user-select:none;cursor:default;margin:0 1px;${bgClass}`;
    dom.innerHTML = `${svg(iconType, iconColor)}<span style="max-width:160px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap">${label}</span>${suffix ? `<span style="opacity:0.55;font-size:10px">${suffix}</span>` : ""}`;
    return dom;
}

// ── FileChipNode ────────────────────────────────────────────────────────────
export const FileChipNode = Node.create({
    name: "fileChip",
    group: "inline",
    inline: true,
    atom: true,
    selectable: true,

    addAttributes() {
        return {
            path: { default: "" },
            lineStart: { default: null as number | null },
            lineEnd: { default: null as number | null },
        };
    },

    parseHTML() {
        return [{ tag: 'span[data-type="file-chip"]' }];
    },

    renderHTML({ node, HTMLAttributes }) {
        return [
            "span",
            mergeAttributes(HTMLAttributes, { "data-type": "file-chip" }),
            node.attrs.lineStart
                ? `${node.attrs.path}:${node.attrs.lineStart}-${node.attrs.lineEnd}`
                : node.attrs.path,
        ];
    },

    addNodeView() {
        return ({ node }) => {
            const suffix = node.attrs.lineStart
                ? `:${node.attrs.lineStart}–${node.attrs.lineEnd}`
                : "";
            const dom = buildChipDom(
                "file",
                node.attrs.path,
                suffix,
                "background:color-mix(in srgb,#3b82f6 15%,transparent);border:1px solid color-mix(in srgb,#3b82f6 35%,transparent);color:#93c5fd",
                "#93c5fd",
            );
            return { dom };
        };
    },
});

// ── TableChipNode ───────────────────────────────────────────────────────────
export const TableChipNode = Node.create({
    name: "tableChip",
    group: "inline",
    inline: true,
    atom: true,
    selectable: true,

    addAttributes() {
        return { tableName: { default: "" } };
    },

    parseHTML() {
        return [{ tag: 'span[data-type="table-chip"]' }];
    },

    renderHTML({ node, HTMLAttributes }) {
        return ["span", mergeAttributes(HTMLAttributes, { "data-type": "table-chip" }), node.attrs.tableName];
    },

    addNodeView() {
        return ({ node }) => {
            const dom = buildChipDom(
                "table",
                node.attrs.tableName,
                "",
                "background:color-mix(in srgb,#a855f7 15%,transparent);border:1px solid color-mix(in srgb,#a855f7 35%,transparent);color:#d8b4fe",
                "#d8b4fe",
            );
            return { dom };
        };
    },
});

// ── ResultChipNode ──────────────────────────────────────────────────────────
export const ResultChipNode = Node.create({
    name: "resultChip",
    group: "inline",
    inline: true,
    atom: true,
    selectable: true,

    addAttributes() {
        return {
            toolId: { default: "" },
            label: { default: "" },
        };
    },

    parseHTML() {
        return [{ tag: 'span[data-type="result-chip"]' }];
    },

    renderHTML({ node, HTMLAttributes }) {
        return ["span", mergeAttributes(HTMLAttributes, { "data-type": "result-chip" }), node.attrs.label];
    },

    addNodeView() {
        return ({ node }) => {
            const dom = buildChipDom(
                "result",
                node.attrs.label,
                "",
                "background:color-mix(in srgb,#22c55e 15%,transparent);border:1px solid color-mix(in srgb,#22c55e 35%,transparent);color:#86efac",
                "#86efac",
            );
            return { dom };
        };
    },
});
```

- [ ] **Step 2: Register nodes in `AgentComposer.svelte`**

In `src/lib/components/agent/AgentComposer.svelte`, add the import and include nodes in the editor:

```ts
import { FileChipNode, TableChipNode, ResultChipNode } from "$lib/agent/composer-nodes";
```

In the `new Editor({ extensions: [...] })` array, add:
```ts
FileChipNode,
TableChipNode,
ResultChipNode,
```

- [ ] **Step 3: Verify TypeScript compiles**

```bash
pnpm check
```
Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add src/lib/agent/composer-nodes.ts src/lib/components/agent/AgentComposer.svelte
git commit -m "feat: add FileChipNode, TableChipNode, ResultChipNode Tiptap nodes"
```

---

## Task 4: `ComposerDropdown.svelte` + @ detection

**Files:**
- Create: `src/lib/components/agent/ComposerDropdown.svelte`
- Modify: `src/lib/components/agent/AgentComposer.svelte`

The dropdown opens when the user types `@`. It uses `@floating-ui/dom` (already in `package.json`) to position itself at the cursor. When the user selects an item, the `@query` text is deleted and the corresponding chip node is inserted.

- [ ] **Step 1: Create `ComposerDropdown.svelte`**

```svelte
<!-- src/lib/components/agent/ComposerDropdown.svelte -->
<script lang="ts">
    import IconFileText from "@tabler/icons-svelte/icons/file-text";
    import IconTable from "@tabler/icons-svelte/icons/table";
    import IconListSearch from "@tabler/icons-svelte/icons/list-search";

    interface DropdownItem {
        type: "file" | "table" | "result";
        label: string;
        sublabel?: string;
        // for file chips:
        path?: string;
        // for table chips:
        tableName?: string;
        // for result chips:
        toolId?: string;
    }

    interface Props {
        items: DropdownItem[];
        onSelect: (item: DropdownItem) => void;
        onClose: () => void;
    }

    let { items, onSelect, onClose }: Props = $props();
    let activeIndex = $state(0);

    export function handleKey(event: KeyboardEvent): boolean {
        if (event.key === "ArrowDown") {
            activeIndex = Math.min(activeIndex + 1, items.length - 1);
            return true;
        }
        if (event.key === "ArrowUp") {
            activeIndex = Math.max(activeIndex - 1, 0);
            return true;
        }
        if (event.key === "Enter" || event.key === "Tab") {
            if (items[activeIndex]) onSelect(items[activeIndex]);
            return true;
        }
        if (event.key === "Escape") {
            onClose();
            return true;
        }
        return false;
    }

    $effect(() => { activeIndex = 0; });
</script>

<div
    class="z-50 min-w-[200px] max-w-[280px] rounded-md border border-border bg-popover shadow-lg overflow-hidden"
    role="listbox"
>
    {#if items.length === 0}
        <div class="px-3 py-2 text-xs text-muted-foreground">No matches</div>
    {:else}
        {#each items as item, i}
            <button
                class="w-full flex items-center gap-2 px-3 py-1.5 text-xs text-left hover:bg-accent {i === activeIndex ? 'bg-accent' : ''}"
                onclick={() => onSelect(item)}
                role="option"
                aria-selected={i === activeIndex}
            >
                {#if item.type === "file"}
                    <IconFileText size={13} class="shrink-0 text-blue-400" />
                {:else if item.type === "table"}
                    <IconTable size={13} class="shrink-0 text-purple-400" />
                {:else}
                    <IconListSearch size={13} class="shrink-0 text-green-400" />
                {/if}
                <span class="truncate text-foreground">{item.label}</span>
                {#if item.sublabel}
                    <span class="ml-auto shrink-0 text-muted-foreground text-[10px]">{item.sublabel}</span>
                {/if}
            </button>
        {/each}
    {/if}
</div>
```

- [ ] **Step 2: Add @ trigger detection and dropdown wiring to `AgentComposer.svelte`**

Add these imports at the top of the `<script>` block:

```ts
import { computePosition, offset, flip, shift } from "@floating-ui/dom";
import ComposerDropdown from "./ComposerDropdown.svelte";
import { composerStore } from "$lib/stores/composer.svelte";
import { schemaStore } from "$lib/stores/schema.svelte";
import { windowState } from "$lib/stores/window.svelte";
```

Add state variables after `let editor`:
```ts
let dropdownVisible = $state(false);
let dropdownItems = $state<DropdownItem[]>([]);
let dropdownPos = $state({ x: 0, y: 0 });
let triggerRange = $state<{ from: number; to: number } | null>(null);
let dropdownRef: ReturnType<typeof ComposerDropdown> | null = null;
```

Add a helper to build dropdown items based on a query string:
```ts
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

    // Tables from schema
    const conn = schemaStore.activeConnection;
    if (conn) {
        for (const db of conn.databases ?? []) {
            for (const schema of db.schemas ?? []) {
                for (const table of schema.tables ?? []) {
                    if (table.name.toLowerCase().includes(q)) {
                        items.push({ type: "table", label: table.name, sublabel: "table", tableName: table.name });
                    }
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
```

Add an `onUpdate` handler inside the `new Editor({...})` call (after `editorProps`):
```ts
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
        // Position dropdown at cursor
        const coords = e.view.coordsAtPos(from);
        dropdownPos = { x: coords.left, y: coords.bottom + 4 };
    } else {
        dropdownVisible = false;
        triggerRange = null;
    }
},
```

Extend the `handleKeyDown` inside `editorProps` to delegate to the dropdown when visible:
```ts
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
```

Add a function to handle item selection from the dropdown:
```ts
function selectDropdownItem(item: DropdownItem) {
    if (!editor || !triggerRange) return;
    // Delete @query text
    editor.chain()
        .focus()
        .deleteRange(triggerRange)
        .run();

    // Insert the appropriate chip node
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
```

In the template, add the dropdown below the editor div (using fixed positioning):
```svelte
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
```

- [ ] **Step 3: Verify @ trigger works**

```bash
pnpm tauri dev
```
Expected: typing `@` in the composer shows a dropdown with open tabs and tables. Arrow keys navigate, Enter inserts a chip. The `@query` text is replaced by the chip.

- [ ] **Step 4: Commit**

```bash
git add src/lib/components/agent/ComposerDropdown.svelte src/lib/components/agent/AgentComposer.svelte
git commit -m "feat: add @ mention dropdown with file/table/result chips"
```

---

## Task 5: Context assembly + send

**Files:**
- Modify: `src/lib/components/agent/AgentComposer.svelte`
- Modify: `src/lib/stores/session.svelte.ts` (expose view content lookup)

At send time, the composer walks the Tiptap document, collects all chip nodes, resolves each to its content, builds a `<context>` XML block, and prepends it to the user text.

- [ ] **Step 1: Add context resolution helpers to `AgentComposer.svelte`**

Add these imports:
```ts
import { invoke } from "@tauri-apps/api/core";
```

Add the resolver function (inside the `<script>` block):
```ts
async function resolveChips(doc: ReturnType<Editor["getJSON"]>): Promise<string> {
    const parts: string[] = [];

    function walk(nodes: any[]) {
        for (const node of nodes) {
            if (node.type === "fileChip") {
                parts.push({ type: "file", attrs: node.attrs });
            } else if (node.type === "tableChip") {
                parts.push({ type: "table", attrs: node.attrs });
            } else if (node.type === "resultChip") {
                parts.push({ type: "result", attrs: node.attrs });
            }
            if (node.content) walk(node.content);
        }
    }
    const collected: any[] = [];
    // re-implement walk to push to collected
    function collect(nodes: any[]) {
        for (const node of nodes ?? []) {
            if (["fileChip", "tableChip", "resultChip"].includes(node.type)) {
                collected.push(node);
            }
            if (node.content) collect(node.content);
        }
    }
    collect(doc.content ?? []);

    if (collected.length === 0) return "";

    const xmlParts: string[] = [];

    for (const node of collected) {
        if (node.type === "fileChip") {
            const { path, lineStart, lineEnd } = node.attrs;
            const session = windowState.activeSession;
            const view = session?.views.find((v) => v.title === path);
            const fullContent: string = view?.data?.content ?? "";
            let content = fullContent;
            if (lineStart != null && lineEnd != null) {
                const lines = fullContent.split("\n");
                content = lines.slice(lineStart - 1, lineEnd).join("\n");
            }
            const attrs = lineStart != null ? ` lines="${lineStart}-${lineEnd}"` : "";
            xmlParts.push(`  <file path="${path}"${attrs}>\n${content}\n  </file>`);
        } else if (node.type === "tableChip") {
            const { tableName } = node.attrs;
            const conn = schemaStore.activeConnection;
            try {
                const schema = await invoke<string>("get_schema_table_details", {
                    connectionId: conn?.id ?? "",
                    schema: schemaStore.selectedSchema ?? "public",
                    table: tableName,
                });
                xmlParts.push(`  <table_schema name="${tableName}">\n${schema}\n  </table_schema>`);
            } catch {
                xmlParts.push(`  <table_schema name="${tableName}">unavailable</table_schema>`);
            }
        } else if (node.type === "resultChip") {
            const { toolId, label } = node.attrs;
            const tagged = composerStore.taggedResults.get(toolId);
            if (tagged) {
                const truncAttr = tagged.truncated
                    ? ` truncated="true" total_rows="${tagged.totalRows}"`
                    : ` truncated="false"`;
                xmlParts.push(`  <query_result tool="${tagged.toolName}"${truncAttr}>\n${tagged.output}\n  </query_result>`);
            }
        }
    }

    return `<context>\n${xmlParts.join("\n")}\n</context>`;
}
```

- [ ] **Step 2: Update `handleSend` to prepend context XML**

Replace the existing `handleSend` function:
```ts
async function handleSend() {
    if (!editor || running || disabled) return;
    const prose = editor.getText({ blockSeparator: "\n" }).trim();
    if (!prose) return;
    const doc = editor.getJSON();
    editor.commands.clearContent();

    const contextXml = await resolveChips(doc);
    const fullText = contextXml ? `${contextXml}\n\n${prose}` : prose;
    onSend(fullText, doc);
}
```

Update the `onSend` prop signature in `AgentPanel.svelte` — the handler already receives `text` so no change needed there (it uses `text` directly in `session.send(text)`).

- [ ] **Step 3: Verify context is sent**

In `AgentPanel.svelte`, temporarily `console.log` the text in `send()`. Run `pnpm tauri dev`, type `@tableName fix the nulls`, and check devtools console shows the `<context>` block in the logged message.

Remove the `console.log` after verifying.

- [ ] **Step 4: Commit**

```bash
git add src/lib/components/agent/AgentComposer.svelte
git commit -m "feat: assemble XML context block at send time from chip nodes"
```

---

## Task 6: ToolCallCard "@ use as context" button

**Files:**
- Modify: `src/lib/components/agent/ToolCallCard.svelte`

When a tool call completes, show a button to tag its output as context. Clicking it adds the result to `composerStore` and the button state toggles to "tagged".

- [ ] **Step 1: Add the button to `ToolCallCard.svelte`**

Add the import at the top of `<script>`:
```ts
import IconListSearch from "@tabler/icons-svelte/icons/list-search";
import { composerStore } from "$lib/stores/composer.svelte";
```

In the props/interface, ensure you have access to `toolCall.id`, `toolCall.toolName`, `toolCall.output`, `toolCall.status`. These already exist on `AgentToolCall`.

Find the section that renders the card header (where the tool name and status icon are shown). After the elapsed time display, add the tag button — show it only when `status === "done"` and `output` is defined:

```svelte
{#if toolCall.status === "done" && toolCall.output}
    {#if composerStore.isTagged(toolCall.id)}
        <span class="flex items-center gap-1 text-[10px] text-green-400">
            <IconListSearch size={11} />
            tagged
        </span>
    {:else}
        <button
            onclick={() => composerStore.tagResult(toolCall.id, toolCall.toolName, toolCall.output!)}
            class="flex items-center gap-1 rounded border border-border/50 px-1.5 py-0.5 text-[10px] text-muted-foreground hover:border-blue-500/40 hover:text-blue-400"
        >
            <IconListSearch size={11} />
            @ use as context
        </button>
    {/if}
{/if}
```

- [ ] **Step 2: Verify tagging works**

```bash
pnpm tauri dev
```
Run a query in the agent. After the tool call completes, the card should show "@ use as context". Click it — button changes to "tagged". Type `@` in the composer — the result should appear in the dropdown.

- [ ] **Step 3: Commit**

```bash
git add src/lib/components/agent/ToolCallCard.svelte
git commit -m "feat: add @ use as context button to ToolCallCard"
```

---

## Task 7: Monaco selection → "Add to chat" + Cmd+L

**Files:**
- Modify: `src/lib/components/editor/SqlTestingEditor.svelte`

When the user selects text in Monaco, a floating button appears. Clicking it (or pressing Cmd+L) sets `composerStore.pendingChip`. `AgentComposer` watches `pendingChip` and inserts a `FileChipNode`.

- [ ] **Step 1: Add selection listener and button to `SqlTestingEditor.svelte`**

Add imports:
```ts
import { composerStore } from "$lib/stores/composer.svelte";
import { windowState } from "$lib/stores/window.svelte";
```

Add state for the button:
```ts
let selectionButton = $state<{ visible: boolean; x: number; y: number }>({ visible: false, x: 0, y: 0 });
```

After the Monaco editor is initialised (find where `editorHandle` is set / `useMonacoEditor` callback), add:
```ts
editor.onDidChangeCursorSelection((e) => {
    const selection = e.selection;
    const isEmpty =
        selection.startLineNumber === selection.endLineNumber &&
        selection.startColumn === selection.endColumn;
    if (isEmpty) {
        selectionButton = { visible: false, x: 0, y: 0 };
        return;
    }
    // Get pixel coords of selection end
    const endPos = editor.getScrolledVisiblePosition({
        lineNumber: selection.endLineNumber,
        column: selection.endColumn,
    });
    if (!endPos) return;
    const containerRect = editorContainer.getBoundingClientRect();
    selectionButton = {
        visible: true,
        x: containerRect.left + endPos.left + 4,
        y: containerRect.top + endPos.top - 30, // float above cursor
    };
});

// Cmd+L / Ctrl+L keybinding
editor.addAction({
    id: "add-to-agent-chat",
    label: "Add to Agent Chat",
    keybindings: [
        // Monaco.KeyMod.CtrlCmd | Monaco.KeyCode.KeyL
        (2048 | 38),  // 2048 = CtrlCmd, 38 = KeyL
    ],
    run() {
        addSelectionToChat(editor);
    },
});
```

Add the helper function:
```ts
function addSelectionToChat(ed: import("monaco-editor").editor.IStandaloneCodeEditor) {
    const selection = ed.getSelection();
    if (!selection) return;
    const isEmpty =
        selection.startLineNumber === selection.endLineNumber &&
        selection.startColumn === selection.endColumn;
    if (isEmpty) return;

    // Find current tab title
    const session = windowState.activeSession;
    const activeView = session?.views.find((v) => v.id === session.activeViewId);
    const path = activeView?.title ?? "query.sql";

    composerStore.pendingChip = {
        path,
        lineStart: selection.startLineNumber,
        lineEnd: selection.endLineNumber,
    };
    selectionButton = { visible: false, x: 0, y: 0 };
}
```

In the template, add the floating button (outside the editor container, using fixed positioning):
```svelte
{#if selectionButton.visible}
    <button
        class="fixed z-50 flex items-center gap-1 rounded border border-blue-500/50 bg-popover px-2 py-1 text-xs text-blue-400 shadow-md hover:bg-accent"
        style="left:{selectionButton.x}px; top:{selectionButton.y}px"
        onclick={() => addSelectionToChat(editorHandle?.editor)}
    >
        + Add to chat
    </button>
{/if}
```

- [ ] **Step 2: Add `pendingChip` watcher in `AgentComposer.svelte`**

Add a `$effect` that watches `composerStore.pendingChip`:
```ts
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
```

- [ ] **Step 3: Verify end-to-end**

```bash
pnpm tauri dev
```
Select 3 lines in a SQL editor tab. The "+ Add to chat" button appears above the selection. Click it — the agent panel gets focus and a blue `📄 filename:1–3` chip appears in the composer. Same result with Cmd+L.

- [ ] **Step 4: Commit**

```bash
git add src/lib/components/editor/SqlTestingEditor.svelte src/lib/components/agent/AgentComposer.svelte
git commit -m "feat: Monaco selection add-to-chat button and Cmd+L shortcut"
```

---

## Task 8: `write_file` tool (system prompt + executor)

**Files:**
- Modify: `src/lib/agent/tools.ts`
- Modify: `src/lib/agent/tool-executor.ts`
- Modify: `src/lib/stores/session.svelte.ts`

The agent calls `write_file` via curl to create or update editor tabs. Never outputs SQL in chat.

- [ ] **Step 1: Add `write_file` to `buildToolInstructions` in `tools.ts`**

In `src/lib/agent/tools.ts`, find `buildToolInstructions` and add this tool to the curl examples section:

```ts
// At the end of the tool instructions string, add:
`
## File Writing

IMPORTANT: Never output SQL or code in your chat response text. Always use write_file to write or update files.
If the user tagged a specific file with @ in their message, use that exact filename to update it in place.
Choose descriptive, lowercase filenames (e.g. "find-null-users.sql", "orders-30d-analysis.sql").

curl -s -X POST http://127.0.0.1:\${port}/db/\${sessionId}/write_file \\
  -H "Content-Type: application/json" \\
  -d '{"fileName": "descriptive-name.sql", "content": "SELECT ..."}'

Response: {"ok": true, "action": "created"|"updated", "fileName": "...", "lines": N}
`
```

- [ ] **Step 2: Add `streamingContent` field to `ViewState` in `session.svelte.ts`**

Find the `ViewState` interface (or wherever views are typed) and add:
```ts
// in the data object shape or directly on ViewState:
streamingContent?: string;  // set during write_file streaming, cleared on finalise
```

Also add `renameView` method to the `Session` class:
```ts
renameView(viewId: string, newTitle: string): void {
    const view = this.views.find((v) => v.id === viewId);
    if (view) view.title = newTitle;
}
```

- [ ] **Step 3: Add `write_file` handler to `tool-executor.ts`**

In `src/lib/agent/tool-executor.ts`, inside the `executeTool` switch statement, add before the `default` case:

```ts
case "write_file": {
    const { fileName, content } = input as { fileName: string; content: string };
    const session = windowState.activeSession;
    if (!session) return { error: "no active session" };

    const existing = session.views.find((v) => v.title === fileName);
    if (existing) {
        existing.data = existing.data ?? {};
        existing.data.content = content;
        existing.data.streamingContent = undefined;
        return { ok: true, action: "updated", fileName, lines: content.split("\n").length };
    } else {
        session.openView("editor", fileName, { content });
        return { ok: true, action: "created", fileName, lines: content.split("\n").length };
    }
}
```

Add `windowState` import at the top of the file if not already present:
```ts
import { windowState } from "$lib/stores/window.svelte";
```

- [ ] **Step 4: Verify write_file works end-to-end**

```bash
pnpm tauri dev
```
Ask Claude: "write a query that counts orders per user into a file called test-query.sql". Expected: agent calls `write_file`, a new editor tab titled "test-query.sql" appears with the SQL content. Claude's chat message should NOT contain the SQL.

- [ ] **Step 5: Commit**

```bash
git add src/lib/agent/tools.ts src/lib/agent/tool-executor.ts src/lib/stores/session.svelte.ts
git commit -m "feat: add write_file tool — agent writes SQL to editor tabs"
```

---

## Task 9: Streaming file writes via `input_json_delta`

**Files:**
- Modify: `packages/harness/src/session.ts`
- Modify: `src/lib/agent/claude.ts`
- Modify: `src/lib/components/agent/AgentPanel.svelte`
- Modify: `src/lib/agent/tool-executor.ts`
- Modify: `src/lib/components/editor/SqlTestingEditor.svelte`

When the agent writes a file, content streams into the editor tab token by token via `input_json_delta` SDK events.

- [ ] **Step 1: Add `tool.input_delta` to `HarnessEvent` in `session.ts`**

In `packages/harness/src/session.ts`, find the `HarnessEvent` type and add:
```ts
| { type: "tool.input_delta"; toolId: string; toolName: string; partialContent: string }
```

- [ ] **Step 2: Emit `tool.input_delta` from `input_json_delta` SDK events in `session.ts`**

In the `consume()` method, find the `stream_event` handling block. Currently it handles `content_block_delta` with `text_delta`. Add tracking for active tool_use blocks and `input_json_delta`:

Add instance variables to `ClaudeSession`:
```ts
private activeToolUseId: string | null = null;
private activeToolName: string | null = null;
private partialInput: string = "";
```

In the `stream_event` handler, extend it to track tool_use blocks:
```ts
if (streamEvent?.type === "content_block_start") {
    const block = (streamEvent as any).content_block;
    if (block?.type === "tool_use" && block?.name === "write_file") {
        this.activeToolUseId = block.id;
        this.activeToolName = block.name;
        this.partialInput = "";
    }
} else if (streamEvent?.type === "content_block_delta") {
    const delta = streamEvent.delta as Record<string, unknown>;
    if (delta?.type === "text_delta" && delta.text) {
        // existing text streaming
        this.turnHasStreamEvents = true;
        this.emitFn({ type: "text.delta", content: delta.text as string });
    } else if (delta?.type === "thinking_delta" && delta.thinking) {
        this.turnHasStreamEvents = true;
        this.emitFn({ type: "thinking.delta", content: delta.thinking as string });
    } else if (delta?.type === "input_json_delta" && this.activeToolUseId) {
        this.partialInput += (delta as any).partial_json ?? "";
        // Extract partial "content" value from accumulated JSON
        const match = /"content"\s*:\s*"((?:[^"\\]|\\.)*)/s.exec(this.partialInput);
        if (match) {
            const partial = match[1]
                .replace(/\\n/g, "\n")
                .replace(/\\"/g, '"')
                .replace(/\\\\/g, "\\");
            this.emitFn({
                type: "tool.input_delta",
                toolId: this.activeToolUseId,
                toolName: this.activeToolName!,
                partialContent: partial,
            });
        }
    }
} else if (streamEvent?.type === "content_block_stop") {
    if (this.activeToolUseId) {
        this.activeToolUseId = null;
        this.activeToolName = null;
        this.partialInput = "";
    }
}
```

- [ ] **Step 3: Add `tool.input_delta` to `AgentEventType` in `claude.ts`**

In `src/lib/agent/claude.ts`, add to `AgentEventType`:
```ts
| { type: "tool.input_delta"; toolId: string; toolName: string; partialContent: string }
```

- [ ] **Step 4: Handle `tool.input_delta` in `AgentPanel.svelte`**

In the `handleEvent` function in `src/lib/components/agent/AgentPanel.svelte`, add a case:
```ts
} else if (event.type === "tool.input_delta") {
    // Stream content into the editor tab
    const session = windowState.activeSession;
    if (session) {
        // Find the tab being written — look for one with status running for this toolId
        const toolCall = agentStore.toolCalls.find((t) => t.id === event.toolId);
        if (toolCall) {
            const fileName = (toolCall.input as any)?.fileName;
            if (fileName) {
                let view = session.views.find((v) => v.title === fileName);
                if (!view) {
                    // Create the tab immediately so streaming is visible
                    session.openView("editor", fileName, { content: "" });
                    view = session.views.find((v) => v.title === fileName);
                }
                if (view) {
                    view.data = view.data ?? {};
                    view.data.streamingContent = event.partialContent;
                }
            }
        }
    }
}
```

- [ ] **Step 5: Watch `streamingContent` in `SqlTestingEditor.svelte` and update Monaco**

In `SqlTestingEditor.svelte`, after the Monaco editor is initialised, add a `$effect` that watches the context's `streamingContent` and pushes it to the editor:

```ts
$effect(() => {
    const streaming = context.streamingContent;
    if (streaming !== undefined && editorHandle?.editor) {
        const ed = editorHandle.editor;
        const current = ed.getValue();
        if (current !== streaming) {
            ed.setValue(streaming);
            // Move cursor to end
            const lineCount = ed.getModel()?.getLineCount() ?? 1;
            ed.setPosition({ lineNumber: lineCount, column: 9999 });
        }
    }
});
```

- [ ] **Step 6: Rebuild harness binary**

```bash
cd packages/harness && bun run build && cd ../..
```

- [ ] **Step 7: Verify streaming write**

```bash
pnpm tauri dev
```
Ask Claude to write a complex query into a file. Expected:
- Editor tab opens immediately when tool starts
- SQL appears token by token in the Monaco editor as the agent generates it
- On completion the full SQL is present in the tab

- [ ] **Step 8: Commit**

```bash
git add packages/harness/src/session.ts src/lib/agent/claude.ts src/lib/components/agent/AgentPanel.svelte src/lib/agent/tool-executor.ts src/lib/components/editor/SqlTestingEditor.svelte src-tauri/binaries/
git commit -m "feat: stream write_file content into editor tab via input_json_delta"
```

---

## Task 10: Tab renaming (double-click)

**Files:**
- Modify: `src/lib/components/editor/EditorTabs.svelte`
- Modify: `src/lib/stores/session.svelte.ts` (already added `renameView` in Task 8)

- [ ] **Step 1: Add inline rename to `EditorTabs.svelte`**

Add to the `<script>` block:
```ts
let renamingViewId = $state<string | null>(null);
let renameValue = $state("");
let renameInput: HTMLInputElement;

function startRename(viewId: string, currentTitle: string) {
    renamingViewId = viewId;
    renameValue = currentTitle;
    // focus input on next tick
    setTimeout(() => renameInput?.focus(), 0);
}

function commitRename() {
    if (!renamingViewId) return;
    const trimmed = renameValue.trim();
    if (trimmed) {
        windowState.activeSession?.renameView(renamingViewId, trimmed);
    }
    renamingViewId = null;
}
```

Find the tab title rendering in the template. It will look something like:
```svelte
<span class="...">{view.title}</span>
```

Replace it with:
```svelte
{#if renamingViewId === view.id}
    <input
        bind:this={renameInput}
        bind:value={renameValue}
        class="w-24 bg-transparent text-xs outline-none border-b border-primary"
        onblur={commitRename}
        onkeydown={(e) => {
            if (e.key === "Enter") commitRename();
            if (e.key === "Escape") renamingViewId = null;
        }}
    />
{:else}
    <span
        class="..."
        ondblclick={() => startRename(view.id, view.title)}
    >
        {view.title}
    </span>
{/if}
```

- [ ] **Step 2: Verify rename works**

```bash
pnpm tauri dev
```
Double-click a tab title — it becomes an editable input. Type a new name and press Enter — tab is renamed. Escape cancels.

- [ ] **Step 3: Commit**

```bash
git add src/lib/components/editor/EditorTabs.svelte
git commit -m "feat: double-click tab title to rename"
```

---

## Self-Review

**Spec coverage check:**
- [x] Tiptap composer replaces ComposerInput → Task 2
- [x] FileChipNode (IconFileText, blue) → Task 3
- [x] TableChipNode (IconTable, purple) → Task 3
- [x] ResultChipNode (IconListSearch, green) → Task 3
- [x] @ dropdown with files/tables/results → Task 4
- [x] Context XML assembled at send time, user turn → Task 5
- [x] ToolCallCard "@ use as context" button, 50-row cap → Task 6
- [x] Monaco selection → Add to chat button + Cmd+L → Task 7
- [x] write_file tool: system prompt + executor → Task 8
- [x] streaming writes via input_json_delta → Task 9
- [x] Tab renaming double-click → Task 10
- [x] renameView() in session.svelte.ts → Task 8 Step 2

**Type consistency check:**
- `composerStore.tagResult(toolId, toolName, rawOutput)` — defined Task 1, called Task 6 ✓
- `composerStore.pendingChip` — set in Task 7, watched in Task 7 Step 2 ✓
- `TaggedResult.toolId/toolName/output/label` — defined Task 1, used in Task 5 context resolver ✓
- `FileChipNode` attrs: `path, lineStart, lineEnd` — defined Task 3, inserted Task 4 + Task 7 ✓
- `HarnessEvent tool.input_delta` — added Task 9 Step 1 (session.ts), consumed Task 9 Step 3 (claude.ts) ✓
- `session.renameView(viewId, newTitle)` — added Task 8 Step 2, called Task 10 ✓
- `view.data.streamingContent` — added Task 8 Step 2, written Task 9 Step 4, read Task 9 Step 5 ✓
