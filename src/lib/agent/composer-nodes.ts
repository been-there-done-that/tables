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
