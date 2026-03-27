import { invoke } from "@tauri-apps/api/core";

export interface ToolContext {
    port: number;
    sessionId: string;
    connectionId: string;
    database: string;
    schema: string;
    openInEditor: (sql: string, title: string) => void;
}

/**
 * Dispatch a tool.started event: execute the tool via Tauri IPC and POST
 * the result back to the harness so the agent's curl call can complete.
 */
export async function dispatchTool(
    toolName: string,
    toolId: string,
    input: unknown,
    ctx: ToolContext,
): Promise<void> {
    let result: unknown;
    try {
        result = await executeTool(toolName, input, ctx);
    } catch (e) {
        result = { error: String(e) };
    }

    // POST result back to harness — unblocks the pending curl call
    try {
        await fetch(`http://127.0.0.1:${ctx.port}/tool-result/${toolId}`, {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify(result),
        });
    } catch (e) {
        console.error("[tool-executor] failed to post result:", e);
    }
}

async function executeTool(toolName: string, input: unknown, ctx: ToolContext): Promise<unknown> {
    const inp = input as Record<string, unknown>;
    const schema = (inp.schema as string | undefined) ?? ctx.schema;

    switch (toolName) {
        case "list_tables": {
            const result = await invoke<any>("get_schema", { connectionId: ctx.connectionId });
            const databases = result as any[];
            const db = databases?.find((d: any) => d.name === ctx.database);
            const sch = db?.schemas?.find((s: any) => s.name === schema);
            return (sch?.tables ?? []).map((t: any) => ({
                table_name: t.table_name,
                table_type: t.table_type,
                column_count: t.columns?.length ?? 0,
            }));
        }

        case "describe_table": {
            const details = await invoke<any>("get_schema_table_details", {
                connectionId: ctx.connectionId,
                database: ctx.database,
                schema,
                tableName: inp.table,
            });
            return (details?.columns ?? []).map((c: any) => ({
                column_name: c.column_name,
                type: c.raw_type,
                nullable: c.nullable,
                is_primary_key: c.is_primary_key,
                default_value: c.default_value ?? null,
            }));
        }

        case "get_indexes": {
            const details = await invoke<any>("get_schema_table_details", {
                connectionId: ctx.connectionId,
                database: ctx.database,
                schema,
                tableName: inp.table,
            });
            return details?.indexes ?? [];
        }

        case "get_foreign_keys": {
            const details = await invoke<any>("get_schema_table_details", {
                connectionId: ctx.connectionId,
                database: ctx.database,
                schema,
                tableName: inp.table,
            });
            return details?.foreign_keys ?? [];
        }

        case "run_query": {
            const limit = (inp.limit as number | undefined) ?? 100;
            const result = await invoke<any>("execute_query", {
                connectionId: ctx.connectionId,
                sessionId: "agent",
                database: ctx.database,
                schema,
                query: inp.sql,
                component: "agent",
                limit,
                offset: 0,
            });
            return {
                columns: (result?.columns ?? []).map((c: any) => c.name ?? c),
                rows: result?.rows ?? [],
                row_count: result?.rows?.length ?? 0,
                truncated: (result?.rows?.length ?? 0) >= limit,
            };
        }

        case "sample_table": {
            const n = (inp.n as number | undefined) ?? 20;
            const sql = `SELECT * FROM "${schema}"."${inp.table}" LIMIT ${n}`;
            const result = await invoke<any>("execute_query", {
                connectionId: ctx.connectionId,
                sessionId: "agent",
                database: ctx.database,
                schema,
                query: sql,
                component: "agent",
                limit: n,
                offset: 0,
            });
            return {
                columns: (result?.columns ?? []).map((c: any) => c.name ?? c),
                rows: result?.rows ?? [],
                row_count: result?.rows?.length ?? 0,
            };
        }

        case "count_rows": {
            const where = inp.where ? ` WHERE ${inp.where}` : "";
            const sql = `SELECT COUNT(*) AS count FROM "${schema}"."${inp.table}"${where}`;
            const result = await invoke<any>("execute_query", {
                connectionId: ctx.connectionId,
                sessionId: "agent",
                database: ctx.database,
                schema,
                query: sql,
                component: "agent",
                limit: 1,
                offset: 0,
            });
            return { count: result?.rows?.[0]?.count ?? result?.rows?.[0]?.[0] ?? 0 };
        }

        case "explain_query": {
            const analyze = (inp.analyze as boolean | undefined) ?? false;
            const prefix = analyze ? "EXPLAIN ANALYZE" : "EXPLAIN";
            const result = await invoke<any>("execute_query", {
                connectionId: ctx.connectionId,
                sessionId: "agent",
                database: ctx.database,
                schema,
                query: `${prefix} ${inp.sql}`,
                component: "agent",
                limit: 500,
                offset: 0,
            });
            const rows = result?.rows ?? [];
            return rows.map((r: any) => Object.values(r)[0]).join("\n");
        }

        case "column_stats": {
            const col = `"${inp.column}"`;
            const sql = `SELECT COUNT(*) AS total_count, COUNT(${col}) AS non_null_count, COUNT(DISTINCT ${col}) AS distinct_count, MIN(${col}::text) AS min_val, MAX(${col}::text) AS max_val FROM "${schema}"."${inp.table}"`;
            const result = await invoke<any>("execute_query", {
                connectionId: ctx.connectionId,
                sessionId: "agent",
                database: ctx.database,
                schema,
                query: sql,
                component: "agent",
                limit: 1,
                offset: 0,
            });
            const row = result?.rows?.[0] ?? {};
            const total = Number(row.total_count ?? 0);
            const nonNull = Number(row.non_null_count ?? 0);
            return {
                total_count: total,
                null_count: total - nonNull,
                null_pct: total > 0 ? (((total - nonNull) / total) * 100).toFixed(1) + "%" : "0%",
                distinct_count: row.distinct_count ?? 0,
                min_val: row.min_val,
                max_val: row.max_val,
            };
        }

        case "find_nulls": {
            const details = await invoke<any>("get_schema_table_details", {
                connectionId: ctx.connectionId,
                database: ctx.database,
                schema,
                tableName: inp.table,
            });
            const columns = (details?.columns ?? []).slice(0, 30);
            if (columns.length === 0) return [];
            const selects = columns
                .map((c: any) => `SUM(CASE WHEN "${c.column_name}" IS NULL THEN 1 ELSE 0 END) AS "${c.column_name}"`)
                .join(", ");
            const sql = `SELECT COUNT(*) AS __total, ${selects} FROM "${schema}"."${inp.table}"`;
            const result = await invoke<any>("execute_query", {
                connectionId: ctx.connectionId,
                sessionId: "agent",
                database: ctx.database,
                schema,
                query: sql,
                component: "agent",
                limit: 1,
                offset: 0,
            });
            const row = result?.rows?.[0] ?? {};
            const total = Number(row.__total ?? 1);
            return columns
                .map((c: any) => {
                    const nullCount = Number(row[c.column_name] ?? 0);
                    return {
                        column: c.column_name,
                        null_count: nullCount,
                        null_pct: ((nullCount / total) * 100).toFixed(1) + "%",
                    };
                })
                .filter((r: any) => r.null_count > 0);
        }

        case "get_distinct_values": {
            const limit = (inp.limit as number | undefined) ?? 20;
            const sql = `SELECT "${inp.column}" AS value, COUNT(*) AS count FROM "${schema}"."${inp.table}" GROUP BY "${inp.column}" ORDER BY count DESC LIMIT ${limit}`;
            const result = await invoke<any>("execute_query", {
                connectionId: ctx.connectionId,
                sessionId: "agent",
                database: ctx.database,
                schema,
                query: sql,
                component: "agent",
                limit,
                offset: 0,
            });
            return result?.rows ?? [];
        }

        case "check_fk_integrity": {
            const details = await invoke<any>("get_schema_table_details", {
                connectionId: ctx.connectionId,
                database: ctx.database,
                schema,
                tableName: inp.table,
            });
            const fks = details?.foreign_keys ?? [];
            if (fks.length === 0) return { message: "No foreign keys found on this table." };
            return fks.map((fk: any) => ({
                constraint_name: fk.constraint_name ?? fk.name ?? "unknown",
                column: fk.column_name ?? fk.column,
                references_table: fk.referenced_table ?? fk.foreign_table,
                references_column: fk.referenced_column ?? fk.foreign_column,
            }));
        }

        case "open_in_editor": {
            const sql = inp.sql as string;
            const title = (inp.title as string | undefined) ?? "Agent Query";
            ctx.openInEditor(sql, title);
            return { success: true };
        }

        case "get_query_history": {
            const limit = (inp.limit as number | undefined) ?? 20;
            const logs = await invoke<any[]>("fetch_query_logs", {
                limit,
                connectionId: ctx.connectionId,
            });
            return (logs ?? []).map((l: any) => ({
                sql: l.query ?? l.sql,
                executed_at: l.timestamp,
                duration_ms: l.duration_ms,
            }));
        }

        default:
            return { error: `Unknown tool: ${toolName}` };
    }
}
