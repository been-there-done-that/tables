<script lang="ts">
    import { marked } from "marked";
    import type { AgentMessage } from "$lib/stores/agent.svelte";
    import { windowState } from "$lib/stores/window.svelte";
    import IconPlayerPlay from "@tabler/icons-svelte/icons/player-play-filled";
    import { renderDocAsHtml } from "$lib/agent/doc-renderer";

    interface Props {
        message: AgentMessage;
    }

    let { message }: Props = $props();

    const isUser = $derived(message.role === "user");

    // Parse markdown and inject "Run" buttons into SQL code blocks
    const html = $derived.by(() => {
        const renderer = new marked.Renderer();

        renderer.code = ({ text, lang }) => {
            const escaped = text.replace(/</g, "&lt;").replace(/>/g, "&gt;");
            const isSql = lang?.toLowerCase() === "sql";
            const runBtn = isSql
                ? `<button class="sql-run-btn" data-sql="${encodeURIComponent(text)}">▶ Run</button>`
                : "";
            return `<div class="code-block${isSql ? " code-block-sql" : ""}">
<pre><code class="language-${lang ?? ""}">${escaped}</code></pre>
${runBtn}
</div>`;
        };

        marked.use({ renderer });
        return marked.parse(message.content) as string;
    });

    function handleClick(e: MouseEvent) {
        // SQL run button in assistant messages
        const btn = (e.target as HTMLElement).closest(".sql-run-btn") as HTMLElement | null;
        if (btn) {
            const sql = decodeURIComponent(btn.dataset.sql ?? "");
            if (sql) openSqlInEditor(sql);
            return;
        }
        // @mention chip click in user messages
        const chip = (e.target as HTMLElement).closest(".agent-chip") as HTMLElement | null;
        if (chip) {
            const type = chip.dataset.chipType;
            const value = chip.dataset.chipValue ?? "";
            if (type === "file" && value) {
                const session = windowState.activeSession;
                const view = session?.views.find((v: { title: string }) => v.title === value);
                if (view) {
                    session!.activeViewId = view.id;
                } else {
                    session?.openView("editor", value, { content: "" });
                }
            }
        }
    }

    function openSqlInEditor(sql: string) {
        const session = windowState.activeSession;
        if (!session) return;
        session.openView("editor", "AI Query", { content: sql });
        // Make sure the SQL editor mode is active
        windowState.layout.showSqlEditor = false;
    }
</script>

<div class="group flex flex-col {isUser ? 'items-end' : 'items-start'} gap-1 px-3 py-1">
    {#if isUser}
        <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
        <div
            class="max-w-[85%] rounded-2xl rounded-tr-sm bg-accent/15 px-3 py-1.5 text-[12px] text-foreground leading-relaxed"
            onclick={handleClick}
        >
            {#if message.docJson}
                <!-- eslint-disable-next-line svelte/no-at-html-tags -->
                {@html renderDocAsHtml(message.docJson)}
            {:else}
                {message.content}
            {/if}
        </div>
    {:else if message.isError}
        <div class="mx-3 my-0.5 flex items-start gap-1.5 rounded-md border border-red-500/20 bg-red-500/8 px-2.5 py-1.5 text-[11.5px] text-red-400/80">
            <span class="mt-0.5 shrink-0">⚠</span>
            <span class="leading-relaxed">{message.content}</span>
        </div>
    {:else}
        <!-- eslint-disable-next-line svelte/no-at-html-tags -->
        <div
            class="agent-message w-full max-w-full text-[12px] text-foreground"
            onclick={handleClick}
            role="presentation"
        >
            {@html html}
            {#if message.streaming}
                <span class="streaming-cursor"></span>
            {/if}
        </div>
    {/if}
</div>

<style>
    .streaming-cursor {
        display: inline-block;
        width: 2px;
        height: 13px;
        vertical-align: middle;
        background: var(--theme-accent-primary);
        animation: blink 0.8s step-end infinite;
    }
    @keyframes blink {
        50% { opacity: 0; }
    }

    /* Markdown prose styles */
    :global(.agent-message p) {
        margin: 0 0 0.5em;
        line-height: 1.6;
    }
    :global(.agent-message p:last-child) {
        margin-bottom: 0;
    }
    :global(.agent-message code:not(pre code)) {
        background: var(--theme-bg-tertiary);
        border-radius: 3px;
        padding: 1px 4px;
        font-size: 12px;
        font-family: var(--font-mono);
    }
    :global(.agent-message .code-block) {
        position: relative;
        margin: 0.5em 0;
        border-radius: 6px;
        overflow: hidden;
        border: 1px solid var(--theme-border-default);
    }
    :global(.agent-message .code-block pre) {
        margin: 0;
        padding: 0.75rem;
        overflow-x: auto;
        background: var(--theme-bg-secondary);
    }
    :global(.agent-message .code-block code) {
        font-size: 12px;
        font-family: var(--font-mono);
        line-height: 1.5;
    }
    :global(.agent-message .sql-run-btn) {
        display: flex;
        align-items: center;
        gap: 4px;
        width: 100%;
        padding: 4px 10px;
        background: color-mix(in srgb, var(--theme-accent-primary) 8%, transparent);
        border-top: 1px solid var(--theme-border-subtle);
        color: var(--theme-accent-primary);
        font-size: 11px;
        cursor: pointer;
        transition: background 0.12s ease;
        text-align: left;
    }
    :global(.agent-message .sql-run-btn:hover) {
        background: color-mix(in srgb, var(--theme-accent-primary) 16%, transparent);
    }
    :global(.agent-message ul, .agent-message ol) {
        margin: 0.25em 0 0.5em 1.25em;
        padding: 0;
    }
    :global(.agent-message li) {
        margin-bottom: 0.15em;
    }
    :global(.agent-message h1, .agent-message h2, .agent-message h3) {
        font-weight: 600;
        margin: 0.75em 0 0.25em;
        line-height: 1.3;
    }
    :global(.agent-message h1) { font-size: 15px; }
    :global(.agent-message h2) { font-size: 14px; }
    :global(.agent-message h3) { font-size: 13px; }
    :global(.agent-message blockquote) {
        border-left: 2px solid var(--theme-border-default);
        margin: 0.5em 0;
        padding-left: 0.75em;
        color: var(--theme-fg-secondary);
    }
    :global(.agent-message table) {
        border-collapse: collapse;
        font-size: 12px;
        margin: 0.5em 0;
        width: 100%;
    }
    :global(.agent-message th, .agent-message td) {
        border: 1px solid var(--theme-border-default);
        padding: 4px 8px;
        text-align: left;
    }
    :global(.agent-message th) {
        background: var(--theme-bg-secondary);
        font-weight: 600;
    }
</style>
