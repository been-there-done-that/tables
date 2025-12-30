<script lang="ts">
    import { onMount, onDestroy } from "svelte";
    import * as monaco from "monaco-editor";
    import { getWindowEditorPool, getMonacoHealth } from "./editor-pool";

    let snapshot = $state<any>(null);
    let interval: any;

    onMount(() => {
        const update = () => {
            try {
                const pool = getWindowEditorPool();
                snapshot = getMonacoHealth(pool, monaco);
            } catch (e) {
                // Pool not ready yet, perfectly fine for health panel
                snapshot = null;
            }
        };

        update();
        interval = setInterval(update, 1000);
    });

    onDestroy(() => {
        if (interval) clearInterval(interval);
    });

    function copyToClipboard() {
        if (!snapshot) return;
        navigator.clipboard.writeText(JSON.stringify(snapshot, null, 2));
        // Simple visual feedback
        const el = document.querySelector(".monaco-health");
        if (el) {
            el.classList.add("copied");
            setTimeout(() => el.classList.remove("copied"), 1000);
        }
    }
</script>

{#if import.meta.env.DEV}
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
        class="monaco-health"
        onclick={copyToClipboard}
        title="Click to copy state"
    >
        <div class="header">
            <h3>Monaco Health</h3>
            <span
                class="status {snapshot?.warnings?.length > 0
                    ? 'warning'
                    : 'ok'}"
            ></span>
        </div>

        {#if snapshot}
            <div class="metrics">
                <div class="section">
                    <strong>Editors:</strong>
                    {snapshot.editors.active} active / {snapshot.editors.idle} idle
                    ({snapshot.editors.total} total)
                </div>
                <div class="section">
                    <strong>Models:</strong>
                    {snapshot.models.sql} sql / {snapshot.models.json} json ({snapshot
                        .models.total} total)
                </div>
                {#if snapshot.layout}
                    <div class="section">
                        <strong>Layout:</strong>
                        {snapshot.layout.width}x{snapshot.layout.height} (LH: {snapshot
                            .layout.lineHeight}, CW: {snapshot.layout
                            .charWidth})
                    </div>
                {/if}
                {#if snapshot.warnings.length > 0}
                    <div class="warnings">
                        {#each snapshot.warnings as warning}
                            <div class="warning-item">⚠️ {warning}</div>
                        {/each}
                    </div>
                {/if}
            </div>
        {:else}
            <div class="metrics">
                <div class="section italic opacity-50">
                    Waiting for Monaco initialization...
                </div>
            </div>
        {/if}
    </div>
{/if}

<style>
    .monaco-health {
        position: fixed;
        bottom: 12px;
        right: 12px;
        background: rgba(15, 23, 42, 0.95);
        color: #94a3b8;
        font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas,
            "Liberation Mono", "Courier New", monospace;
        font-size: 11px;
        padding: 10px;
        border-radius: 6px;
        border: 1px solid #334155;
        box-shadow:
            0 4px 6px -1px rgba(0, 0, 0, 0.1),
            0 2px 4px -1px rgba(0, 0, 0, 0.06);
        z-index: 9999;
        min-width: 220px;
        cursor: pointer;
        transition: all 0.2s ease;
    }

    .monaco-health:hover {
        background: rgba(30, 41, 59, 1);
        border-color: #475569;
    }

    .header {
        display: flex;
        justify-content: space-between;
        align-items: center;
        margin-bottom: 8px;
        border-bottom: 1px solid #334155;
        padding-bottom: 4px;
    }

    h3 {
        margin: 0;
        font-size: 10px;
        text-transform: uppercase;
        letter-spacing: 0.05em;
        color: #e2e8f0;
    }

    .status {
        width: 8px;
        height: 8px;
        border-radius: 50%;
    }

    .status.ok {
        background: #22c55e;
    }
    .status.warning {
        background: #f59e0b;
        animation: pulse 2s infinite;
    }

    .monaco-health.copied {
        border-color: #22c55e;
        background: rgba(20, 83, 45, 0.95);
    }

    .metrics {
        display: flex;
        flex-direction: column;
        gap: 4px;
    }

    strong {
        color: #cbd5e1;
    }

    .warnings {
        margin-top: 6px;
        padding-top: 6px;
        border-top: 1px dashed #334155;
        color: #fca5a5;
    }

    @keyframes pulse {
        0% {
            opacity: 1;
        }
        50% {
            opacity: 0.5;
        }
        100% {
            opacity: 1;
        }
    }
</style>
