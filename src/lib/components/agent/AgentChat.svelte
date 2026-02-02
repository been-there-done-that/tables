<script lang="ts">
    import { agentStore } from "$lib/agent/agent.svelte";
    import { onMount, tick } from "svelte";
    import { Send, Bot, Loader2 } from "lucide-svelte";
    import { slide } from "svelte/transition";
    import { settingsStore } from "$lib/stores/settings.svelte";
    import { cn } from "$lib/utils";
    import ToolExecutionIndicator from "./ToolExecutionIndicator.svelte";
    import { marked } from "marked";

    // Configure marked for safe rendering
    marked.setOptions({
        breaks: true,
        gfm: true,
    });

    function renderMarkdown(content: string | null): string {
        if (!content) return "";
        return marked.parse(content) as string;
    }

    let inputMessage = $state("");
    let chatContainer = $state<HTMLElement | null>(null);
    let selectedModel = $state(settingsStore.aiAgentModel);

    async function handleSend() {
        if (!inputMessage.trim() || agentStore.isStreaming) return;
        const msg = inputMessage;
        inputMessage = "";
        await agentStore.sendMessage(
            msg,
            "openai",
            settingsStore.aiAgentApiKey,
            selectedModel,
            settingsStore.aiAgentUrl,
        );
        await tick();
        scrollToBottom();
    }

    $effect(() => {
        // Track dependencies specifically
        const _len = agentStore.messages.length;
        const _stream = agentStore.streamingContent;
        const _isStreaming = agentStore.isStreaming;

        // Use tick to ensure DOM is updated before scrolling
        tick().then(() => {
            scrollToBottom();
        });
    });

    // Update selectedModel if it changes in settings
    $effect(() => {
        if (settingsStore.aiAgentModel) {
            selectedModel = settingsStore.aiAgentModel;
        }
    });

    function scrollToBottom() {
        if (chatContainer) {
            chatContainer.scrollTop = chatContainer.scrollHeight;
        }
    }

    onMount(() => {
        scrollToBottom();
    });
</script>

<div
    class="flex flex-col h-full w-full bg-background border-l border-border font-sans"
>
    <div class="flex flex-1 flex-col overflow-hidden bg-background">
        <!-- Messages -->
        <div
            bind:this={chatContainer}
            class="flex-1 overflow-y-auto p-4 space-y-6"
        >
            {#if agentStore.messages.length === 0 && !agentStore.isStreaming}
                <div
                    class="flex h-full flex-col items-center justify-center space-y-2 opacity-20"
                >
                    <Bot size={48} />
                    <p class="text-[10px] uppercase font-bold tracking-widest">
                        Ready for input
                    </p>
                </div>
            {/if}

            {#each agentStore.messages as msg}
                <div
                    class="flex flex-col {msg.role === 'user'
                        ? 'items-end'
                        : 'items-start'}"
                >
                    <div class="flex items-center gap-2 mb-1 opacity-50">
                        {#if msg.role === "user"}
                            <span
                                class="text-[9px] font-bold uppercase tracking-tighter"
                                >You</span
                            >
                        {:else if msg.role === "assistant"}
                            <span
                                class="text-[9px] font-bold uppercase tracking-tighter"
                                >{settingsStore.aiAgentName}</span
                            >
                        {/if}
                    </div>
                    <div
                        class={cn(
                            "max-w-[90%] rounded-lg px-3 py-2 text-sm shadow-sm border",
                            msg.role === "user"
                                ? "bg-primary text-primary-foreground border-primary/20"
                                : "bg-muted/50 text-foreground border-border chat-markdown",
                        )}
                    >
                        {@html renderMarkdown(msg.content)}
                    </div>
                </div>
            {/each}

            {#if agentStore.isStreaming}
                <div class="flex flex-col items-start">
                    <!-- Tool execution indicators -->
                    {#if agentStore.executingTools.size > 0}
                        <div class="flex flex-wrap gap-1 mb-2">
                            {#each [...agentStore.executingTools.entries()] as [id, tool]}
                                <ToolExecutionIndicator
                                    toolName={tool.name}
                                    status={tool.status}
                                    result={tool.result}
                                    error={tool.error}
                                />
                            {/each}
                        </div>
                    {/if}
                    <div class="flex items-center gap-2 mb-1 opacity-50">
                        <span
                            class="text-[9px] font-bold uppercase tracking-tighter"
                            >{settingsStore.aiAgentName}</span
                        >
                    </div>
                    <div
                        class="max-w-[90%] rounded-lg px-3 py-2 text-sm bg-muted/50 text-foreground border border-border whitespace-pre-wrap min-h-[38px] flex items-center"
                    >
                        {#if agentStore.streamingContent}
                            <div class="chat-markdown">
                                {@html renderMarkdown(
                                    agentStore.streamingContent,
                                )}<span
                                    class="inline-block w-1.5 h-3.5 ml-0.5 align-middle bg-current animate-pulse"
                                ></span>
                            </div>
                        {:else}
                            <div class="flex space-x-1 opacity-50">
                                <div
                                    class="w-1.5 h-1.5 bg-current rounded-full animate-bounce [animation-delay:-0.3s]"
                                ></div>
                                <div
                                    class="w-1.5 h-1.5 bg-current rounded-full animate-bounce [animation-delay:-0.15s]"
                                ></div>
                                <div
                                    class="w-1.5 h-1.5 bg-current rounded-full animate-bounce"
                                ></div>
                            </div>
                        {/if}
                    </div>
                </div>
            {/if}
        </div>

        <!-- Input Area -->
        <div class="px-4 pb-4 bg-background">
            <div
                class="relative flex items-end border border-border rounded-xl bg-background focus-within:ring-1 focus-within:ring-primary/30 transition-shadow"
            >
                <textarea
                    bind:value={inputMessage}
                    onkeydown={(e) =>
                        e.key === "Enter" &&
                        !e.shiftKey &&
                        (e.preventDefault(), handleSend())}
                    placeholder="Ask anything..."
                    class="flex-1 min-h-[40px] max-h-32 resize-none bg-transparent border-none p-3 pr-10 text-sm focus:outline-none focus:ring-0"
                    rows="1"
                    style="field-sizing: content;"
                ></textarea>
                <div class="absolute right-2 bottom-2">
                    <button
                        onclick={handleSend}
                        disabled={agentStore.isStreaming ||
                            !inputMessage.trim()}
                        class="size-7 flex items-center justify-center bg-primary text-primary-foreground rounded-lg hover:opacity-90 disabled:opacity-0 transition-all shadow-sm"
                        title="Send message"
                    >
                        <Send size={14} />
                    </button>
                </div>
            </div>
        </div>
    </div>
</div>

<style>
    /* Markdown prose styles for chat messages */
    :global(.chat-markdown p) {
        margin: 0.5em 0;
    }
    :global(.chat-markdown p:first-child) {
        margin-top: 0;
    }
    :global(.chat-markdown p:last-child) {
        margin-bottom: 0;
    }
    :global(.chat-markdown code) {
        background: rgba(0, 0, 0, 0.1);
        padding: 0.15em 0.3em;
        border-radius: 3px;
        font-size: 0.9em;
        font-family: "SF Mono", Monaco, "Cascadia Code", monospace;
    }
    :global(.chat-markdown pre) {
        background: rgba(0, 0, 0, 0.15);
        padding: 0.75em 1em;
        border-radius: 6px;
        overflow-x: auto;
        margin: 0.5em 0;
    }
    :global(.chat-markdown pre code) {
        background: transparent;
        padding: 0;
        font-size: 0.85em;
        line-height: 1.5;
    }
    :global(.chat-markdown ul, .chat-markdown ol) {
        margin: 0.5em 0;
        padding-left: 1.5em;
    }
    :global(.chat-markdown li) {
        margin: 0.25em 0;
    }
    :global(.chat-markdown strong) {
        font-weight: 600;
    }
    :global(.chat-markdown em) {
        font-style: italic;
    }
</style>
