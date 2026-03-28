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
    chipClass: string,
    iconColor: string,
    onRemove?: () => void,
): HTMLElement {
    const dom = document.createElement("span");
    dom.setAttribute("contenteditable", "false");
    dom.setAttribute("data-chip", iconType);
    dom.className = `agent-chip agent-chip-${chipClass}`;

    // SVG icon — safe, static string from constants
    const iconWrapper = document.createElement("span");
    iconWrapper.innerHTML = svg(iconType, iconColor);
    dom.appendChild(iconWrapper.firstElementChild!);

    // Label — set via textContent (safe, no XSS)
    const labelSpan = document.createElement("span");
    labelSpan.style.cssText = "overflow:hidden;text-overflow:ellipsis;white-space:nowrap";
    labelSpan.textContent = label;
    dom.appendChild(labelSpan);

    // Suffix — set via textContent (safe, no XSS)
    if (suffix) {
        const suffixSpan = document.createElement("span");
        suffixSpan.style.cssText = "opacity:0.55;font-size:10px";
        suffixSpan.textContent = suffix;
        dom.appendChild(suffixSpan);
    }

    // Remove (×) button
    if (onRemove) {
        const removeBtn = document.createElement("span");
        removeBtn.className = "chip-remove";
        removeBtn.textContent = "×";
        removeBtn.addEventListener("mousedown", (e) => {
            e.preventDefault();
            e.stopPropagation();
            onRemove();
        });
        dom.appendChild(removeBtn);
    }

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
        const label = (node.attrs.lineStart != null && node.attrs.lineEnd != null)
            ? `${node.attrs.path}:${node.attrs.lineStart}-${node.attrs.lineEnd}`
            : node.attrs.path;
        return [
            "span",
            mergeAttributes(HTMLAttributes, { "data-type": "file-chip" }),
            label,
        ];
    },

    addNodeView() {
        return ({ node, editor, getPos }) => {
            const suffix = node.attrs.lineStart
                ? `(${node.attrs.lineStart}–${node.attrs.lineEnd})`
                : "";
            const dom = buildChipDom(
                "file",
                node.attrs.path,
                suffix,
                "file",
                "currentColor",
                () => {
                    const pos = typeof getPos === "function" ? getPos() : null;
                    if (pos != null) editor.chain().focus().deleteRange({ from: pos, to: pos + node.nodeSize }).run();
                },
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
        return ({ node, editor, getPos }) => {
            const dom = buildChipDom(
                "table",
                node.attrs.tableName,
                "",
                "table",
                "currentColor",
                () => {
                    const pos = typeof getPos === "function" ? getPos() : null;
                    if (pos != null) editor.chain().focus().deleteRange({ from: pos, to: pos + node.nodeSize }).run();
                },
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
        return ({ node, editor, getPos }) => {
            const dom = buildChipDom(
                "result",
                node.attrs.label,
                "",
                "result",
                "currentColor",
                () => {
                    const pos = typeof getPos === "function" ? getPos() : null;
                    if (pos != null) editor.chain().focus().deleteRange({ from: pos, to: pos + node.nodeSize }).run();
                },
            );
            return { dom };
        };
    },
});
