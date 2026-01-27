import type { ColumnType } from "./types";

/**
 * Normalizes a database column type to a frontend ColumnType.
 * Usage:
 * const type = normalizeColumnType('varchar', 'text');
 * const type = normalizeColumnType('_int4', 'int'); // Array detection
 */
export function normalizeColumnType(
    rawType: string,
    semanticHint?: string
): ColumnType {
    if (!rawType) return "text";

    // Normalize input
    const t = rawType.toLowerCase().trim();

    // 1. Check for Arrays
    // Postgres internal: _int4, _uuid
    // Standard SQL: integer[], text[]
    if (t.startsWith('_') || t.endsWith('[]') || t.includes('array')) {
        return "json"; // We display arrays as JSON
    }

    // 2. Use Semantic Hint if available (e.g. from SQLite heuristic)
    if (semanticHint) {
        const hint = semanticHint.toLowerCase();
        if (hint === 'json') return 'json';
        if (hint === 'uuid') return 'text'; // Keep UUID as text for now, or add specific uuid type if Cell supports it
        if (hint === 'boolean') return 'boolean';
        if (hint === 'datetime') return 'datetime';
        if (hint === 'date') return 'date';
        if (hint === 'time') return 'time';
    }

    // 3. Pattern Matching on Raw Type

    // Integers
    if (
        t.includes("int") ||
        t === "serial" ||
        t === "bigserial" ||
        t === "rowid"
    ) return "int";

    // Floats / Decimals
    if (
        t.includes("float") ||
        t.includes("double") ||
        t.includes("numeric") ||
        t.includes("decimal") ||
        t.includes("real")
    ) return "float";

    // Booleans
    if (
        t.includes("bool") ||
        t === "bit" // MS SQL 'bit' is often boolean
    ) return "boolean";

    // JSON
    if (
        t.includes("json") // json, jsonb
    ) return "json";

    // Dates & Times
    if (t.includes("timestamp") || t.includes("datetime")) return "datetime";
    if (t === "date") return "date";
    if (t.includes("time") && !t.includes("stamp")) return "time"; // Avoid timestamp matching 'time'

    // Binary / Blobs
    if (t.includes("bytea") || t.includes("blob") || t.includes("binary")) return "blob";

    // UUID
    if (t.includes("uuid")) return "text"; // Usually render as text

    // Default to text
    return "text";
}

/**
 * Ensures all columns have unique IDs and augments rows so data is accessible via those IDs.
 * This prevents Svelte 'each_key_duplicate' errors when SQL results have clashing column names.
 */
export function ensureUniqueColumnIds(
    rawColumns: any[],
    rawRows: any[]
): { columns: any[]; rows: any[] } {
    const seenIds = new Map<string, number>();

    // First pass: generate unique IDs for columns
    const columns = rawColumns.map((c) => {
        const name = c.name || c.id || "unnamed";
        const count = seenIds.get(name) || 0;
        const uniqueId = count === 0 ? name : `${name}_${count}`;
        seenIds.set(name, count + 1);

        // Keep a reference to the original name for data mapping
        return {
            ...c,
            id: uniqueId,
            label: c.label || name,
            _originalId: name,
        };
    });

    // Second pass: ensure row data is accessible via the unique IDs
    const rows = rawRows.map((row) => {
        let needsAdjustment = false;
        for (const col of columns) {
            if (col.id !== col._originalId) {
                needsAdjustment = true;
                break;
            }
        }

        if (!needsAdjustment) return row;

        const newRow = { ...row };
        for (const col of columns) {
            if (col.id !== col._originalId && newRow[col.id] === undefined) {
                newRow[col.id] = row[col._originalId];
            }
        }
        return newRow;
    });

    return { columns, rows };
}
