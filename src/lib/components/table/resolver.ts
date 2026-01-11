import type { Column, EditorConfig } from "./types";

/**
 * Helper function to detect if a column type represents an array
 * Handles multiple database formats:
 * - PostgreSQL: uuid[], text[], integer[], etc.
 * - PostgreSQL internal: _uuid, _text, _int4, etc.
 * - Generic: types containing "array"
 */
function isArrayType(type: string): boolean {
    if (!type) return false;
    const lower = type.toLowerCase();

    // PostgreSQL array syntax: uuid[], text[], integer[]
    if (lower.includes('[]')) return true;

    // PostgreSQL internal array types: _uuid, _text, _int4
    // These start with underscore followed by base type
    if (lower.startsWith('_')) return true;

    // Generic array indicators
    if (lower.includes('array')) return true;

    return false;
}

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

    // Special case: Arrays -> JSON popover editor
    // This handles PostgreSQL arrays (uuid[], text[], _uuid), MySQL JSON arrays, and SQLite arrays
    if (isArrayType(type) || Array.isArray(value)) {
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

    // Number -> popover editor for consistent editing experience
    if (type === "int" || type === "float") {
        return {
            mode: "popover",
            renderer: "number-popover",
            props: { kind: type }
        };
    }

    // Default: Text -> always popover for consistent editing
    return {
        mode: "popover",
        renderer: "text-popover"
    };
}

