<script lang="ts">
    import { agentStore } from "$lib/agent/agent.svelte";
    import { onMount } from "svelte";
    import { Send, User, Bot, Trash2, Plus, Loader2 } from "lucide-svelte";
    import { slide } from "svelte/transition";

    let inputMessage = $state("");
    let chatContainer = $state<HTMLElement | null>(null);

    // Placeholder settings - in a real app these would be in a settings store
    let apiKey = $state("");
    let provider = $state("openai");
    let model = $state("gpt-4o");

    async function handleSend() {
        if (!inputMessage.trim() || agentStore.isStreaming) return;
        const msg = inputMessage;
        inputMessage = "";
        await agentStore.sendMessage(msg, provider, apiKey, model);
    }

    $effect(() => {
        if (agentStore.messages.length || agentStore.streamingContent) {
            scrollToBottom();
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
    class="flex h-[600px] w-full max-w-4xl flex-col border-4 border-black bg-white font-mono shadow-[8px_8px_0px_0px_rgba(0,0,0,1)]"
>
    <!-- Header -->
    <div
        class="flex items-center justify-between border-b-4 border-black bg-yellow-400 p-4"
    >
        <h2 class="text-xl font-black uppercase">Agent Console</h2>
        <div class="flex gap-2">
            <button
                onclick={() => agentStore.createSession()}
                class="border-2 border-black bg-white p-2 hover:bg-black hover:text-white transition-colors"
                title="New Chat"
            >
                <Plus size={18} />
            </button>
            <button
                onclick={() =>
                    agentStore.currentSession &&
                    agentStore.deleteSession(agentStore.currentSession.id)}
                class="border-2 border-black bg-white p-2 hover:bg-red-500 hover:text-white transition-colors"
                title="Delete Chat"
            >
                <Trash2 size={18} />
            </button>
        </div>
    </div>

    <div class="flex flex-1 overflow-hidden">
        <!-- Sidebar: Sessions -->
        <div class="w-64 border-r-4 border-black bg-gray-100 overflow-y-auto">
            {#each agentStore.sessions as session}
                <button
                    onclick={() => agentStore.selectSession(session)}
                    class="w-full border-b-2 border-black p-3 text-left hover:bg-yellow-200 transition-colors {agentStore
                        .currentSession?.id === session.id
                        ? 'bg-yellow-400 font-bold'
                        : ''}"
                >
                    <div class="truncate text-sm uppercase">
                        {session.title}
                    </div>
                    <div class="text-[10px] text-gray-500">
                        {new Date(session.updated_at * 1000).toLocaleString()}
                    </div>
                </button>
            {/each}
        </div>

        <!-- Chat Area -->
        <div class="flex flex-1 flex-col overflow-hidden bg-white">
            <!-- Messages -->
            <div
                bind:this={chatContainer}
                class="flex-1 overflow-y-auto p-4 space-y-4"
            >
                {#if agentStore.messages.length === 0 && !agentStore.isStreaming}
                    <div
                        class="flex h-full flex-col items-center justify-center space-y-4 text-gray-400"
                    >
                        <Bot size={64} />
                        <p class="uppercase font-bold">Initiate sequence...</p>
                    </div>
                {/if}

                {#each agentStore.messages as msg}
                    <div
                        class="flex flex-col {msg.role === 'user'
                            ? 'items-end'
                            : 'items-start'}"
                    >
                        <div class="flex items-center gap-2 mb-1 px-1">
                            {#if msg.role === "user"}
                                <span class="text-[10px] font-bold uppercase"
                                    >Operator</span
                                >
                                <User size={12} />
                            {:else if msg.role === "assistant"}
                                <Bot size={12} />
                                <span class="text-[10px] font-bold uppercase"
                                    >Agent</span
                                >
                            {:else if msg.role === "tool"}
                                <span
                                    class="text-[10px] font-bold uppercase text-blue-600"
                                    >Tool Result</span
                                >
                            {/if}
                        </div>
                        <div
                            class="max-w-[90%] border-2 border-black p-3 shadow-[4px_4px_0px_0px_rgba(0,0,0,1)]
                            {msg.role === 'user'
                                ? 'bg-blue-100'
                                : msg.role === 'tool'
                                  ? 'bg-gray-100 text-xs italic'
                                  : 'bg-green-100'}"
                        >
                            {msg.content}
                            {#if msg.tool_calls}
                                <div
                                    class="mt-2 text-[10px] border-t border-black pt-1 opacity-60"
                                >
                                    [TOOL CALLS DETECTED]
                                </div>
                            {/if}
                        </div>
                    </div>
                {/each}

                {#if agentStore.isStreaming && agentStore.streamingContent}
                    <div class="flex flex-col items-start" transition:slide>
                        <div class="flex items-center gap-2 mb-1 px-1">
                            <Bot size={12} />
                            <span class="text-[10px] font-bold uppercase"
                                >Agent</span
                            >
                            <span class="animate-pulse">_</span>
                        </div>
                        <div
                            class="max-w-[90%] border-2 border-black p-3 shadow-[4px_4px_0px_0px_rgba(0,0,0,1)] bg-green-100 whitespace-pre-wrap"
                        >
                            {agentStore.streamingContent}
                        </div>
                    </div>
                {/if}
            </div>

            <!-- Input Area -->
            <div class="border-t-4 border-black p-4 bg-gray-50">
                <div class="flex flex-col gap-2 mb-4">
                    <div class="flex gap-2 text-[10px]">
                        <input
                            type="password"
                            bind:value={apiKey}
                            placeholder="OPENAI_API_KEY"
                            class="flex-1 border-2 border-black p-1 bg-white"
                        />
                        <select
                            bind:value={model}
                            class="border-2 border-black p-1 bg-white"
                        >
                            <option value="gpt-4o">GPT-4O</option>
                            <option value="gpt-4-turbo">GPT-4-TURBO</option>
                        </select>
                    </div>
                </div>
                <div class="flex gap-2">
                    <textarea
                        bind:value={inputMessage}
                        onkeydown={(e) =>
                            e.key === "Enter" &&
                            !e.shiftKey &&
                            (e.preventDefault(), handleSend())}
                        placeholder="ENTER COMMAND..."
                        class="flex-1 resize-none border-4 border-black p-3 focus:outline-none focus:ring-2 focus:ring-yellow-400"
                        rows="1"
                    ></textarea>
                    <button
                        onclick={handleSend}
                        disabled={agentStore.isStreaming ||
                            !inputMessage.trim()}
                        class="border-4 border-black bg-yellow-400 p-4 font-black uppercase hover:bg-black hover:text-white disabled:bg-gray-300 disabled:text-gray-500 transition-colors"
                    >
                        {#if agentStore.isStreaming}
                            <Loader2 class="animate-spin" size={24} />
                        {:else}
                            <Send size={24} />
                        {/if}
                    </button>
                </div>
            </div>
        </div>
    </div>
</div>

<style>
    /* Custom scrollbar for brutalist look */
    ::-webkit-scrollbar {
        width: 12px;
    }
    ::-webkit-scrollbar-track {
        background: #eee;
        border-left: 2px solid black;
    }
    ::-webkit-scrollbar-thumb {
        background: #000;
        border: 2px solid white;
    }
</style>
