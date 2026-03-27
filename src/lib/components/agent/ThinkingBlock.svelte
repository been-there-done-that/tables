<script lang="ts">
    import IconBrain from "@tabler/icons-svelte/icons/brain";
    import IconChevronDown from "@tabler/icons-svelte/icons/chevron-down";
    import IconChevronRight from "@tabler/icons-svelte/icons/chevron-right";
    import IconLoader from "@tabler/icons-svelte/icons/loader-2";

    interface Props {
        content: string;
        streaming?: boolean;
    }

    let { content, streaming = false }: Props = $props();
    let expanded = $state(false);

    const preview = $derived(content.slice(0, 120) + (content.length > 120 ? "…" : ""));
</script>

<div class="my-1 rounded-lg border border-border/60 bg-muted/5 text-[12px]">
    <button
        class="flex w-full items-center gap-2 px-3 py-2 text-left"
        onclick={() => (expanded = !expanded)}
    >
        {#if streaming}
            <IconLoader size={12} class="shrink-0 animate-spin text-accent" />
            <span class="flex-1 text-[11px] text-muted-foreground italic">Thinking…</span>
        {:else}
            <IconBrain size={12} class="shrink-0 text-muted-foreground" />
            <span class="flex-1 truncate text-[11px] text-muted-foreground italic">
                {expanded ? "Thought process" : preview}
            </span>
        {/if}
        {#if expanded}
            <IconChevronDown size={12} class="shrink-0 text-muted-foreground" />
        {:else}
            <IconChevronRight size={12} class="shrink-0 text-muted-foreground" />
        {/if}
    </button>

    {#if expanded}
        <div
            class="max-h-[400px] overflow-y-auto border-t border-border/60 px-3 py-2 text-[11px] text-muted-foreground"
        >
            {content}{#if streaming}<span class="streaming-cursor"></span>{/if}
        </div>
    {/if}
</div>

<style>
    .streaming-cursor {
        display: inline-block;
        width: 2px;
        height: 12px;
        vertical-align: middle;
        background: var(--theme-accent-primary);
        animation: blink 0.8s step-end infinite;
    }
    @keyframes blink {
        50% { opacity: 0; }
    }
</style>
