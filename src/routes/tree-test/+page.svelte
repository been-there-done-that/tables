<script lang="ts">
    import FileTree, {
        type TreeNode,
    } from "$lib/components/explorer/FileTree.svelte";

    import Key from "@tabler/icons-svelte/icons/key";

    // Detailed Sakila DB Structure Mock
    const data: TreeNode[] = [
        {
            name: "sakila.db",
            type: "database",
            children: [
                {
                    name: "main",
                    type: "schema", // Using 'schema' for main
                    children: [
                        {
                            name: "tables",
                            type: "folder",
                            children: [
                                {
                                    name: "actor",
                                    type: "table",
                                    children: [
                                        {
                                            name: "columns",
                                            type: "folder",
                                            children: [
                                                {
                                                    name: "actor_id",
                                                    type: "column",
                                                    detail: "integer (auto increment)",
                                                },
                                                {
                                                    name: "first_name",
                                                    type: "column",
                                                    detail: "VARCHAR(45)",
                                                },
                                                {
                                                    name: "last_name",
                                                    type: "column",
                                                    detail: "VARCHAR(45)",
                                                },
                                                {
                                                    name: "last_update",
                                                    type: "column",
                                                    detail: "TIMESTAMP = CURRENT_TIMESTAMP",
                                                },
                                            ],
                                        },
                                        {
                                            name: "keys",
                                            type: "folder",
                                            children: [],
                                        },
                                        {
                                            name: "indexes",
                                            type: "folder",
                                            children: [],
                                        },
                                        {
                                            name: "triggers",
                                            type: "folder",
                                            children: [],
                                        },
                                    ],
                                },
                                {
                                    name: "address",
                                    type: "table",
                                    children: [
                                        {
                                            name: "columns",
                                            type: "folder",
                                            children: [
                                                {
                                                    name: "address_id",
                                                    type: "column",
                                                    detail: "int",
                                                },
                                                {
                                                    name: "address",
                                                    type: "column",
                                                    detail: "VARCHAR(50)",
                                                },
                                                {
                                                    name: "address2",
                                                    type: "column",
                                                    detail: "VARCHAR(50) = NULL",
                                                },
                                                {
                                                    name: "district",
                                                    type: "column",
                                                    detail: "VARCHAR(20)",
                                                },
                                                {
                                                    name: "city_id",
                                                    type: "column",
                                                    detail: "INT",
                                                },
                                                {
                                                    name: "postal_code",
                                                    type: "column",
                                                    detail: "VARCHAR(10) = NULL",
                                                },
                                                {
                                                    name: "phone",
                                                    type: "column",
                                                    detail: "VARCHAR(20)",
                                                },
                                                {
                                                    name: "last_update",
                                                    type: "column",
                                                    detail: "TIMESTAMP",
                                                },
                                            ],
                                        },
                                        {
                                            name: "keys",
                                            type: "folder",
                                            children: [
                                                {
                                                    name: "key #1",
                                                    type: "key",
                                                    detail: "(address_id)",
                                                },
                                            ],
                                        },
                                        {
                                            name: "foreign keys",
                                            type: "folder",
                                            children: [
                                                {
                                                    name: "fk_address_city",
                                                    type: "key",
                                                    detail: "(city_id) -> city (city_id)",
                                                },
                                            ],
                                        },
                                        {
                                            name: "indexes",
                                            type: "folder",
                                            children: [
                                                {
                                                    name: "sqlite_autoindex_address_1",
                                                    type: "index",
                                                    detail: "(address_id) UNIQUE",
                                                },
                                                {
                                                    name: "idx_fk_city_id",
                                                    type: "index",
                                                    detail: "(city_id)",
                                                },
                                            ],
                                        },
                                        {
                                            name: "triggers",
                                            type: "folder",
                                            children: [
                                                {
                                                    name: "address_trigger_ai",
                                                    type: "trigger",
                                                    detail: "after insert",
                                                },
                                                {
                                                    name: "address_trigger_au",
                                                    type: "trigger",
                                                    detail: "after update",
                                                },
                                            ],
                                        },
                                    ],
                                },
                            ],
                        },
                    ],
                },
            ],
        },
    ];
</script>

<div class="p-8 w-full max-w-sm border h-full bg-background text-foreground">
    <h2 class="mb-4 font-bold">Tree Test</h2>
    <FileTree items={data} indent={12} />
</div>
