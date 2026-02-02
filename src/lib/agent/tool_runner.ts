import { invoke } from "@tauri-apps/api/core";
import { schemaStore } from "$lib/stores/schema.svelte";
import type { ToolCall } from "./tools";

export interface ToolDefinition {
    type: "function";
    function: {
        name: string;
        description: string;
        parameters: any;
    };
}

export class ToolRunner {
    tools: Map<string, any> = new Map();

    /**
     * Execute a tool call from the LLM and report status to agentStore
     */
    async execute(toolCall: ToolCall): Promise<string> {
        console.log("[ToolRunner] execute() called with:", toolCall);

        // Dynamic import to avoid circular dependency
        const { agentStore } = await import("./agent.svelte");

        const toolId = toolCall.id || crypto.randomUUID();
        const toolName = toolCall.function.name;

        console.log("[ToolRunner] Starting execution of:", toolName, "id:", toolId);
        agentStore.startToolExecution(toolId, toolName);

        try {
            let result: string;
            const args = JSON.parse(toolCall.function.arguments || "{}");
            console.log("[ToolRunner] Parsed args:", args);

            switch (toolName) {
                case "get_table_schema":
                    result = await this.getTableSchema(args);
                    break;
                case "list_tables":
                    result = await this.listTables(args);
                    break;
                default:
                    throw new Error(`Unknown tool: ${toolName}`);
            }

            console.log("[ToolRunner] Tool result:", result.slice(0, 200));
            agentStore.completeToolExecution(toolId, result);
            return result;
        } catch (e) {
            const error = String(e);
            console.error("[ToolRunner] Tool error:", error);
            agentStore.failToolExecution(toolId, error);
            return `Error: ${error}`;
        }
    }

    /**
     * Get schema information for a specific table
     */
    private async getTableSchema(args: { table_name: string; schema_name?: string }): Promise<string> {
        const conn = schemaStore.activeConnection;
        if (!conn) {
            return "No active database connection";
        }

        try {
            // Try to get columns for the specified table
            const columns = await invoke("get_columns_for_table", {
                connectionId: conn.id,
                tableName: args.table_name,
                schemaName: args.schema_name || "public"
            });

            return JSON.stringify(columns, null, 2);
        } catch (e) {
            // Fallback: search in cached schema
            const databases = schemaStore.databases;
            for (const db of databases) {
                for (const schema of db.schemas) {
                    const table = schema.tables.find(t => t.table_name === args.table_name);
                    if (table) {
                        return JSON.stringify({
                            table_name: table.table_name,
                            schema: schema.name,
                            columns: table.columns?.map(c => ({
                                name: c.column_name,
                                type: c.raw_type,
                                nullable: c.nullable,
                                default: c.default_value
                            })) || []
                        }, null, 2);
                    }
                }
            }
            return `Table '${args.table_name}' not found in cached schema`;
        }
    }

    /**
     * List all tables in the current database
     */
    private async listTables(args: { schema_name?: string }): Promise<string> {
        const conn = schemaStore.activeConnection;
        if (!conn) {
            return "No active database connection";
        }

        const databases = schemaStore.databases;
        if (databases.length === 0) {
            return "No databases found in cache. Try refreshing the schema.";
        }

        const tables: { schema: string; table: string; type: string }[] = [];

        for (const db of databases) {
            for (const schema of db.schemas) {
                // Filter by schema if specified
                if (args.schema_name && schema.name !== args.schema_name) {
                    continue;
                }

                for (const table of schema.tables) {
                    tables.push({
                        schema: schema.name,
                        table: table.table_name,
                        type: table.table_type || "table"
                    });
                }
            }
        }

        if (tables.length === 0) {
            return args.schema_name
                ? `No tables found in schema '${args.schema_name}'`
                : "No tables found";
        }

        return JSON.stringify(tables, null, 2);
    }

    /**
     * Converts an OpenAPI spec (subset) to OpenAI function definitions.
     */
    fromOpenAPI(spec: any): ToolDefinition[] {
        const definitions: ToolDefinition[] = [];
        const paths = spec.paths || {};

        for (const [path, methods] of Object.entries(paths)) {
            for (const [method, operation] of Object.entries(methods as any)) {
                const op = operation as any;
                const name = op.operationId || `${method}_${path.replace(/\//g, "_")}`;

                definitions.push({
                    type: "function",
                    function: {
                        name: name,
                        description: op.description || op.summary || `Call ${method.toUpperCase()} ${path}`,
                        parameters: this.deriveParameters(op)
                    }
                });

                this.tools.set(name, { path, method, spec: op });
            }
        }

        return definitions;
    }

    private deriveParameters(operation: any) {
        const properties: any = {};
        const required: string[] = [];

        if (operation.parameters) {
            for (const param of operation.parameters) {
                properties[param.name] = {
                    type: param.schema?.type || "string",
                    description: param.description || ""
                };
                if (param.required) {
                    required.push(param.name);
                }
            }
        }

        if (operation.requestBody) {
            const content = operation.requestBody.content?.["application/json"];
            if (content?.schema?.properties) {
                Object.assign(properties, content.schema.properties);
                if (content.schema.required) {
                    required.push(...content.schema.required);
                }
            }
        }

        return {
            type: "object",
            properties,
            required: required.length > 0 ? required : undefined
        };
    }
}

export const toolRunner = new ToolRunner();
