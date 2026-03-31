<script lang="ts">
    import { exportStore, type ExportEntry, EXPORT_FORMAT_LABELS } from "$lib/stores/export.svelte";
    import IconCheck from "@tabler/icons-svelte/icons/check";
    import IconAlertTriangle from "@tabler/icons-svelte/icons/alert-triangle";
    import IconX from "@tabler/icons-svelte/icons/x";
    import IconLoader2 from "@tabler/icons-svelte/icons/loader-2";

    interface Props {
        entry: ExportEntry;
    }
    let { entry }: Props = $props();

    function formatBytes(b: number): string {
        if (b < 1024) return `${b} B`;
        if (b < 1024 * 1024) return `${(b / 1024).toFixed(1)} KB`;
        return `${(b / (1024 * 1024)).toFixed(1)} MB`;
    }

    function formatRate(bytes: number, ms: number): string {
        if (ms === 0) return "";
        const bps = (bytes / ms) * 1000;
        if (bps < 1024) return `${bps.toFixed(0)} B/s`;
        if (bps < 1024 * 1024) return `${(bps / 1024).toFixed(1)} KB/s`;
        return `${(bps / (1024 * 1024)).toFixed(1)} MB/s`;
    }

    function formatElapsed(ms: number): string {
        const s = Math.floor(ms / 1000);
        const m = Math.floor(s / 60);
        return `${String(m).padStart(2, "0")}:${String(s % 60).padStart(2, "0")}`;
    }

    const isActive = $derived(
        entry.status === "connecting" || entry.status === "executing" || entry.status === "streaming"
    );
    const isDone = $derived(entry.status === "done");
    const isError = $derived(entry.status === "error" || entry.status === "cancelled");

    const fileName = $derived(entry.filePath.split("/").pop() ?? entry.filePath);
    const rate = $derived(formatRate(entry.bytesWritten, entry.elapsedMs));
</script>

<div class="flex items-center gap-1.5 rounded-md border border-border bg-background px-2 py-1 text-xs">
    {#if isActive}
        <IconLoader2 class="h-3.5 w-3.5 animate-spin text-blue-400 shrink-0" />
        <span class="text-foreground font-mono">
            {entry.rowsWritten.toLocaleString()} rows
        </span>
        {#if rate}
            <span class="text-muted-foreground">{rate}</span>
        {/if}
        <span class="text-muted-foreground font-mono">{formatElapsed(entry.elapsedMs)}</span>
        <button
            class="ml-1 rounded hover:bg-accent p-0.5"
            title="Cancel export"
            onclick={() => exportStore.cancelExport(entry.exportId)}
        >
            <IconX class="h-3 w-3" />
        </button>
    {:else if isDone}
        <IconCheck class="h-3.5 w-3.5 text-green-500 shrink-0" />
        <span class="text-green-500">{EXPORT_FORMAT_LABELS[entry.format]}</span>
        <span class="text-muted-foreground truncate max-w-[140px]" title={entry.filePath}>{fileName}</span>
        <span class="text-muted-foreground">{entry.rowsWritten.toLocaleString()} rows</span>
        <button
            class="ml-1 rounded hover:bg-accent p-0.5 text-muted-foreground"
            title="Dismiss"
            onclick={() => exportStore.dismissExport(entry.exportId)}
        >
            <IconX class="h-3 w-3" />
        </button>
    {:else if isError}
        <IconAlertTriangle class="h-3.5 w-3.5 text-red-400 shrink-0" />
        <span class="text-red-400">
            {entry.status === "cancelled" ? "Cancelled" : "Export failed"}
        </span>
        <button
            class="ml-1 rounded hover:bg-accent p-0.5 text-muted-foreground"
            onclick={() => exportStore.dismissExport(entry.exportId)}
        >
            <IconX class="h-3 w-3" />
        </button>
    {/if}
</div>
