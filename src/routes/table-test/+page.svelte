<script lang="ts">
    import Table from "$lib/components/table/Table.svelte";

    // Generate dummy data
    // Generate dummy data synchronously
    function generateInitialData() {
        const data = [];
        for (let i = 0; i < 10000; i++) {
            data.push({
                _rowId: i, // Reference table expects _rowId
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
                            details: `Detailed log entry for action ${j} performed by user ${i}. Contains extra padding text to make this JSON blob larger and ensure we are testing the performance of the editor with substantial content.`,
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

    const columns: any[] = [
        {
            id: "id",
            label: "ID",
            type: "int",
            width: 80,
            sortable: true,
            editable: false,
            filterable: true,
        },
        // ... (previous columns kept implicitly if I don't replace them, but I am replacing the array definition partially/fully?
        // Wait, replace_file_content replaces a block. I need to be careful with the range.
        // I'll define the metadata column in the columns array)

        {
            id: "name",
            label: "Name",
            type: "text",
            width: 200,
            sortable: true,
            editable: true,
            filterable: true,
        },
        {
            id: "email",
            label: "Email",
            type: "text",
            width: 250,
            sortable: true,
            editable: true,
            filterable: true,
        },
        {
            id: "role",
            label: "Role",
            type: "enum",
            width: 150,
            sortable: true,
            editable: true,
            filterable: true,
        },
        {
            id: "status",
            label: "Active",
            type: "boolean",
            width: 100,
            sortable: true,
            editable: true,
            filterable: true,
        },
        {
            id: "last_login",
            label: "Last Login",
            type: "datetime",
            width: 200,
            sortable: true,
            editable: false,
            filterable: true,
        },
        {
            id: "balance",
            label: "Balance",
            type: "float",
            width: 120,
            sortable: true,
            editable: true,
            filterable: true,
        },
        {
            id: "notes",
            label: "Notes",
            type: "text",
            width: 300,
            sortable: true,
            editable: true,
            filterable: true,
        },
        {
            id: "metadata",
            label: "Metadata",
            type: "json",
            width: 300,
            sortable: false,
            editable: true,
            filterable: true,
        },
    ];
    // Helper to fill defaults
    const fullColumns = columns.map((c) => ({
        editable: true,
        filterable: true,
        ...c,
    }));

    // Simulate async fetch
    async function fetchData(params: any) {
        const { offset, limit, sort, filters } = params;

        let result = [...rows];

        // 1. Filter
        if (filters && Object.keys(filters).length > 0) {
            Object.entries(filters).forEach(([columnId, filterValue]) => {
                if (!filterValue) return;
                // Add basic filtering logic if needed for test parity
            });
        }

        // 2. Sort
        if (sort && sort.length > 0) {
            // Add basic sort logic if needed
        }

        // 3. Paginate
        const sliced = result.slice(offset, offset + limit);

        return {
            rows: sliced,
            total: rows.length,
            columns: fullColumns,
        };
    }
</script>

<div class="flex flex-col h-full w-full bg-zinc-950 text-zinc-200">
    <div
        class="flex-none p-4 border-b border-zinc-800 flex justify-between items-center"
    >
        <h1 class="text-xl font-bold">Virtual Table Feature Parity Test</h1>
        <div class="text-sm text-zinc-400">
            10,000 Rows • Virtualized • Sorting • Filtering • Editing
        </div>
    </div>

    <div class="flex-1 min-h-0">
        <Table
            columns={fullColumns}
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
```
