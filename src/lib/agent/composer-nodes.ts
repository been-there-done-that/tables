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
        `display:inline-flex;align-items:center;gap:2px;border-radius:3px;padding:1px 4px;font-size:10.5px;font-weight:500;line-height:1.4;vertical-align:middle;user-select:none;cursor:default;margin:0 1px;position:relative;top:-0.5px;${bgClass}`;
    // SVG icon — safe, static string from constants
    const iconWrapper = document.createElement("span");
    iconWrapper.innerHTML = svg(iconType, iconColor);
    dom.appendChild(iconWrapper.firstElementChild!);

    // Label — set via textContent (safe, no XSS)
    const labelSpan = document.createElement("span");
    labelSpan.style.cssText = "max-width:140px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap";
    labelSpan.textContent = label;
    dom.appendChild(labelSpan);

    // Suffix — set via textContent (safe, no XSS)
    if (suffix) {
        const suffixSpan = document.createElement("span");
        suffixSpan.style.cssText = "opacity:0.55;font-size:10px";
        suffixSpan.textContent = suffix;
        dom.appendChild(suffixSpan);
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
        return ({ node }) => {
            const suffix = node.attrs.lineStart
                ? `(${node.attrs.lineStart}–${node.attrs.lineEnd})`
                : "";
            const dom = buildChipDom(
                "file",
                node.attrs.path,
                suffix,
                "background:rgba(59,130,246,0.22);border:1px solid rgba(59,130,246,0.45);color:#93c5fd",
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
                "background:rgba(168,85,247,0.22);border:1px solid rgba(168,85,247,0.45);color:#d8b4fe",
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
                "background:rgba(34,197,94,0.18);border:1px solid rgba(34,197,94,0.4);color:#86efac",
                "#86efac",
            );
            return { dom };
        };
    },
});
