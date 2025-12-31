<script lang="ts">
    import Table from "$lib/components/table/Table.svelte";
    import type { Column, ColumnType } from "$lib/components/table/types";

    // Generate dummy data
    function generateInitialData() {
        const data = [];
        for (let i = 0; i < 10000; i++) {
            data.push({
                _rowId: i,
                id: i,
                name: `User ${i}`,
                email: `user${i}@example.com`,
                role: i % 3 === 0 ? "Admin" : i % 3 === 1 ? "User" : "Guest",
                status: i % 2 === 0,
                last_login: new Date(
                    Date.now() - Math.random() * 10000000000,
                ).toISOString(),
                balance: (Math.random() * 10000).toFixed(2),
                notes: `Note for user ${i} with some long text to test overflow behavior`,
                metadata: JSON.stringify(
                    {
                        id: `meta_${i}`,
                        preferences: {
                            theme: i % 2 === 0 ? "dark" : "light",
                            notifications: {
                                email: true,
                                push: false,
                                sms: true,
                            },
                            dashboard: {
                                layout: "grid",
                                widgets: [
                                    "stats",
                                    "recent_activity",
                                    "logs",
                                    "graphs",
                                    "reports",
                                ],
                                refreshRate: 30,
                            },
                        },
                        history: Array.from({ length: 50 }, (_, j) => ({
                            action: `action_${j}`,
                            timestamp: new Date().toISOString(),
                            details: `Detailed log entry for action ${j} performed by user ${i}.`,
                        })),
                        tags: [
                            "user",
                            "active",
                            "test",
                            "scale",
                            "performance",
                            "json",
                            "data",
                        ],
                    },
                    null,
                    2,
                ),
            });
        }
        return data;
    }

    let rows: any[] = $state(generateInitialData());

    /**
     * Infer column definitions from data by analyzing the first N rows.
     * This removes the need for hardcoded column definitions.
     */
    function inferColumnsFromData(data: any[], sampleSize = 100): Column[] {
        if (!data.length) return [];

        const sample = data.slice(0, sampleSize);
        const keys = new Set<string>();
        const typeMap: Record<string, Set<string>> = {};

        // Collect all keys and their value types
        for (const row of sample) {
            for (const [key, value] of Object.entries(row)) {
                if (key === "_rowId") continue; // Skip internal key
                keys.add(key);
                if (!typeMap[key]) typeMap[key] = new Set();
                typeMap[key].add(typeof value);
            }
        }

        // Infer column type from collected types
        function inferType(key: string, sample: any[]): ColumnType {
            const firstValue = sample.find(
                (r) => r[key] !== null && r[key] !== undefined,
            )?.[key];
            if (firstValue === undefined) return "text";

            // Check for JSON strings
            if (typeof firstValue === "string") {
                try {
                    const parsed = JSON.parse(firstValue);
                    if (typeof parsed === "object") return "json";
                } catch {
                    // Not JSON
                }
                // Check for date/datetime patterns
                if (/^\d{4}-\d{2}-\d{2}T/.test(firstValue)) return "datetime";
                if (/^\d{4}-\d{2}-\d{2}$/.test(firstValue)) return "date";
            }

            if (typeof firstValue === "boolean") return "boolean";
            if (typeof firstValue === "number") {
                return Number.isInteger(firstValue) ? "int" : "float";
            }
            if (typeof firstValue === "object" && firstValue !== null)
                return "json";

            return "text";
        }

        // Check if column has enum-like values (limited distinct values)
        function isEnumLike(key: string, sample: any[]): boolean {
            const values = new Set(
                sample.map((r) => r[key]).filter((v) => v != null),
            );
            return (
                values.size > 1 &&
                values.size <= 10 &&
                values.size < sample.length / 2
            );
        }

        return Array.from(keys).map((key) => {
            let type = inferType(key, sample);

            // Override to enum if it looks like one
            if (type === "text" && isEnumLike(key, sample)) {
                type = "enum";
            }

            return {
                id: key,
                label:
                    key.charAt(0).toUpperCase() +
                    key.slice(1).replace(/_/g, " "),
                type,
                editable: key !== "id", // Primary key not editable
                sortable: type !== "json",
                filterable: true,
                pinnable: type !== "json",
            };
        });
    }

    // Simulate async fetch
    async function fetchData(params: any) {
        const { offset, limit, sort, filters } = params;

        let result = [...rows];

        // 1. Filter (basic implementation)
        if (filters && Object.keys(filters).length > 0) {
            Object.entries(filters).forEach(([columnId, filterValue]) => {
                if (!filterValue) return;
            });
        }

        // 2. Sort (basic implementation)
        if (sort && sort.length > 0) {
            // Add basic sort logic if needed
        }

        // 3. Paginate
        const sliced = result.slice(offset, offset + limit);

        // Infer columns from data on the fly
        const inferredColumns = inferColumnsFromData(rows);

        return {
            rows: sliced,
            total: rows.length,
            columns: inferredColumns,
        };
    }
</script>

<div
    class="flex flex-col h-full w-full bg-(--theme-bg-primary) text-(--theme-fg-primary)"
>
    <div
        class="flex-none p-4 border-b border-(--theme-border-default) flex justify-between items-center"
    >
        <h1 class="text-xl font-bold">Virtual Table Feature Parity Test</h1>
        <div class="text-sm text-(--theme-fg-tertiary)">
            10,000 Rows • Virtualized • Sorting • Filtering • Editing
        </div>
    </div>

    <div class="flex-1 min-h-0">
        <Table
            columns={[]}
            readOnly={false}
            dataFetcher={fetchData}
            tableName="test_users"
            onApplyEdits={(newRow) => {
                console.log("Edit completed", newRow);
                // Update local state
                const idx = rows.findIndex((r) => r._rowId === newRow._rowId);
                if (idx !== -1) {
                    rows[idx] = newRow;
                }
                return Promise.resolve();
            }}
        />
    </div>
</div>
