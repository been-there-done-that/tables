// src/lib/components/table/copyFormats.ts

export type CopyFormat =
    | "plain"         // raw value (single cell) or TSV no-header (multi)
    | "tsv"           // tab-separated with header
    | "csv"           // comma-separated with header
    | "json"          // JSON array of objects, compact
    | "json_pretty"   // JSON array of objects, indented
    | "sql_insert"    // INSERT INTO table (...) VALUES (...)
    | "markdown"      // | col | col | table
    | "sql_where"     // WHERE col = val AND col = val
    | "sql_in"        // (val1, val2, val3)  — single column only
    | "column_names"; // space-separated column headers

export const COPY_FORMAT_LABELS: Record<CopyFormat, string> = {
    plain:        "Plain",
    tsv:          "TSV",
    csv:          "CSV",
    json:         "JSON",
    json_pretty:  "JSON (pretty)",
    sql_insert:   "SQL INSERT",
    markdown:     "Markdown",
    sql_where:    "WHERE condition",
    sql_in:       "IN list",
    column_names: "Column names",
};

export interface CopyColumn {
    id: string;
    label: string;
    type?: string; // ColumnType string from types.ts
}

export interface CopyOptions {
    tableName?: string; // required for sql_insert
}

// --- helpers ---

const NUMERIC_TYPES = new Set(["int", "float"]);

function isNumeric(col: CopyColumn): boolean {
    return NUMERIC_TYPES.has((col.type ?? "").toLowerCase());
}

function sqlQuote(value: any, col: CopyColumn): string {
    if (value === null || value === undefined) return "NULL";
    if (typeof value === "boolean") return value ? "TRUE" : "FALSE";
    if (isNumeric(col) && !isNaN(Number(value))) return String(value);
    const escaped = String(value).replace(/'/g, "''");
    return `'${escaped}'`;
}

function escapeMarkdown(s: string): string {
    return String(s).replace(/\|/g, "\\|");
}

// --- main export ---

/**
 * Serialise `rows` × `columns` into the requested copy format.
 * Returns the string to place on the clipboard.
 *
 * Throws if `format === "sql_in"` and `columns.length !== 1`.
 * Falls back to "results" if `format === "sql_insert"` and `options.tableName` is not provided.
 */
export function formatForCopy(
    rows: Record<string, any>[],
    columns: CopyColumn[],
    format: CopyFormat,
    options: CopyOptions = {},
): string {
    if (rows.length === 0 || columns.length === 0) return "";

    const tableName = options.tableName ?? "results";

    switch (format) {
        case "plain": {
            if (rows.length === 1 && columns.length === 1) {
                const v = rows[0][columns[0].id];
                return v === null || v === undefined ? "" : String(v);
            }
            // Fall through to TSV without header
            return rows
                .map((r) => columns.map((c) => {
                    const v = r[c.id];
                    return v === null || v === undefined ? "" : String(v);
                }).join("\t"))
                .join("\n");
        }

        case "tsv": {
            const header = columns.map((c) => c.label).join("\t");
            const body = rows.map((r) =>
                columns.map((c) => {
                    const v = r[c.id];
                    if (v === null || v === undefined) return "";
                    const s = String(v);
                    if (s.includes("\t") || s.includes("\n") || s.includes('"')) {
                        return `"${s.replace(/"/g, '""')}"`;
                    }
                    return s;
                }).join("\t")
            ).join("\n");
            return `${header}\n${body}`;
        }

        case "csv": {
            const quoteCSV = (v: any): string => {
                if (v === null || v === undefined) return "";
                const s = String(v);
                if (s.includes(",") || s.includes('"') || s.includes("\n")) {
                    return `"${s.replace(/"/g, '""')}"`;
                }
                return s;
            };
            const header = columns.map((c) => quoteCSV(c.label)).join(",");
            const body = rows.map((r) =>
                columns.map((c) => quoteCSV(r[c.id])).join(",")
            ).join("\n");
            return `${header}\n${body}`;
        }

        case "json": {
            const arr = rows.map((r) =>
                Object.fromEntries(columns.map((c) => [c.label, r[c.id] ?? null]))
            );
            return JSON.stringify(arr);
        }

        case "json_pretty": {
            const arr = rows.map((r) =>
                Object.fromEntries(columns.map((c) => [c.label, r[c.id] ?? null]))
            );
            return JSON.stringify(arr, null, 2);
        }

        case "sql_insert": {
            const colNames = columns.map((c) => `"${c.label}"`).join(", ");
            return rows
                .map((r) => {
                    const vals = columns.map((c) => sqlQuote(r[c.id], c)).join(", ");
                    return `INSERT INTO ${tableName} (${colNames}) VALUES (${vals});`;
                })
                .join("\n");
        }

        case "markdown": {
            const header = `| ${columns.map((c) => escapeMarkdown(c.label)).join(" | ")} |`;
            const sep    = `| ${columns.map(() => "---").join(" | ")} |`;
            const body   = rows.map((r) =>
                `| ${columns.map((c) => escapeMarkdown(
                    r[c.id] === null || r[c.id] === undefined ? "" : String(r[c.id])
                )).join(" | ")} |`
            ).join("\n");
            return `${header}\n${sep}\n${body}`;
        }

        case "sql_where": {
            const rowCondition = (r: Record<string, any>) =>
                columns
                    .map((c) => {
                        const v = r[c.id];
                        if (v === null || v === undefined) return `"${c.label}" IS NULL`;
                        return `"${c.label}" = ${sqlQuote(v, c)}`;
                    })
                    .join(" AND ");

            if (rows.length === 1) {
                return `WHERE ${rowCondition(rows[0])}`;
            }
            return `WHERE ${rows.map((r) => `(${rowCondition(r)})`).join("\nOR ")}`;
        }

        case "sql_in": {
            if (columns.length !== 1) {
                throw new Error("sql_in format requires exactly one column selected");
            }
            const col = columns[0];
            const vals = rows.map((r) => sqlQuote(r[col.id], col)).join(",\n");
            return `(\n${vals}\n)`;
        }

        case "column_names": {
            return columns.map((c) => c.label).join(" ");
        }
    }
}
