# Agent Composer Design

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace the simple textarea in the agent panel with a rich Tiptap-based composer that supports inline context chips, Monaco line-range selection, result tagging, streaming file writes, and clean XML context injection.

**Architecture:** Tiptap editor with custom inline nodes per chip type. Context is assembled as XML and prepended to the user turn at send time — never injected into the system prompt. The agent writes SQL directly into editor tabs via a `write_file` tool, streaming content live via `input_json_delta` SDK events.

**Tech Stack:** Tiptap (ProseMirror), Svelte 5 runes, Monaco editor selection API, existing Tauri IPC commands, Bun harness HTTP+SSE.

---

## File Structure

### New files
```
src/lib/components/agent/AgentComposer.svelte      — Tiptap composer, replaces ComposerInput.svelte
src/lib/components/agent/ComposerDropdown.svelte   — @ mention dropdown (files, tables, results)
src/lib/stores/composer.svelte.ts                  — tagged results store, chip content cache
```

### Modified files
```
src/lib/components/agent/ToolCallCard.svelte       — add "@ use as context" button
src/lib/components/agent/AgentPanel.svelte         — wire new composer, handle write_file/tool.input_delta
src/lib/agent/tools.ts                             — add write_file to system prompt tool list
src/lib/agent/tool-executor.ts                     — handle write_file, tool.input_delta streaming
src/lib/components/editor/SqlTestingEditor.svelte  — Monaco selection → "Add to chat" button + Cmd+L
src/lib/components/editor/EditorTabs.svelte        — double-click tab title to rename
packages/harness/src/session.ts                    — emit tool.input_delta for write_file calls
```

---

## Chip Types

Four chip types, all rendered inline inside Tiptap as non-editable nodes:

| Node name | Visual | Colour | Resolves to at send time |
|---|---|---|---|
| `file-chip` | `📄 orders.sql` | blue | full `view.data.content` from session tab |
| `file-chip` (ranged) | `📄 orders.sql:13–16` | blue | content sliced to line range |
| `table-chip` | `🗄️ orders` | purple | `invoke("get_schema_table_details", ...)` |
| `result-chip` | `📊 run_query result` | green | stored tool output, max 50 rows |

---

## Component Designs

### `AgentComposer.svelte`

Tiptap editor (`@tiptap/core` + `@tiptap/extension-mention`) with three custom extensions:

1. **`FileChipNode`** — inline node, `contentEditable: false`. Props: `path: string`, `lineStart?: number`, `lineEnd?: number`. Renders as blue chip. Created from `@` dropdown (full file) or by `Cmd+L` / "Add to chat" from Monaco.
2. **`TableChipNode`** — inline node. Props: `tableName: string`. Purple chip. Created from `@` dropdown.
3. **`ResultChipNode`** — inline node. Props: `toolId: string`, `label: string`. Green chip. Created when user clicks "@ use as context" on a ToolCallCard.

**@ trigger:** `@tiptap/extension-mention` configured with `char: "@"`. On trigger, opens `ComposerDropdown`. On item select, inserts the appropriate node type.

**Send behaviour:**
1. Walk Tiptap document, collect all chip nodes and prose text.
2. For each chip, resolve content (see Context Assembly below).
3. Assemble XML + prose, POST to `/session/send`.
4. Clear editor, set `agentStore.status = "running"`.

**Keyboard:** `Enter` → send. `Shift+Enter` → newline. `Backspace` on chip → delete whole chip (Tiptap handles this natively for inline nodes). `Cmd+L` → insert line-range chip from active Monaco selection (dispatched via `composerStore`).

### `ComposerDropdown.svelte`

Floating dropdown anchored to the `@` cursor position. Three search buckets merged and filtered by query string:

- **Open tabs** — all `view.type === "editor"` tabs in the active session. Label: `📄 {title}`.
- **Tables** — all table names from `schemaStore` for the active connection. Label: `🗄️ {tableName}`.
- **Recent results** — last 10 tagged results from `composerStore.taggedResults`. Label: `📊 {toolName} result`.

Keyboard: `↑↓` navigate, `Enter`/`Tab` select, `Escape` close.

### `composer.svelte.ts`

```ts
interface TaggedResult {
  toolId: string;
  toolName: string;
  output: string;      // raw output, already truncated to 50 rows
  label: string;       // "run_query result"
  truncated: boolean;
  totalRows?: number;
  timestamp: number;
}

class ComposerStore {
  taggedResults = $state<Map<string, TaggedResult>>(new Map());
  pendingChip = $state<{ path: string; lineStart: number; lineEnd: number } | null>(null);

  tagResult(toolId: string, toolName: string, rawOutput: string): void
  untagResult(toolId: string): void
  isTagged(toolId: string): boolean
  // pendingChip is set by Monaco "Add to chat" — AgentComposer watches and inserts the chip
}
export const composerStore = new ComposerStore();
```

---

## Context Assembly

At send time, the frontend resolves every chip to its content and builds a `<context>` block prepended to the user's prose text. This goes in the **user turn message body**, not the system prompt.

```xml
<context>
  <file path="orders.sql" lines="13-16">
    LEFT JOIN users u
      ON o.user_id = u.id
    WHERE o.created_at
      > NOW() - INTERVAL '30d'
  </file>
  <table_schema name="orders">
    id          bigint       NOT NULL
    user_id     bigint       NULLABLE
    created_at  timestamptz  NOT NULL
  </table_schema>
  <query_result tool="run_query" rows="3" truncated="false">
    [{"user_id":null,"cnt":3},...]
  </query_result>
</context>

why is the join slow on large date ranges?
```

**Resolution per chip type:**
- `file-chip` (no range): read `view.data.content` from `sessionStore` for the matching tab title.
- `file-chip` (ranged): same, then `.split("\n").slice(lineStart-1, lineEnd).join("\n")`.
- `table-chip`: `await invoke("get_schema_table_details", { connectionId, schema, table })`.
- `result-chip`: look up `composerStore.taggedResults.get(toolId).output`.

**Result truncation:** When a tool output is tagged, `composerStore.tagResult()` parses JSON array results and caps at 50 rows. If truncated: `truncated="true" total_rows="N"` attributes are set on `<query_result>` so the agent knows the data is partial.

---

## Agent `write_file` Tool

### System prompt instruction
Added to `tools.ts` system prompt:

> Never output SQL or code in the chat message body. Always use `write_file` to write or update files. If the user tagged a specific file with `@`, update that file by using its exact name as `fileName`.

### Tool call (agent → harness via curl)
```bash
curl -s -X POST http://127.0.0.1:PORT/db/SESSION_ID/write_file \
  -d '{"fileName": "orders-analysis.sql", "content": "SELECT ..."}'
```

### Frontend handler in `tool-executor.ts`
```
write_file received { fileName, content }
  → find session tab where view.title === fileName
  → if found: update view.data.content in place (existing tab)
  → if not found: session.openView("editor", fileName, { content })
  → POST result: { ok: true, action: "created"|"updated", fileName, lines: N }
```

**User-directed update:** When the user tags `@orders.sql` and says "rewrite the join", the agent sees `<file path="orders.sql">` in context and calls `write_file` with `fileName: "orders.sql"`. The frontend matches by title and updates in place — no new tab.

---

## Streaming File Writes

The SDK emits `input_json_delta` stream events as the agent generates tool input. For `write_file` calls, we intercept these to stream content into the editor tab live.

### `session.ts` changes

Track active `tool_use` blocks. On each `stream_event` of type `content_block_delta` with `input_json_delta` subtype, check if the active block is `write_file`. If so:
1. Append delta to accumulated partial JSON string.
2. Extract partial `content` field value with: `/"content"\s*:\s*"((?:[^"\\]|\\.)*)/`.
3. Unescape `\n`, `\"`, `\\` in the extracted string.
4. Emit new event: `{ type: "tool.input_delta", toolId, toolName: "write_file", partialContent }`.

New `HarnessEvent` type:
```ts
| { type: "tool.input_delta"; toolId: string; toolName: string; partialContent: string }
```

### `tool-executor.ts` changes

Handle `tool.input_delta` for `write_file`:
1. On first delta: find or create the editor tab (so tab opens immediately).
2. Update `view.data.content` with `partialContent` (debounced 50ms to avoid thrashing).

On `tool.completed` for `write_file`: set final content from the completed tool result.

### `ToolCallCard.svelte` changes

While `status === "running"` and `toolName === "write_file"`: show animated write indicator and preview of first line. When done: show `✓ wrote {fileName} — {N} lines` and a `↗ jump to file` button that focuses the relevant editor tab.

---

## Monaco Line-Range Selection

### `SqlTestingEditor.svelte` changes

Register `editor.onDidChangeCursorSelection`:
- When selection spans one or more lines (non-empty), show a floating `+ Add to chat` button positioned near the end of the selection.
- Register `Cmd+L` (Mac) / `Ctrl+L` (Win/Linux) as a Monaco keybinding that triggers the same action without the button click.

On activate (button click or `Cmd+L`):
1. Read `model.getValueInRange(selection)` — not needed for content (resolved at send time).
2. Read `selection.startLineNumber`, `selection.endLineNumber`.
3. Read active tab `view.title` from session store.
4. Set `composerStore.pendingChip = { path: title, lineStart, lineEnd }`.
5. Focus the agent composer input.

`AgentComposer.svelte` watches `composerStore.pendingChip` via `$effect`. When non-null: insert a `FileChipNode` with the line range, clear `pendingChip`.

---

## File Renaming

`EditorTabs.svelte`: double-click on a tab title puts it into edit mode — the title text is replaced with an `<input>` pre-filled with the current title. `Enter` or blur confirms. `Escape` cancels. On confirm: call `session.renameView(viewId, newTitle)`, add `renameView` method to `Session` in `session.svelte.ts`.

---

## What the Agent Sees (System Prompt Additions)

Added to `buildToolInstructions()` in `tools.ts`:

```
CONTEXT: The user may attach files, table schemas, or query results as context in <context> blocks
at the start of their message. Read these carefully — they are the exact content the user wants
you to work with.

FILE WRITES: Never output SQL or code in your chat response. Use write_file instead:
  curl -s -X POST http://127.0.0.1:PORT/db/SESSION_ID/write_file \
    -d '{"fileName": "descriptive-name.sql", "content": "..."}'

If the user tagged a file with @ in their message, use that exact filename to update it in place.
Choose descriptive filenames (e.g. "find-null-users.sql", "orders-30d-analysis.sql").
```

---

## Data Flow Summary

```
User highlights lines 13-16 in orders.sql
  → Monaco selection event → floating "+ Add to chat" button
  → Click (or Cmd+L) → composerStore.pendingChip set
  → AgentComposer inserts FileChipNode { path: "orders.sql", lineStart: 13, lineEnd: 16 }

User types "why is this join slow?" and hits Enter
  → AgentComposer resolves chips:
      file-chip → slice session tab content lines 13-16
  → Assembles: <context><file ...>...</file></context>\n\nwhy is this join slow?
  → POST /session/send

Agent receives message, thinks, calls write_file
  → content_block_start → tab "orders-fix.sql" created immediately (empty)
  → input_json_delta ×N → partial content extracted → tab content streams in live
  → content_block_stop → tab content finalised
  → tool.completed → ToolCallCard shows "✓ wrote orders-fix.sql — 12 lines  ↗"
  → Agent chat message: "Updated the join to use an index hint — see orders-fix.sql"
```
