<script lang="ts">
    import Table from "$lib/components/table/Table.svelte";
    import type { Column, DataFetcher } from "$lib/components/table/types";
    import { settingsStore } from "$lib/stores/settings.svelte";

    // Mock columns
    const columns: Column[] = [
        {
            id: "id",
            label: "id",
            type: "int",
            editable: false,
            sortable: true,
            filterable: true,
            width: 80,
        },
        {
            id: "email",
            label: "email",
            type: "text",
            editable: false,
            sortable: true,
            filterable: true,
            width: 200,
        },
        {
            id: "total_orders",
            label: "total_orders",
            type: "int",
            editable: false,
            sortable: true,
            filterable: true,
            width: 120,
        },
        {
            id: "lifetime_value",
            label: "lifetime_value",
            type: "text",
            editable: false,
            sortable: true,
            filterable: true,
            width: 150,
        },
    ];

    // Mock data
    const mockRows = [
        {
            id: 1024,
            email: "alice@example.com",
            total_orders: 42,
            lifetime_value: "$4,250.00",
        },
        {
            id: 305,
            email: "bob.smith@work.net",
            total_orders: 12,
            lifetime_value: "$1,120.50",
        },
        {
            id: 77,
            email: "charlie@startup.io",
            total_orders: 5,
            lifetime_value: "$850.00",
        },
        {
            id: 892,
            email: "danielle@studio.art",
            total_orders: 0,
            lifetime_value: "NULL",
        },
    ];

    const dataFetcher: DataFetcher = async (params) => {
        return {
            rows: mockRows,
            total: mockRows.length,
            columns,
        };
    };

    let { fontFamily } = $props<{ fontFamily: string }>();
</script>

<div
    class="w-full h-full border border-border rounded-md overflow-hidden bg-background"
    style:font-family={fontFamily.includes(" ")
        ? `"${fontFamily}"`
        : fontFamily}
    style:font-size="{settingsStore.editorFontSize}px"
>
    <Table {columns} {dataFetcher} class="h-full w-full" />
</div>
