<script lang="ts">
    import Table from "$lib/components/table/Table.svelte";
    import { type Column, type Row } from "$lib/components/table/types";

    // 1. Define Columns
    const columns: Column[] = [
        { id: "id", label: "ID", type: "int", width: 80, sortable: true },
        { id: "name", label: "Name", type: "text", width: 200, sortable: true },
        {
            id: "email",
            label: "Email",
            type: "text",
            width: 250,
            sortable: true,
        },
        { id: "role", label: "Role", type: "enum", width: 150, sortable: true },
        {
            id: "active",
            label: "Active",
            type: "boolean",
            width: 100,
            sortable: true,
        },
        {
            id: "lastLogin",
            label: "Last Login",
            type: "datetime",
            width: 200,
            sortable: true,
        },
        {
            id: "balance",
            label: "Balance",
            type: "float",
            width: 120,
            sortable: true,
        },
        {
            id: "notes",
            label: "Notes",
            type: "text",
            width: 300,
            sortable: true,
        },
    ];

    // 2. Generate 10,000 Rows
    const rows: Row[] = Array.from({ length: 10000 }, (_, i) => ({
        _rowId: i,
        id: i,
        name: `User ${i}`,
        email: `user${i}@example.com`,
        role: i % 3 === 0 ? "Admin" : i % 3 === 1 ? "Editor" : "Viewer",
        active: i % 2 === 0,
        lastLogin: new Date(
            Date.now() - Math.random() * 10000000000,
        ).toISOString(),
        balance: (Math.random() * 10000).toFixed(2),
        notes: `Validation check for row ${i} with some long text to test clipping`,
    }));
</script>

<div class="h-screen w-full p-4 flex flex-col gap-4">
    <div class="flex items-center justify-between">
        <h1 class="text-xl font-bold">Table Performance Test (100k Rows)</h1>
        <div class="text-sm text-muted-foreground">
            Svelte 5 Runes + Virtualization
        </div>
    </div>

    <div class="flex-1 min-h-0">
        <Table data={rows} {columns} />
    </div>
</div>
