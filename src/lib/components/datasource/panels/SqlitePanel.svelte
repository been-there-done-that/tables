<script lang="ts">
    import ConfigField from "../fields/ConfigField.svelte";
    import ConfigInput from "../fields/ConfigInput.svelte";
    import FilePathField from "../fields/FilePathField.svelte";
    import AdvancedOptions from "../AdvancedOptions.svelte";

    let {
        config = $bindable({
            name: "",
            path: "",
            readOnly: false,
            mustExist: false,
            busyTimeout: 5000,
            foreignKeys: true,
        }),
    } = $props();

    let advancedOpen = $state(false);
</script>

<div class="space-y-1">
    <ConfigField label="Name" id="sq-name">
        <ConfigInput
            id="sq-name"
            bind:value={config.name}
            placeholder="e.g. production-db"
        />
    </ConfigField>

    <ConfigField label="Database File" id="sq-path">
        <FilePathField id="sq-path" bind:value={config.path} />
    </ConfigField>

    <AdvancedOptions bind:open={advancedOpen}>
        <ConfigField label="Busy Timeout" id="sq-timeout">
            <div class="flex items-center gap-2">
                <ConfigInput
                    id="sq-timeout"
                    type="number"
                    bind:value={config.busyTimeout}
                    class="w-20"
                />
                <span class="text-[10px] text-(--theme-fg-tertiary)">ms</span>
            </div>
        </ConfigField>

        <div class="space-y-2 pt-2">
            <label class="flex items-center gap-2 cursor-pointer group">
                <input
                    type="checkbox"
                    bind:checked={config.readOnly}
                    class="size-3 accent-(--theme-accent-primary)"
                />
                <span
                    class="text-xs text-(--theme-fg-secondary) group-hover:text-(--theme-fg-primary)"
                    >Read-only mode</span
                >
            </label>

            <label class="flex items-center gap-2 cursor-pointer group">
                <input
                    type="checkbox"
                    bind:checked={config.mustExist}
                    class="size-3 accent-(--theme-accent-primary)"
                />
                <span
                    class="text-xs text-(--theme-fg-secondary) group-hover:text-(--theme-fg-primary)"
                    >File must exist</span
                >
            </label>

            <label class="flex items-center gap-2 cursor-pointer group">
                <input
                    type="checkbox"
                    bind:checked={config.foreignKeys}
                    class="size-3 accent-(--theme-accent-primary)"
                />
                <span
                    class="text-xs text-(--theme-fg-secondary) group-hover:text-(--theme-fg-primary)"
                    >Enforce Foreign Keys</span
                >
            </label>
        </div>
    </AdvancedOptions>
</div>
