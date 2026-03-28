import { invoke } from "@tauri-apps/api/core";
import { windowState } from "$lib/stores/window.svelte";

export interface ToolContext {
    port: number;
    sessionId: string;
    connectionId: string;
    database: string;
    schema: string;
    openInEditor: (sql: string, title: string, autoRun?: boolean) => void;
    spawnSubagent?: (goal: string, model?: string) => Promise<string>;
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
            const sql = inp.sql as string;
            ctx.openInEditor(sql, "Query", true);
            return { opened: true, message: "Query opened in editor and running" };
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
            // Legacy alias — treat as write_file without running
            const sql = inp.sql as string;
            const title = (inp.title as string | undefined) ?? "Agent Query";
            const session = windowState.activeSession;
            if (!session) return { success: true };
            const newId = session.openView("editor", title, { content: sql });
            return { success: true, fileId: newId, fileName: title };
        }

        case "read_file": {
            const { fileId, fileName, lineStart, lineEnd } = inp as { fileId?: string; fileName?: string; lineStart?: number; lineEnd?: number };
            const session = windowState.activeSession;
            if (!session) return { error: "no active session" };
            const view = fileId
                ? session.views.find((v) => v.id === fileId)
                : session.views.find((v) => v.type === "editor" && v.title === fileName);
            if (!view) {
                const openFiles = session.views.filter(v => v.type === "editor").map(v => `${v.title} (id: ${v.id})`).join(", ") || "none";
                return { error: `File not found. Open files: ${openFiles}` };
            }
            let content: string = (view.data as Record<string, unknown>)?.content as string ?? "";
            const totalLines = content.split("\n").length;
            if (lineStart != null) {
                const end = lineEnd ?? lineStart;
                const lines = content.split("\n");
                content = lines.slice(lineStart - 1, end).join("\n");
                return { fileId: view.id, fileName: view.title, content, lineStart, lineEnd: end, totalLines };
            }
            return { fileId: view.id, fileName: view.title, content, lines: totalLines };
        }

        case "write_file": {
            const { fileId: targetId, fileName, content } = inp as { fileId?: string; fileName: string; content: string };
            const session = windowState.activeSession;
            if (!session) return { error: "no active session" };

            const existing = targetId
                ? session.views.find((v) => v.id === targetId)
                : session.views.find((v) => v.title === fileName);

            if (existing) {
                existing.data = existing.data ?? {};
                (existing.data as Record<string, unknown>).content = content;
                existing.streamingContent = undefined;
                if (fileName && existing.title !== fileName) existing.title = fileName;
                return { ok: true, action: "updated", fileId: existing.id, fileName: existing.title, lines: content.split("\n").length };
            } else {
                const newId = session.openView("editor", fileName, { content });
                return { ok: true, action: "created", fileId: newId, fileName, lines: content.split("\n").length };
            }
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

        case "spawn_subagent": {
            const goal = inp.goal as string | undefined;
            const model = inp.model as string | undefined;
            if (!goal) return { error: "spawn_subagent requires a 'goal' field" };
            if (!ctx.spawnSubagent) return { error: "spawn_subagent not available in this context" };
            try {
                const result = await ctx.spawnSubagent(goal, model);
                return { result };
            } catch (e) {
                return { error: String(e) };
            }
        }

        default:
            return { error: `Unknown tool: ${toolName}` };
    }
}
