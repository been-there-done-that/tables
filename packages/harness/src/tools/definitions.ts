import { tool } from "ai";
import { z } from "zod";
import type { Tool } from "ai";
import { callTool } from "../tool-bridge";
import type { HarnessEvent } from "../types";

/**
 * Create all DB tools as AI SDK tool() definitions.
 * Tools close over sessionId + emitFn so callTool() can route to the right SSE stream.
 */
export function createDbTools(
    sessionId: string,
    emitFn: (e: HarnessEvent) => void,
): Record<string, Tool> {
    const call = (name: string, args: unknown) => callTool(sessionId, name, args, emitFn);

    return {
        list_tables: tool({
            description: "List all tables in a database schema. Call this before writing queries to know what tables exist.",
            parameters: z.object({
                schema: z.string().optional().describe("Schema name. Default: public"),
            }),
            execute: (args) => call("list_tables", args),
        }),

        describe_table: tool({
            description: "Get columns, types, primary keys, nullable flags, and default values for a table. Always call this before writing a query against an unfamiliar table.",
            parameters: z.object({
                table: z.string().describe("Table name"),
                schema: z.string().optional().describe("Schema name. Default: public"),
            }),
            execute: (args) => call("describe_table", args),
        }),

        run_query: tool({
            description: "Execute SQL against the live database. Returns columns + up to 50 rows. May require user approval.",
            parameters: z.object({
                sql: z.string().describe("SQL query to execute"),
            }),
            execute: (args) => call("run_query", args),
        }),

        sample_table: tool({
            description: "Sample N rows from a table to understand its data shape.",
            parameters: z.object({
                table: z.string(),
                schema: z.string().optional(),
                n: z.number().int().positive().optional().describe("Number of rows to sample. Default: 20"),
            }),
            execute: (args) => call("sample_table", args),
        }),

        count_rows: tool({
            description: "Count rows in a table with an optional WHERE filter.",
            parameters: z.object({
                table: z.string(),
                schema: z.string().optional(),
                where: z.string().optional().describe("SQL WHERE clause (without the WHERE keyword)"),
            }),
            execute: (args) => call("count_rows", args),
        }),

        explain_query: tool({
            description: "Get the execution plan for a SQL query to check for performance issues.",
            parameters: z.object({
                sql: z.string(),
                analyze: z.boolean().optional().describe("Use EXPLAIN ANALYZE to include actual runtimes. Default: false"),
            }),
            execute: (args) => call("explain_query", args),
        }),

        get_indexes: tool({
            description: "List indexes on a table including their columns and uniqueness.",
            parameters: z.object({
                table: z.string(),
                schema: z.string().optional(),
            }),
            execute: (args) => call("get_indexes", args),
        }),

        get_foreign_keys: tool({
            description: "List foreign key constraints for a table, including referenced tables and columns.",
            parameters: z.object({
                table: z.string(),
                schema: z.string().optional(),
            }),
            execute: (args) => call("get_foreign_keys", args),
        }),

        column_stats: tool({
            description: "Get statistics for a column: NULL%, distinct count, min value, max value.",
            parameters: z.object({
                table: z.string(),
                column: z.string(),
                schema: z.string().optional(),
            }),
            execute: (args) => call("column_stats", args),
        }),

        find_nulls: tool({
            description: "Find all columns in a table that contain NULL values and report the null count per column.",
            parameters: z.object({
                table: z.string(),
                schema: z.string().optional(),
            }),
            execute: (args) => call("find_nulls", args),
        }),

        get_distinct_values: tool({
            description: "Get the top N most frequent distinct values for a column along with their counts.",
            parameters: z.object({
                table: z.string(),
                column: z.string(),
                schema: z.string().optional(),
                limit: z.number().int().positive().optional().describe("Max distinct values to return. Default: 20"),
            }),
            execute: (args) => call("get_distinct_values", args),
        }),

        check_fk_integrity: tool({
            description: "Check for orphaned rows — foreign key values that reference non-existent parent rows.",
            parameters: z.object({
                table: z.string(),
                schema: z.string().optional(),
            }),
            execute: (args) => call("check_fk_integrity", args),
        }),

        read_file: tool({
            description: "Read the content of an open editor tab. Use fileId (from a previous write_file response) for precise targeting.",
            parameters: z.object({
                fileId: z.string().optional().describe("File ID from a previous write_file response"),
                fileName: z.string().optional().describe("File name if fileId is not available"),
                lineStart: z.number().int().optional(),
                lineEnd: z.number().int().optional(),
            }),
            execute: (args) => call("read_file", args),
        }),

        write_file: tool({
            description: "Create or update an editor tab with SQL or other content. NEVER output SQL in your text response — always use write_file. Use fileId from a previous write_file response to update the same file without creating duplicates.",
            parameters: z.object({
                fileId: z.string().optional().describe("File ID to update an existing file precisely"),
                fileName: z.string().describe("Descriptive filename e.g. find-null-users.sql or orders-analysis.sql"),
                content: z.string().describe("Full file content"),
            }),
            execute: (args) => call("write_file", args),
        }),

        list_files: tool({
            description: "List all files you have created in this session. Call at the start of a task to recover fileIds and avoid creating duplicates.",
            parameters: z.object({}),
            execute: (args) => call("list_files", args),
        }),

        get_query_history: tool({
            description: "Get recent SQL queries from the editor history.",
            parameters: z.object({
                limit: z.number().int().positive().optional().describe("Number of recent queries to return. Default: 20"),
            }),
            execute: (args) => call("get_query_history", args),
        }),
    };
}
