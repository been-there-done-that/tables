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
    import { plansStore } from "$lib/stores/plans.svelte";
    import MessageList from "./MessageList.svelte";
    import AgentComposer from "./AgentComposer.svelte";
    import ThreadPicker from "./ThreadPicker.svelte";
    import IconAi from "@tabler/icons-svelte/icons/ai";
    import IconAlertCircle from "@tabler/icons-svelte/icons/alert-circle";
    import IconCopy from "@tabler/icons-svelte/icons/copy";
    import IconCheck from "@tabler/icons-svelte/icons/check";

    // Plan mode: tools that execute SQL against live data require user approval before running.
    // Read-only tools and write_file auto-execute. Only run_query is gated.
    let planMode = $state(false);
    const APPROVAL_REQUIRED = new Set(["run_query"]);
    type PendingApproval = { toolName: string; input: unknown; ctx: ToolContext };
    const pendingApprovals = new Map<string, PendingApproval>();

    let abortController = $state<AbortController | null>(null);
    let sessionReady = $state(false);
    let sessionError = $state<string | null>(null);
    let sessionConnectionId = $state<string | null>(null);
    let sessionDatabase = $state<string | null>(null);
    let sessionModel = $state<string | null>(null);
    let sessionEffort = $state<string | null>(null);
    let streamingMsgId: string | null = null;
    let titleSet = $state(false); // whether we've auto-titled this thread yet

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
        const openTabs = windowState.activeSession?.views
            .filter((v) => v.type === "editor")
            .map((v) => ({ id: v.id, title: v.title })) ?? [];
        return buildSystemPrompt(
            schemaStore.databases,
            schemaStore.selectedDatabase,
            conn.engine,
            port > 0 ? { port, sessionId, schema } : undefined,
            openTabs,
            planMode,
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
        await plansStore.loadForThread(thread.id);

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

    async function runChildAgent(goal: string, model?: string): Promise<string> {
        const conn = schemaStore.activeConnection;
        if (!conn) throw new Error("No active connection");
        const parentThreadId = threadsStore.activeThreadId ?? undefined;

        // Create a child thread in SQLite
        const childThread = await threadsStore.createThread({
            connectionId: conn.id,
            databaseName: schemaStore.selectedDatabase,
            model: model ?? settingsStore.aiModel,
            effort: settingsStore.aiEffort,
            parentThreadId,
        });

        // Start a child harness session
        const childSessionId = crypto.randomUUID();
        const childAc = new AbortController();
        const childMessages: string[] = [];

        await new Promise<void>((resolve, reject) => {
            startAgentSession({
                sessionId: childSessionId,
                threadId: childThread.id,
                systemPrompt: buildPrompt(childSessionId),
                model: model ?? settingsStore.aiModel,
                effort: settingsStore.aiEffort,
                onEvent: (event) => {
                    if (event.type === "text.delta") {
                        childMessages.push(event.content);
                    } else if (event.type === "turn.done") {
                        resolve();
                    } else if (event.type === "error") {
                        reject(new Error(event.message));
                    } else if (event.type === "tool.started") {
                        // Auto-dispatch all tools for child (no approval gate)
                        const childCtx = getToolContext();
                        if (childCtx) {
                            dispatchTool(event.toolName, event.toolId, event.input, {
                                ...childCtx,
                                sessionId: childSessionId,
                            }).catch(console.error);
                        }
                    }
                },
                abortController: childAc,
            })
            .then((sess) => sess.send(goal))
            .catch(reject);
        });

        return childMessages.join("");
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
            openInEditor: (sql: string, _title: string, autoRun = false) => {
                handleRunQuery(sql, autoRun);
            },
            spawnSubagent: (goal: string, model?: string) => runChildAgent(goal, model),
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
                // Finalize any in-progress text bubble so it appears before the
                // tool card in the timeline.
                if (streamingMsgId) {
                    agentStore.finalizeMessage(streamingMsgId);
                    streamingMsgId = null;
                }
                const ctx = getToolContext();
                const needsApproval = planMode && APPROVAL_REQUIRED.has(event.toolName) && !!ctx;
                agentStore.addToolCall(event.toolId, event.toolName, event.input, needsApproval ? "awaiting" : "running");
                if (ctx) {
                    if (needsApproval) {
                        // Hold execution — user must approve before the POST goes to harness.
                        pendingApprovals.set(event.toolId, { toolName: event.toolName, input: event.input, ctx });
                    } else {
                        dispatchTool(event.toolName, event.toolId, event.input, ctx).catch((e) => {
                            console.error("[AgentPanel] tool dispatch error:", e);
                        });
                    }
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
                agentStore.addTurnSummary(turnElapsed, sessionModel ?? "");
                agentStore.setStatus("idle");

                // On turn.done: check if the last assistant message contains <plan> XML
                const lastMsg = agentStore.messages.findLast((m) => m.role === "assistant" && !m.isError);
                if (lastMsg) {
                    const planMatch = lastMsg.content.match(/<plan[^>]*>([\s\S]*?)<\/plan>/i);
                    if (planMatch && threadsStore.activeThreadId) {
                        // Parse steps
                        const stepRe = /<step\s+phase="([^"]+)"[^>]*>([\s\S]*?)<\/step>/gi;
                        const parsedSteps: Array<{ phase: string; description: string }> = [];
                        let sm: RegExpExecArray | null;
                        while ((sm = stepRe.exec(planMatch[1])) !== null) {
                            parsedSteps.push({ phase: sm[1].trim(), description: sm[2].trim() });
                        }
                        if (parsedSteps.length > 0) {
                            // Create plan and steps in store/DB
                            plansStore.createPlan(threadsStore.activeThreadId, "Plan")
                                .then((plan) => {
                                    agentStore.setMessagePlanId(lastMsg.id, plan.id);
                                    for (const s of parsedSteps) {
                                        plansStore.addStep(plan.id, s.phase as "gather" | "draft" | "execute", s.description)
                                            .catch(console.error);
                                    }
                                })
                                .catch(console.error);
                        }
                    }
                }
                break;
            }
            case "error": {
                stopTurnTimer();
                if (streamingMsgId) {
                    agentStore.finalizeMessage(streamingMsgId);
                    streamingMsgId = null;
                }
                agentStore.addErrorMessage(event.message);
                agentStore.setError(event.message);
                break;
            }
        }
    }

    async function send(displayText: string, fullText: string, docJson?: unknown) {
        if (!agentStore.session || agentStore.status === "running") return;
        agentStore.addUserMessage(displayText, docJson);
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
        // Drain pending approvals so the harness unblocks immediately
        for (const [toolId, pending] of pendingApprovals) {
            agentStore.failToolCall(toolId, "Session stopped");
            fetch(`http://127.0.0.1:${pending.ctx.port}/tool-result/${toolId}`, {
                method: "POST",
                headers: { "Content-Type": "application/json" },
                body: JSON.stringify({ error: "Session stopped by user" }),
            }).catch(console.error);
        }
        pendingApprovals.clear();

        stopTurnTimer();
        abortController?.abort();
        if (streamingMsgId) {
            agentStore.finalizeMessage(streamingMsgId);
            streamingMsgId = null;
        }
        agentStore.addTurnSummary(turnElapsed, sessionModel ?? "", true);
        agentStore.setStatus("idle");
    }

    function handleFocusFile(fileId: string, lineStart?: number, lineEnd?: number) {
        const activeSess = windowState.activeSession;
        if (!activeSess) return;
        // Try by UUID first, then fall back to title match
        const view = activeSess.views.find((v) => v.id === fileId)
            ?? activeSess.views.find((v) => v.title === fileId);
        if (!view) return;
        activeSess.activeViewId = view.id;
        if (lineStart != null) {
            view.data ??= {};
            const prevSeq: number = (view.data as Record<string, unknown>).revealAt
                ? ((view.data as Record<string, unknown>).revealAt as Record<string, unknown>).seq as number ?? 0
                : 0;
            (view.data as Record<string, unknown>).revealAt = { start: lineStart, end: lineEnd ?? lineStart, seq: prevSeq + 1 };
        }
    }

    function approveToolCall(toolId: string) {
        const pending = pendingApprovals.get(toolId);
        if (!pending) return;
        pendingApprovals.delete(toolId);
        agentStore.setToolCallRunning(toolId);
        dispatchTool(pending.toolName, toolId, pending.input, pending.ctx).catch((e) => {
            console.error("[AgentPanel] approved tool dispatch error:", e);
        });
    }

    function rejectToolCall(toolId: string) {
        const pending = pendingApprovals.get(toolId);
        if (!pending) return;
        pendingApprovals.delete(toolId);
        agentStore.failToolCall(toolId, "Rejected by user");
        // POST rejection to harness so the agent's curl call unblocks with an error
        fetch(`http://127.0.0.1:${pending.ctx.port}/tool-result/${toolId}`, {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({ error: "User rejected SQL execution in plan mode" }),
        }).catch((e) => console.error("[AgentPanel] reject POST failed:", e));
    }

    function handleRunQuery(sql: string, autoRun = false) {
        const activeSess = windowState.activeSession;
        if (!activeSess) return;
        activeSess.openView("editor", "AI Query", { content: sql, pendingRun: autoRun });
        windowState.layout.showSqlEditor = false;
    }

    $effect(() => {
        const connId = schemaStore.activeConnection?.id;
        const db = schemaStore.selectedDatabase;
        const model = settingsStore.aiModel;
        const effort = settingsStore.aiEffort;
        if (connId && (connId !== sessionConnectionId || db !== sessionDatabase || model !== sessionModel || effort !== sessionEffort)) {
            initForConnection().catch((e) => { sessionError = String(e); });
        }
    });

    // Restart the harness session when planMode is toggled mid-conversation so
    // the system prompt reflects the new mode. Guard: only restart if the thread
    // already has messages (avoid restarting a fresh empty thread).
    let prevPlanMode = planMode;
    $effect(() => {
        const current = planMode;
        if (current === prevPlanMode) return;
        prevPlanMode = current;
        if (agentStore.messages.length > 0 && threadsStore.activeThread) {
            startThread(threadsStore.activeThread).catch(console.error);
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
            {#if planMode}
                {@const pendingCount = pendingApprovals.size}
                <span class="flex items-center gap-1 rounded-full bg-amber-400/15 px-2 py-0.5 text-[9.5px] font-medium text-amber-400 select-none">
                    Plan{pendingCount > 0 ? ` · ${pendingCount}` : ""}
                </span>
            {/if}
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
        <MessageList
            onRunQuery={handleRunQuery}
            onFocusFile={handleFocusFile}
            onApprove={approveToolCall}
            onReject={rejectToolCall}
        />
        <AgentComposer
            onSend={(displayText, fullText, doc) => send(displayText, fullText, doc)}
            onStop={stop}
            running={agentStore.status === "running"}
            disabled={!sessionReady || !!sessionError}
            {planMode}
            onPlanModeToggle={() => { planMode = !planMode; }}
        />
    {/if}
</div>
