import type { MetaDatabase } from "$lib/commands/types";

export function buildSystemPrompt(
    databases: MetaDatabase[],
    activeDb: string | null,
    engine: string | null,
    toolCtx?: { port: number; sessionId: string; schema: string },
    openTabs?: string[],
): string {
    const engineLabel = engine ?? "SQL";
    const dbLabel = activeDb ?? "unknown";
    const schemaSection = buildSchemaMarkdown(databases, activeDb);
    const toolSection = toolCtx
        ? buildToolInstructions(toolCtx.port, toolCtx.sessionId, toolCtx.schema)
        : "";
    const tabsSection = openTabs && openTabs.length > 0
        ? `## Open Editor Tabs\n\nThe user currently has these files open:\n${openTabs.map((t) => `- ${t}`).join("\n")}\n\nUse read_file to read a tab's current content, or write_file to update it.\n\n`
        : "";

    return `You are an expert ${engineLabel} database analyst integrated into Tables, a desktop database IDE.

Active connection: ${engineLabel} — database: "${dbLabel}"

${schemaSection}
${tabsSection}${toolSection}
Guidelines:
- NEVER output SQL or code directly in your chat response text — always use write_file to write or update files
- Be concise and precise
- When asked to write a query, use write_file immediately without preamble
- If a query could be destructive (DELETE, DROP, TRUNCATE), add a warning comment inside the file
- Prefer readable formatting with proper indentation
- If a request is ambiguous or could be interpreted multiple ways, ask one targeted clarifying question before proceeding`;
}

export function buildToolInstructions(port: number, sessionId: string, schema: string): string {
    const base = `http://127.0.0.1:${port}/db/${sessionId}`;
    return `
## Database Tools

Use the Bash tool with curl to call these endpoints. All are POST with JSON body.
Base URL: ${base}

| Tool | Body fields | Description |
|------|-------------|-------------|
| \`run_query\` | \`sql\` | Open query in editor tab and run it — results appear in the main results panel |
| \`sample_table\` | \`table\`, \`schema?\` (default "${schema}"), \`n?\` (default 20) | Sample N rows |
| \`count_rows\` | \`table\`, \`schema?\`, \`where?\` | COUNT with optional WHERE |
| \`explain_query\` | \`sql\`, \`analyze?\` (default false) | EXPLAIN plan |
| \`list_tables\` | \`schema?\` | All tables in schema with types |
| \`describe_table\` | \`table\`, \`schema?\` | Columns, types, PKs, nullable |
| \`get_indexes\` | \`table\`, \`schema?\` | Indexes on a table |
| \`get_foreign_keys\` | \`table\`, \`schema?\` | FK relationships |
| \`column_stats\` | \`table\`, \`column\`, \`schema?\` | NULL%, distinct count, min/max |
| \`find_nulls\` | \`table\`, \`schema?\` | Columns with unexpected NULLs |
| \`get_distinct_values\` | \`table\`, \`column\`, \`schema?\`, \`limit?\` (default 20) | Top N distinct values |
| \`check_fk_integrity\` | \`table\`, \`schema?\` | Orphaned FK rows |
| \`open_in_editor\` | \`sql\`, \`title?\` | Open SQL in editor tab |
| \`get_query_history\` | \`limit?\` (default 20) | Recent queries from editor |
| \`read_file\` | \`fileName\` | Read current content of an open editor tab |
| \`write_file\` | \`fileName\`, \`content\` | Write/update an editor tab with SQL content |

Example (open query in editor):
\`\`\`bash
curl -s -X POST ${base}/run_query \\
  -H 'Content-Type: application/json' \\
  -d '{"sql":"SELECT * FROM users LIMIT 50"}'
\`\`\`

Example (sample rows for analysis — use this when you need to see the data yourself):
\`\`\`bash
curl -s -X POST ${base}/sample_table \\
  -H 'Content-Type: application/json' \\
  -d '{"table":"users","n":20}'
\`\`\`

**Use tools proactively.** Before writing queries, call \`describe_table\` to know exact column names and types. Call \`sample_table\` to understand data shape. Chain tools freely.

## File Writing

IMPORTANT: Never output SQL or code in your chat response text. Always use write_file to write or update files.
If the user tagged a specific file with @ in their message, use that exact filename to update it in place.
Choose descriptive, lowercase filenames (e.g. "find-null-users.sql", "orders-30d-analysis.sql").

\`\`\`bash
curl -s -X POST ${base}/write_file \\
  -H 'Content-Type: application/json' \\
  -d '{"fileName": "descriptive-name.sql", "content": "SELECT ..."}'
\`\`\`

Response: {"ok": true, "action": "created"|"updated", "fileName": "...", "lines": N}

`;
}

function buildSchemaMarkdown(databases: MetaDatabase[], activeDb: string | null): string {
    if (databases.length === 0) return "Schema: not yet loaded.\n";

    const target = activeDb
        ? databases.find((d) => d.name === activeDb)
        : databases[0];

    if (!target) return `Schema: database "${activeDb}" not found.\n`;

    const lines: string[] = [`## Database: ${target.name}\n`];

    for (const schema of target.schemas ?? []) {
        if (!schema.tables || schema.tables.length === 0) continue;
        lines.push(`### Schema: ${schema.name}\n`);
        for (const table of schema.tables) {
            lines.push(`**${schema.name}.${table.table_name}**`);
            if (table.columns && table.columns.length > 0) {
                lines.push("| column | type | nullable |");
                lines.push("|--------|------|----------|");
                for (const col of table.columns.slice(0, 30)) {
                    lines.push(
                        `| ${col.column_name} | ${col.raw_type} | ${col.nullable ? "YES" : "NO"} |`,
                    );
                }
                if (table.columns.length > 30) {
                    lines.push(`| … (${table.columns.length - 30} more) | | |`);
                }
            }
            lines.push("");
        }
    }

    return lines.join("\n");
}
