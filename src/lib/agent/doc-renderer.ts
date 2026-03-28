/**
 * Renders a TipTap/ProseMirror doc JSON as an HTML string, preserving chip
 * nodes with the same visual style used in the composer input.
 * All user-controlled strings are HTML-escaped to prevent XSS.
 */

function esc(s: string): string {
    return String(s)
        .replace(/&/g, "&amp;")
        .replace(/</g, "&lt;")
        .replace(/>/g, "&gt;")
        .replace(/"/g, "&quot;");
}

// Tabler icon SVG paths (same as composer-nodes.ts)
const ICONS = {
    file: `<path stroke="none" d="M0 0h24v24H0z" fill="none"/><path d="M14 3v4a1 1 0 0 0 1 1h4"/><path d="M17 21h-10a2 2 0 0 1 -2 -2v-14a2 2 0 0 1 2 -2h7l5 5v11a2 2 0 0 1 -2 2z"/>`,
    table: `<path stroke="none" d="M0 0h24v24H0z" fill="none"/><path d="M3 5a2 2 0 0 1 2 -2h14a2 2 0 0 1 2 2v14a2 2 0 0 1 -2 2h-14a2 2 0 0 1 -2 -2v-14z"/><path d="M3 10h18"/><path d="M10 3v18"/>`,
    result: `<path stroke="none" d="M0 0h24v24H0z" fill="none"/><path d="M15 15m-4 0a4 4 0 1 0 8 0a4 4 0 1 0 -8 0"/><path d="M18.5 18.5l2.5 2.5"/><path d="M4 6h16"/><path d="M4 12h4"/><path d="M4 18h4"/>`,
};

function icon(type: keyof typeof ICONS, color: string): string {
    return `<svg xmlns="http://www.w3.org/2000/svg" width="11" height="11" viewBox="0 0 24 24" stroke="${esc(color)}" stroke-width="1.5" fill="none" stroke-linecap="round" stroke-linejoin="round" style="flex-shrink:0">${ICONS[type]}</svg>`;
}

function chip(
    type: keyof typeof ICONS,
    label: string,
    suffix: string,
    bg: string,
    border: string,
    color: string,
): string {
    const suffixHtml = suffix
        ? `<span style="opacity:0.55;font-size:10px">${esc(suffix)}</span>`
        : "";
    return `<span style="display:inline-flex;align-items:center;gap:2px;border-radius:3px;padding:1px 4px;font-size:10.5px;font-weight:500;line-height:1.4;vertical-align:middle;margin:0 1px;background:${bg};border:1px solid ${border};color:${color}">${icon(type, color)}<span style="max-width:140px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap">${esc(label)}</span>${suffixHtml}</span>`;
}

function walkNodes(nodes: unknown[]): string {
    let html = "";
    for (const node of nodes ?? []) {
        const n = node as Record<string, unknown>;
        if (n.type === "text") {
            html += esc(n.text as string);
        } else if (n.type === "fileChip") {
            const a = n.attrs as Record<string, unknown>;
            const label = String(a.path ?? "");
            const suffix = a.lineStart != null ? `(${a.lineStart}–${a.lineEnd})` : "";
            html += chip("file", label, suffix, "rgba(59,130,246,0.22)", "rgba(59,130,246,0.45)", "#93c5fd");
        } else if (n.type === "tableChip") {
            const a = n.attrs as Record<string, unknown>;
            html += chip("table", String(a.tableName ?? ""), "", "rgba(168,85,247,0.22)", "rgba(168,85,247,0.45)", "#d8b4fe");
        } else if (n.type === "resultChip") {
            const a = n.attrs as Record<string, unknown>;
            html += chip("result", String(a.label ?? ""), "", "rgba(34,197,94,0.18)", "rgba(34,197,94,0.4)", "#86efac");
        } else if (n.content) {
            html += walkNodes(n.content as unknown[]);
            if (["paragraph", "heading"].includes(n.type as string)) {
                html += "<br>";
            }
        }
    }
    return html;
}

export function renderDocAsHtml(doc: unknown): string {
    if (!doc || typeof doc !== "object") return "";
    const d = doc as Record<string, unknown>;
    const html = walkNodes(d.content as unknown[]);
    // Trim trailing <br> added by the last paragraph
    return html.replace(/<br>$/, "");
}
