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
        // Simple parameter derivation from OpenAPI operation
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

    async execute(toolCall: any): Promise<any> {
        const { name, arguments: argsJson } = toolCall.function;
        const tool = this.tools.get(name);
        if (!tool) throw new Error(`Unknown tool: ${name}`);

        const args = JSON.parse(argsJson);
        console.log(`Executing tool ${name} with args:`, args);

        // Here we would perform the actual network call.
        // For now, this is a placeholder for the logic that maps tool calls to API requests.
        // In a real implementation, this would involve fetching from the proxy or direct calls.

        return {
            status: "success",
            data: `Placeholder response for ${name}`
        };
    }
}

export const toolRunner = new ToolRunner();
