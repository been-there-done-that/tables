// Database Helper Tools
// These are read-only tools for schema introspection

export interface ToolDefinition {
    type: "function";
    function: {
        name: string;
        description: string;
        parameters: {
            type: string;
            properties: Record<string, { type: string; description: string }>;
            required?: string[];
        };
    };
}

export const DATABASE_TOOLS: ToolDefinition[] = [
    {
        type: "function",
        function: {
            name: "get_table_schema",
            description: "Get column information (names, types, constraints) for a specific table. Use this when the user asks about a table's structure.",
            parameters: {
                type: "object",
                properties: {
                    table_name: {
                        type: "string",
                        description: "Name of the table to inspect"
                    },
                    schema_name: {
                        type: "string",
                        description: "Schema name (optional, defaults to 'public' for PostgreSQL)"
                    }
                },
                required: ["table_name"]
            }
        }
    },
    {
        type: "function",
        function: {
            name: "list_tables",
            description: "List all tables in the current database. Use this when the user asks what tables exist or wants an overview.",
            parameters: {
                type: "object",
                properties: {
                    schema_name: {
                        type: "string",
                        description: "Schema to filter by (optional)"
                    }
                }
            }
        }
    }
];

// Tool call interface matching OpenAI format
export interface ToolCall {
    id: string;
    type: "function";
    function: {
        name: string;
        arguments: string; // JSON string
    };
}

// Tool execution status for UI
export interface ToolExecutionStatus {
    name: string;
    status: "pending" | "success" | "error";
    result?: string;
    error?: string;
}
