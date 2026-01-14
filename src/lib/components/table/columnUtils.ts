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
