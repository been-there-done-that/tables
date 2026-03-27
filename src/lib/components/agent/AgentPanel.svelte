<script lang="ts">
    import { onDestroy } from "svelte";
    import { schemaStore } from "$lib/stores/schema.svelte";
    import { agentStore } from "$lib/stores/agent.svelte";
    import { windowState } from "$lib/stores/window.svelte";
    import { startAgentSession, type AgentEventType } from "$lib/agent/claude";
    import { buildSystemPrompt } from "$lib/agent/tools";
    import { dispatchTool, type ToolContext } from "$lib/agent/tool-executor";
    import { harnessStore } from "$lib/stores/harness.svelte";
    import MessageList from "./MessageList.svelte";
    import AgentComposer from "./AgentComposer.svelte";
    import IconAi from "@tabler/icons-svelte/icons/ai";
    import IconAlertCircle from "@tabler/icons-svelte/icons/alert-circle";

    let abortController = $state<AbortController | null>(null);
    let sessionReady = $state(false);
    let sessionError = $state<string | null>(null);
    let sessionConnectionId = $state<string | null>(null);
    let sessionDatabase = $state<string | null>(null);
    let streamingMsgId: string | null = null;

    // Turn-level elapsed timer
    let turnStartedAt = $state<number | null>(null);
    let turnElapsed = $state(0);
    let turnTimerInterval: ReturnType<typeof setInterval> | null = null;

    function startTurnTimer() {
        turnStartedAt = Date.now();
        turnElapsed = 0;
        turnTimerInterval = setInterval(() => {
            if (turnStartedAt !== null) turnElapsed = Date.now() - turnStartedAt;
        }, 100);
    }

    function stopTurnTimer() {
        if (turnTimerInterval !== null) {
            clearInterval(turnTimerInterval);
            turnTimerInterval = null;
        }
        if (turnStartedAt !== null) turnElapsed = Date.now() - turnStartedAt;
    }

    function formatTurnElapsed(ms: number): string {
        if (ms < 1000) return `${ms}ms`;
        return `${(ms / 1000).toFixed(1)}s`;
    }

    async function bootSession() {
        if (abortController) abortController.abort();
        agentStore.clear();
        sessionReady = false;
        sessionError = null;
        streamingMsgId = null;
        turnStartedAt = null;

        const conn = schemaStore.activeConnection;
        if (!conn) return;

        sessionConnectionId = conn.id;
        sessionDatabase = schemaStore.selectedDatabase;

        const sessionId = crypto.randomUUID();
        const port = harnessStore.port ?? 0;
        const schema = schemaStore.activeSchema ?? "public";

        const systemPrompt = buildSystemPrompt(
            schemaStore.databases,
            schemaStore.selectedDatabase,
            conn.engine,
            port > 0 ? { port, sessionId, schema } : undefined,
        );

        const ac = new AbortController();
        abortController = ac;

        try {
            const sess = await startAgentSession({
                sessionId,
                systemPrompt,
                onEvent: handleEvent,
                abortController: ac,
            });
            agentStore.session = sess;
            sessionReady = true;
        } catch (e) {
            sessionError = String(e);
        }
    }

    function getToolContext(): ToolContext | null {
        const sess = agentStore.session;
        const conn = schemaStore.activeConnection;
        if (!sess || !conn) return null;
        return {
            port: sess.port,
            sessionId: sess.sessionId,
            connectionId: conn.id,
            database: schemaStore.selectedDatabase ?? "",
            schema: schemaStore.activeSchema ?? "public",
            openInEditor: (sql: string, _title: string) => {
                handleRunQuery(sql);
            },
        };
    }

    function handleEvent(event: AgentEventType) {
        switch (event.type) {
            case "text.delta": {
                if (!streamingMsgId) {
                    streamingMsgId = agentStore.startAssistantMessage();
                }
                agentStore.appendDelta(streamingMsgId, event.content);
                break;
            }
            case "thinking.delta": {
                if (!streamingMsgId) {
                    streamingMsgId = agentStore.startAssistantMessage();
                }
                agentStore.appendThinking(streamingMsgId, event.content);
                break;
            }
            case "tool.started": {
                agentStore.addToolCall(event.toolId, event.toolName, event.input);
                const ctx = getToolContext();
                if (ctx) {
                    dispatchTool(event.toolName, event.toolId, event.input, ctx).catch((e) => {
                        console.error("[AgentPanel] tool dispatch error:", e);
                    });
                }
                break;
            }
            case "tool.completed": {
                agentStore.completeToolCall(event.toolId, event.output);
                break;
            }
            case "tool.input_delta": {
                const session = windowState.activeSession;
                if (session) {
                    const toolCall = agentStore.toolCalls.find((t) => t.id === event.toolId);
                    const fileName = toolCall
                        ? ((toolCall.input as Record<string, unknown>)?.fileName as string | undefined)
                        : undefined;
                    if (fileName) {
                        let view = session.views.find((v) => v.title === fileName);
                        if (!view) {
                            session.openView("editor", fileName, { content: "" });
                            view = session.views.find((v) => v.title === fileName);
                        }
                        if (view) {
                            view.streamingContent = event.partialContent;
                        }
                    }
                }
                break;
            }
            case "turn.done": {
                stopTurnTimer();
                if (streamingMsgId) {
                    agentStore.finalizeMessage(streamingMsgId);
                    streamingMsgId = null;
                }
                agentStore.setStatus("idle");
                break;
            }
            case "error": {
                stopTurnTimer();
                if (streamingMsgId) {
                    agentStore.finalizeMessage(streamingMsgId);
                    streamingMsgId = null;
                }
                agentStore.setError(event.message);
                break;
            }
        }
    }

    async function send(text: string) {
        if (!agentStore.session || agentStore.status === "running") return;
        agentStore.addUserMessage(text);
        agentStore.setStatus("running");
        streamingMsgId = null;
        startTurnTimer();
        agentStore.session.send(text);
    }

    function stop() {
        stopTurnTimer();
        abortController?.abort();
        if (streamingMsgId) {
            agentStore.finalizeMessage(streamingMsgId);
            streamingMsgId = null;
        }
        agentStore.setStatus("idle");
    }

    function handleRunQuery(sql: string) {
        const activeSess = windowState.activeSession;
        if (!activeSess) return;
        activeSess.openView("editor", "AI Query", { content: sql });
        windowState.layout.showSqlEditor = false;
    }

    $effect(() => {
        const connId = schemaStore.activeConnection?.id;
        const db = schemaStore.selectedDatabase;
        if (connId && (connId !== sessionConnectionId || db !== sessionDatabase)) {
            bootSession();
        }
    });

    onDestroy(() => {
        abortController?.abort();
        if (turnTimerInterval !== null) clearInterval(turnTimerInterval);
    });
</script>

<div class="flex h-full flex-col bg-background">
    <!-- Header -->
    <div class="flex h-8 shrink-0 items-center justify-between border-b border-border px-3">
        <div class="flex items-center gap-1.5 text-[12px] font-medium text-foreground">
            <IconAi size={13} class="text-accent" />
            <span>Claude</span>
        </div>
        {#if agentStore.status === "running" && turnStartedAt !== null}
            <span class="font-mono text-[10px] text-accent">
                {formatTurnElapsed(turnElapsed)}
            </span>
        {/if}
    </div>

    <!-- Content -->
    {#if !schemaStore.activeConnection}
        <div class="flex flex-1 flex-col items-center justify-center gap-2 px-4 text-center text-muted-foreground">
            <IconAi size={24} class="opacity-30" />
            <span class="text-[12px]">Connect to a database to start chatting</span>
        </div>
    {:else if sessionError}
        <div class="flex flex-1 flex-col items-center justify-center gap-3 px-4 text-center">
            <IconAlertCircle size={20} class="text-destructive" />
            <span class="text-[12px] text-muted-foreground">{sessionError}</span>
            <button
                onclick={bootSession}
                class="rounded-md bg-accent/10 px-3 py-1.5 text-[12px] text-accent hover:bg-accent/20"
            >
                Retry
            </button>
        </div>
    {:else if !sessionReady}
        <div class="flex flex-1 items-center justify-center text-muted-foreground">
            <span class="text-[12px]">Starting session…</span>
        </div>
    {:else}
        <MessageList onRunQuery={handleRunQuery} />
        <AgentComposer
            onSend={(text, _doc) => send(text)}
            onStop={stop}
            running={agentStore.status === "running"}
            disabled={!sessionReady || !!sessionError}
        />
    {/if}
</div>
