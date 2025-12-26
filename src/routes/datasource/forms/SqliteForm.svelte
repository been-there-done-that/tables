<script lang="ts">
    import { IconFolder, IconChevronDown } from "@tabler/icons-svelte";
    // import { open } from '@tauri-apps/plugin-dialog'; // We will enable this later

    interface Props {
        data: any;
        onChange: (field: string, value: any) => void;
    }

    let { data, onChange }: Props = $props();

    async function browseFile() {
        // Mock browse for now, or just use window.alert if tauri APIs aren't ready to be called blindly
        // const selected = await open({
        //     multiple: false,
        //     filters: [{
        //         name: 'SQLite Database',
        //         extensions: ['db', 'sqlite', 'sqlite3']
        //     }]
        // });
        // if (selected) {
        //     onChange('file', selected);
        // }
    }
</script>

<div class="space-y-4 text-sm">
    <div class="grid grid-cols-[120px_1fr] gap-y-3 items-center">
        <label for="file" class="text-[#bbbbbb]">File:</label>
        <div class="flex space-x-2">
            <input
                type="text"
                id="file"
                value={data.file || ""}
                oninput={(e) => onChange("file", e.currentTarget.value)}
                class="flex-grow bg-[#2b2d30] border border-[#5e6060] rounded px-2 py-1 text-[#a9b7c6] focus:border-[#3574f0] focus:ring-1 focus:ring-[#3574f0] outline-none"
            />
            <button
                onclick={browseFile}
                class="px-2 py-1 bg-[#4c5052] border border-[#5e6060] rounded hover:bg-[#5c6062] text-[#bbbbbb]"
            >
                <IconFolder size={14} />
            </button>
        </div>

        <label for="url" class="text-[#bbbbbb]">URL:</label>
        <div
            class="flex items-center bg-[#2b2d30] border border-[#5e6060] rounded px-2 py-1 text-[#808080] italic cursor-not-allowed"
        >
            <span class="truncate">jdbc:sqlite:{data.file || ""}</span>
        </div>
    </div>
</div>
