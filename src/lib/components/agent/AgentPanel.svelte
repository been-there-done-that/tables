<script lang="ts">
    import { onDestroy } from "svelte";
    import { schemaStore } from "$lib/stores/schema.svelte";
    import { settingsStore } from "$lib/stores/settings.svelte";
    import { agentStore } from "$lib/stores/agent.svelte";
    import { threadsStore, type AgentThread } from "$lib/stores/threads.svelte";
    import { windowState } from "$lib/stores/window.svelte";
    import { startAgentSession, type AgentEventType } from "$lib/agent/claude";
    import { buildSystemPrompt } from "$lib/agent/tools";
    import { dispatchTool, type ToolContext } from "$lib/agent/tool-executor";
    import { harnessStore } from "$lib/stores/harness.svelte";
    import MessageList from "./MessageList.svelte";
    import AgentComposer from "./AgentComposer.svelte";
    import ThreadPicker from "./ThreadPicker.svelte";
    import IconAi from "@tabler/icons-svelte/icons/ai";
    import IconAlertCircle from "@tabler/icons-svelte/icons/alert-circle";
    import IconCopy from "@tabler/icons-svelte/icons/copy";
    import IconCheck from "@tabler/icons-svelte/icons/check";

    let abortController = $state<AbortController | null>(null);
    let sessionReady = $state(false);
    let sessionError = $state<string | null>(null);
    let sessionConnectionId = $state<string | null>(null);
    let sessionDatabase = $state<string | null>(null);
    let sessionModel = $state<string | null>(null);
    let sessionEffort = $state<string | null>(null);
    let streamingMsgId: string | null = null;
    let titleSet = false; // whether we've auto-titled this thread yet

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

    function buildPrompt(sessionId: string) {
        const conn = schemaStore.activeConnection!;
        const port = harnessStore.port ?? 0;
        const schema = schemaStore.activeSchema ?? "public";
        return buildSystemPrompt(
            schemaStore.databases,
            schemaStore.selectedDatabase,
            conn.engine,
            port > 0 ? { port, sessionId, schema } : undefined,
        );
    }

    async function startThread(thread: AgentThread) {
        if (abortController) abortController.abort();
        sessionReady = false;
        sessionError = null;
        streamingMsgId = null;
        titleSet = false;
        turnStartedAt = null;

        const conn = schemaStore.activeConnection;
        if (!conn) return;

        threadsStore.setActive(thread.id);
        await agentStore.loadThread(thread.id);

        sessionConnectionId = conn.id;
        sessionDatabase = schemaStore.selectedDatabase;
        sessionModel = settingsStore.aiModel;
        sessionEffort = settingsStore.aiEffort;

        const sessionId = crypto.randomUUID();
        const ac = new AbortController();
        abortController = ac;

        try {
            const sess = await startAgentSession({
                sessionId,
                threadId: thread.id,
                systemPrompt: buildPrompt(sessionId),
                model: settingsStore.aiModel,
                effort: settingsStore.aiEffort,
                resumeSdkSessionId: thread.sdkSessionId ?? undefined,
                onEvent: handleEvent,
                abortController: ac,
            });
            agentStore.session = sess;
            sessionReady = true;
        } catch (e) {
            sessionError = String(e);
        }
    }

    async function createAndStartThread() {
        const conn = schemaStore.activeConnection;
        if (!conn) return;
        const thread = await threadsStore.createThread({
            connectionId: conn.id,
            databaseName: schemaStore.selectedDatabase,
            model: settingsStore.aiModel,
            effort: settingsStore.aiEffort,
        });
        await startThread(thread);
    }

    async function initForConnection() {
        const conn = schemaStore.activeConnection;
        if (!conn) return;

        await threadsStore.load(conn.id, schemaStore.selectedDatabase);

        if (threadsStore.threads.length === 0) {
            await createAndStartThread();
        } else {
            // Resume the most recent thread
            await startThread(threadsStore.threads[0]);
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
            case "session.init": {
                const threadId = threadsStore.activeThreadId;
                if (threadId) {
                    threadsStore.setSdkSessionId(threadId, event.sdkSessionId);
                }
                break;
            }
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

    async function send(displayText: string, fullText: string) {
        if (!agentStore.session || agentStore.status === "running") return;
        agentStore.addUserMessage(displayText);
        agentStore.setStatus("running");
        streamingMsgId = null;
        startTurnTimer();

        // Auto-title the thread from the first user message
        if (!titleSet) {
            titleSet = true;
            const title = displayText.slice(0, 60).replace(/\n/g, " ");
            const threadId = threadsStore.activeThreadId;
            if (threadId) {
                threadsStore.setTitle(threadId, title);
            }
        }

        agentStore.session.send(fullText);
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
        const model = settingsStore.aiModel;
        const effort = settingsStore.aiEffort;
        if (connId && (connId !== sessionConnectionId || db !== sessionDatabase || model !== sessionModel || effort !== sessionEffort)) {
            initForConnection();
        }
    });

    onDestroy(() => {
        abortController?.abort();
        if (turnTimerInterval !== null) clearInterval(turnTimerInterval);
    });

    let copyDone = $state(false);

    function copyChat() {
        const timeline = [
            ...agentStore.messages.map((m) => ({ kind: "message" as const, item: m, ts: m.timestamp })),
            ...agentStore.toolCalls.map((t) => ({ kind: "tool" as const, item: t, ts: t.timestamp })),
        ].sort((a, b) => a.ts - b.ts);

        const lines: string[] = [
            `# Tables Agent Chat`,
            `Copied: ${new Date().toISOString()}`,
            `Connection: ${schemaStore.activeConnection?.name ?? "unknown"} / ${schemaStore.selectedDatabase ?? ""}`,
            ``,
        ];

        for (const entry of timeline) {
            if (entry.kind === "message") {
                const m = entry.item;
                lines.push(`## ${m.role === "user" ? "User" : "Assistant"}`);
                if (m.thinking) lines.push(`<thinking>\n${m.thinking}\n</thinking>`);
                lines.push(m.content);
                lines.push(``);
            } else {
                const t = entry.item;
                const status = t.status === "running" ? "⏳" : t.status === "done" ? "✓" : "✗";
                lines.push(`### Tool: ${t.toolName} ${status}`);
                if (t.input) lines.push(`\`\`\`json\n${JSON.stringify(t.input, null, 2)}\n\`\`\``);
                if (t.output) lines.push(`**Output:**\n${t.output.slice(0, 2000)}${t.output.length > 2000 ? "\n…(truncated)" : ""}`);
                lines.push(``);
            }
        }

        navigator.clipboard.writeText(lines.join("\n")).then(() => {
            copyDone = true;
            setTimeout(() => { copyDone = false; }, 2000);
        });
    }
</script>

<div class="flex h-full flex-col bg-background">
    <!-- Header -->
    <div class="flex h-8 shrink-0 items-center justify-between border-b border-border px-2">
        <div class="flex items-center gap-1 min-w-0">
            <IconAi size={13} class="shrink-0 text-accent" />
            <ThreadPicker
                onNewThread={createAndStartThread}
                onSelectThread={(t) => startThread(t)}
            />
        </div>
        <div class="flex items-center gap-2 shrink-0">
            {#if agentStore.status === "running" && turnStartedAt !== null}
                <span class="font-mono text-[10px] text-accent">
                    {formatTurnElapsed(turnElapsed)}
                </span>
            {/if}
            {#if agentStore.messages.length > 0}
                <button
                    onclick={copyChat}
                    title="Copy chat for debugging"
                    class="flex items-center justify-center rounded p-0.5 text-muted-foreground/50 hover:text-muted-foreground transition-colors"
                >
                    {#if copyDone}
                        <IconCheck size={12} class="text-green-500" />
                    {:else}
                        <IconCopy size={12} />
                    {/if}
                </button>
            {/if}
        </div>
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
                onclick={() => initForConnection()}
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
            onSend={(displayText, fullText, _doc) => send(displayText, fullText)}
            onStop={stop}
            running={agentStore.status === "running"}
            disabled={!sessionReady || !!sessionError}
        />
    {/if}
</div>
