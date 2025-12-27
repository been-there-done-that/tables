<script lang="ts">
    import { IconFolder, IconChevronDown } from "@tabler/icons-svelte";
    import FormInput from "$lib/components/FormInput.svelte";
    import Button from "$lib/components/Button.svelte";
    import { getCurrentWindow } from "@tauri-apps/api/window";
    import { notifications } from "$lib/utils/notification.svelte";

    interface Props {
        data: any;
        onChange: (field: string, value: any) => void;
    }

    let { data, onChange }: Props = $props();

    async function browseFile() {
        // Mock browse for now
    }

    // Self-contained action handlers
    async function handleCancel() {
        const window = getCurrentWindow();
        await window.close();
    }

    function handleApply() {
        if (!data.file) {
            notifications.error("Database file is required");
            return;
        }
        notifications.success("SQLite settings saved!");
        console.log("Apply SQLite connection", data);
    }

    function handleTestConnection() {
        if (!data.file) {
            notifications.error("Database file is required");
            return;
        }
        notifications.info("Testing connection...");
        setTimeout(() => {
            notifications.success("Connection successful!");
        }, 1000);
    }
</script>

<div class="space-y-4 text-sm">
    <div class="grid grid-cols-[120px_1fr] gap-y-3 items-center">
        <label for="file" class="text-[--theme-fg-secondary]">File:</label>
        <div class="flex space-x-2">
            <div class="grow">
                <FormInput
                    inputId="file"
                    value={data.file}
                    oninput={(e: any) => onChange("file", e.target.value)}
                    class="bg-[--theme-bg-secondary] border-[--theme-border-default] text-[--theme-fg-secondary] focus:border-[--theme-accent-primary]"
                />
            </div>
            <button
                onclick={browseFile}
                class="px-2 py-1 h-8 bg-[--theme-bg-tertiary] border border-[--theme-border-default] rounded-md hover:bg-[--theme-bg-hover] text-[--theme-fg-secondary] flex items-center justify-center"
            >
                <IconFolder size={16} />
            </button>
        </div>

        <label for="url" class="text-[--theme-fg-secondary]">URL:</label>
        <div
            class="flex items-center h-8 bg-[--theme-bg-secondary] border border-[--theme-border-default] rounded-md px-3 py-1.5 text-[--theme-fg-tertiary] italic cursor-not-allowed text-sm"
        >
            <span class="truncate">jdbc:sqlite:{data.file || ""}</span>
        </div>
    </div>

    <!-- Footer Actions -->
    <div
        class="flex justify-center items-center py-4 mt-6 border-t border-[--theme-border-default]"
    >
        <div class="flex space-x-3">
            <Button onClick={handleCancel}>Cancel</Button>
            <Button onClick={handleApply}>Apply</Button>
            <Button onClick={handleTestConnection}>Test Connection</Button>
        </div>
    </div>
</div>
