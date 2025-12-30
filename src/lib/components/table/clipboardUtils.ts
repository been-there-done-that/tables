export interface ClipboardTypeHandler {
    format(value: any): string;
    parse(text: string): any;
}

class ClipboardRegistry {
    private handlers: Map<string, ClipboardTypeHandler> = new Map();
    private defaultHandler: ClipboardTypeHandler = {
        format: (value: any) => (value === null || value === undefined ? "" : String(value)),
        parse: (text: string) => text,
    };

    constructor() {
        this.registerDefaults();
    }

    register(type: string, handler: ClipboardTypeHandler) {
        this.handlers.set(type.toLowerCase(), handler);
    }

    get(type: string): ClipboardTypeHandler {
        return this.handlers.get(type.toLowerCase()) || this.defaultHandler;
    }

    format(value: any, type: string): string {
        return this.get(type).format(value);
    }

    parse(text: string, type: string): any {
        const trimmed = text.trim();
        if (trimmed === "") return null;
        return this.get(type).parse(trimmed);
    }

    private registerDefaults() {
        // JSON Handler
        this.register("json", {
            format: (value: any) => {
                if (typeof value === "object") {
                    return JSON.stringify(value);
                }
                return String(value);
            },
            parse: (text: string) => {
                try {
                    return JSON.parse(text);
                } catch (e) {
                    return text;
                }
            },
        });

        // Boolean Handler
        this.register("boolean", {
            format: (value: any) => (value ? "true" : "false"),
            parse: (text: string) => {
                const lower = text.toLowerCase();
                if (lower === "true") return true;
                if (lower === "false") return false;
                return Boolean(text);
            },
        });

        // Date/Datetime Handler
        const dateHandler: ClipboardTypeHandler = {
            format: (value: any) => {
                if (value instanceof Date) {
                    return value.toISOString();
                }
                return String(value);
            },
            parse: (text: string) => text, // Keep as string for now, or could parse to Date
        };
        this.register("date", dateHandler);
        this.register("datetime", dateHandler);

        // Number Handlers
        this.register("int", {
            format: (value: any) => String(value),
            parse: (text: string) => {
                const val = parseInt(text, 10);
                return isNaN(val) ? text : val;
            },
        });
        this.register("integer", this.get("int"));

        this.register("float", {
            format: (value: any) => String(value),
            parse: (text: string) => {
                const val = parseFloat(text);
                return isNaN(val) ? text : val;
            },
        });
        this.register("number", this.get("float"));
    }
}

export const clipboardRegistry = new ClipboardRegistry();

// Backward compatibility wrappers (optional, but good for transition)
export function formatValueForClipboard(value: any, type: string): string {
    return clipboardRegistry.format(value, type);
}

export function parseClipboardValue(text: string, type: string): any {
    return clipboardRegistry.parse(text, type);
}
