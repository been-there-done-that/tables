<script lang="ts">
    import FileTree from "$lib/components/explorer/FileTree.svelte";

    const data = [
        {
            name: "src",
            type: "folder",
            children: [
                {
                    name: "routes",
                    type: "folder",
                    children: [
                        {
                            name: "demo",
                            type: "folder",
                            children: [{ name: "+page.svelte", type: "file" }],
                        },
                    ],
                },
                {
                    name: "lib",
                    type: "folder",
                    children: [
                        {
                            name: "components",
                            type: "folder",
                            children: [{ name: "Tree.svelte", type: "file" }],
                        },
                    ],
                },
            ],
        },
        {
            name: "database",
            type: "folder",
            children: [
                { name: "themes.db", type: "file" },
                {
                    name: "keys",
                    type: "folder",
                    children: [{ name: "api.key", type: "key" }],
                },
            ],
        },
    ];
</script>

<div class="p-8 w-full max-w-sm border h-full bg-background text-foreground">
    <h2 class="mb-4 font-bold">Tree Test</h2>
    <FileTree items={data} />
</div>
