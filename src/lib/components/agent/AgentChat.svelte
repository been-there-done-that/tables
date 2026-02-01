<script lang="ts">
    import { agentStore } from "$lib/agent/agent.svelte";
    import { onMount } from "svelte";
    import { Send, Bot, Loader2 } from "lucide-svelte";
    import { slide } from "svelte/transition";
    import { settingsStore } from "$lib/stores/settings.svelte";
    import { cn } from "$lib/utils";

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
    }

    $effect(() => {
        if (agentStore.messages.length || agentStore.streamingContent) {
            scrollToBottom();
        }
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
                                : "bg-muted/50 text-foreground border-border",
                        )}
                    >
                        {msg.content}
                    </div>
                </div>
            {/each}

            {#if agentStore.isStreaming && agentStore.streamingContent}
                <div class="flex flex-col items-start" transition:slide>
                    <div class="flex items-center gap-2 mb-1 opacity-50">
                        <span
                            class="text-[9px] font-bold uppercase tracking-tighter"
                            >{settingsStore.aiAgentName}</span
                        >
                        <span class="animate-pulse">...</span>
                    </div>
                    <div
                        class="max-w-[90%] rounded-lg px-3 py-2 text-sm bg-muted/50 text-foreground border border-border whitespace-pre-wrap"
                    >
                        {agentStore.streamingContent}
                    </div>
                </div>
            {/if}
        </div>

        <!-- Input Area -->
        <div class="p-4 border-t border-border bg-muted/5">
            <div class="relative flex items-end gap-2">
                <textarea
                    bind:value={inputMessage}
                    onkeydown={(e) =>
                        e.key === "Enter" &&
                        !e.shiftKey &&
                        (e.preventDefault(), handleSend())}
                    placeholder="Ask anything..."
                    class="flex-1 min-h-[40px] max-h-32 resize-none bg-background border border-border rounded-xl p-3 text-sm focus:outline-none focus:ring-1 focus:ring-primary/30 transition-shadow"
                    rows="1"
                ></textarea>
                <button
                    onclick={handleSend}
                    disabled={agentStore.isStreaming || !inputMessage.trim()}
                    class="size-10 shrink-0 flex items-center justify-center bg-primary text-primary-foreground rounded-xl hover:opacity-90 disabled:opacity-30 disabled:grayscale transition-all shadow-sm"
                >
                    {#if agentStore.isStreaming}
                        <Loader2 class="animate-spin" size={18} />
                    {:else}
                        <Send size={18} />
                    {/if}
                </button>
            </div>
        </div>
    </div>
</div>
