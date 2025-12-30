import type { Column, EditorConfig } from "./types";

export function resolveEditor(
    column: Column,
    value: any,
    trigger?: string
): EditorConfig {
    // 1. Check if column has explicit editor config
    if (column.editor?.renderer && column.editor?.mode) {
        return {
            mode: column.editor.mode,
            renderer: column.editor.renderer,
            props: column.editor.props
        };
    }

    // 2. Default logic based on column type
    const type = column.type;

    // Special case: Enum -> always popover select
    if (type === "enum") {
        return {
            mode: "popover",
            renderer: "enum",
            props: { options: column.enumValues || [] }
        };
    }

    // Special case: Boolean -> inline checkbox/toggle (using popover for now until inline boolean is ready)
    if (type === "boolean") {
        return {
            mode: "popover", // Using popover for now to match current behavior
            renderer: "boolean"
        };
    }

    // Special case: JSON -> popover/modal
    if (type === "json" || type === "jsonb" || type === "JSON") {
        return {
            mode: "popover",
            renderer: "json"
        };
    }

    // Special case: Date/Time -> popover
    if (type === "date" || type === "time" || type === "datetime") {
        return {
            mode: "popover",
            renderer: "datetime",
            props: { mode: type === "datetime" ? "datetime" : "date" }
        };
    }

    // Special case: Number -> inline input (with type=number ideally, or text for now)
    if (type === "int" || type === "float") {
        return {
            mode: "inline",
            renderer: "number",
            props: { kind: type }
        };
    }

    // Default: Text -> Inline
    return {
        mode: "inline",
        renderer: "text"
    };
}
