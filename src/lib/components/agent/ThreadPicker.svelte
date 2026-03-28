<!-- src/lib/components/agent/ThreadPicker.svelte -->
<script lang="ts">
    import * as Menu from "$lib/components/ui/dropdown-menu";
    import { threadsStore, type AgentThread } from "$lib/stores/threads.svelte";
    import IconChevronDown from "@tabler/icons-svelte/icons/chevron-down";
    import IconPlus from "@tabler/icons-svelte/icons/plus";
    import IconTrash from "@tabler/icons-svelte/icons/trash";

    interface Props {
        onNewThread: () => void;
        onSelectThread: (thread: AgentThread) => void;
    }

    let { onNewThread, onSelectThread }: Props = $props();

    const activeThread = $derived(threadsStore.activeThread);
    const activeThreadId = $derived(threadsStore.activeThreadId);
    const threads = $derived(threadsStore.threads);
    const title = $derived(activeThread?.title ?? "New session");

    let confirmingId = $state<string | null>(null);
</script>

<Menu.Root>
    <Menu.Trigger>
        <button
            class="flex max-w-[160px] items-center gap-1 truncate rounded px-1.5 py-1 text-[11px] font-medium text-foreground/70 transition-colors hover:bg-foreground/5 hover:text-foreground"
            title={title}
        >
            <span class="truncate">{title}</span>
            <IconChevronDown size={9} class="shrink-0 opacity-50" />
        </button>
    </Menu.Trigger>
    <Menu.Content
        class="w-56 border border-border bg-background shadow-md p-1"
        align="start"
        side="bottom"
    >
        {#each threads as thread (thread.id)}
            <Menu.Item
                class="group flex items-center justify-between gap-2 rounded px-2 py-1.5 text-[11px] cursor-pointer {thread.id === activeThreadId ? 'bg-accent/10 text-foreground' : 'text-foreground/70 hover:bg-foreground/5 hover:text-foreground'}"
                onclick={() => { if (confirmingId !== thread.id) onSelectThread(thread); }}
            >
                {#if confirmingId === thread.id}
                    <!-- Inline confirm state -->
                    <span class="text-[10.5px] text-foreground/60">Delete this session?</span>
                    <div class="flex shrink-0 items-center gap-1">
                        <button
                            onclick={(e) => { e.stopPropagation(); confirmingId = null; }}
                            class="rounded px-1.5 py-0.5 text-[10px] text-muted-foreground hover:bg-foreground/5"
                        >Cancel</button>
                        <button
                            onclick={(e) => { e.stopPropagation(); threadsStore.deleteThread(thread.id); confirmingId = null; }}
                            class="rounded px-1.5 py-0.5 text-[10px] text-destructive hover:bg-destructive/10"
                        >Delete</button>
                    </div>
                {:else}
                    <span class="truncate">{thread.title}</span>
                    <button
                        class="shrink-0 opacity-0 group-hover:opacity-60 hover:!opacity-100 text-destructive transition-opacity"
                        title="Delete session"
                        onclick={(e) => { e.stopPropagation(); confirmingId = thread.id; }}
                    >
                        <IconTrash size={10} />
                    </button>
                {/if}
            </Menu.Item>
        {/each}

        {#if threads.length > 0}
            <Menu.Separator class="my-1 border-t border-border/50" />
        {/if}

        <!-- New session at bottom -->
        <Menu.Item
            class="flex items-center gap-2 px-2 py-1.5 text-[11px] rounded cursor-pointer text-accent/70 hover:bg-accent/10 hover:text-accent"
            onclick={onNewThread}
        >
            <IconPlus size={11} />
            <span>New session</span>
        </Menu.Item>
    </Menu.Content>
</Menu.Root>
