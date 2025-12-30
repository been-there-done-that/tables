<script lang="ts">
    import { getContext, onMount } from "svelte";
    // @ts-ignore - types resolved at runtime
    import loader from "@monaco-editor/loader";
    // @ts-ignore - types resolved at runtime
    import type * as Monaco from "monaco-editor";
    // @ts-ignore - worker imports resolved by Vite
    import EditorWorker from "monaco-editor/esm/vs/editor/editor.worker?worker";
    // @ts-ignore - worker imports resolved by Vite
    import JsonWorker from "monaco-editor/esm/vs/language/json/json.worker?worker";
    import { cn } from "$lib/utils";

    interface Props {
        value: any;
        anchorEl?: HTMLElement | null;
        onCommit: (newValue: any) => void;
        onCancel: () => void;
    }

    let { value, anchorEl, onCommit, onCancel }: Props = $props();

    let overlayEl: HTMLElement | null = null;
    let editorContainer: HTMLElement | null = null;
    let editor: Monaco.editor.IStandaloneCodeEditor | null = null;
    let model: Monaco.editor.ITextModel | null = null;
    let monacoApi: typeof Monaco | null = null;
    const originalText = $derived(buildInitialText(value));
    const originalParsed = $derived(parseJsonSafe(originalText, value));
    let position = $state({ top: 0, left: 0, width: 520 });
    let isVisible = $state(false);
    let errorMessage = $state<string | null>(null);

    const GUTTER = 4;

    function buildInitialText(val: any): string {
        return DEFAULT_JSON;
        if (typeof val === "string") {
            try {
                return JSON.stringify(JSON.parse(val), null, 2);
            } catch {
                return val;
            }
        }
        try {
            return JSON.stringify(val, null, 2);
        } catch {
            return String(val);
        }
    }

    const DEFAULT_JSON = JSON.stringify(
        {
            id: "root-123",
            meta: {
                createdAt: "2025-01-15T12:34:56.789Z",
                updatedAt: "2025-02-01T08:15:30.000Z",
                tags: ["alpha", "beta", "gamma"],
                flags: { active: true, archived: false, pendingReview: null },
                stats: {
                    views: 120394,
                    rating: 4.87,
                    bounceRate: 0.142,
                    ratios: [1, 2.5, 3.14159, -7],
                },
            },
            owner: {
                userId: "u-789",
                profile: {
                    name: "Ada Lovelace",
                    roles: ["admin", "editor"],
                    preferences: {
                        theme: "dark",
                        notifications: { email: true, sms: false, push: true },
                    },
                },
            },
            items: [
                {
                    id: "item-1",
                    type: "document",
                    title: "Spec A",
                    content: {
                        sections: [
                            {
                                title: "Intro",
                                body: "Lorem ipsum dolor sit amet.",
                            },
                            {
                                title: "Body",
                                body: "Consectetur adipiscing elit.",
                            },
                            {
                                title: "Conclusion",
                                body: "Donec vitae sapien ut libero.",
                            },
                        ],
                        footnotes: [
                            { ref: 1, text: "First note" },
                            { ref: 2, text: "Second note" },
                        ],
                    },
                    metrics: { score: 92, quality: "high" },
                    links: [],
                },
                {
                    id: "item-2",
                    type: "dataset",
                    title: "Data B",
                    columns: ["id", "value", "ts", "ok", "extra"],
                    rows: [
                        ["r1", 123, "2025-02-02T10:00:00Z", true, null],
                        [
                            "r2",
                            -45.6,
                            "2025-02-02T11:00:00Z",
                            false,
                            { a: 1, b: [1, 2, 3] },
                        ],
                        [
                            "r3",
                            0,
                            "2025-02-02T12:00:00Z",
                            true,
                            ["x", "y", "z"],
                        ],
                    ],
                },
                {
                    id: "item-3",
                    type: "tree",
                    title: "Nested",
                    children: [
                        {
                            id: "child-1",
                            value: 10,
                            children: [
                                { id: "child-1-1", value: null },
                                { id: "child-1-2", value: 20, children: [] },
                            ],
                        },
                        {
                            id: "child-2",
                            value: 30,
                            children: [
                                {
                                    id: "child-2-1",
                                    value: 40,
                                    meta: { note: "deep node", flag: true },
                                },
                            ],
                        },
                    ],
                },
            ],
            attachments: [
                { name: "image.png", size: 34567, mime: "image/png" },
                { name: "report.pdf", size: 2345678, mime: "application/pdf" },
            ],
            audit: [
                { ts: "2025-02-01T09:00:00Z", action: "created", by: "u-789" },
                {
                    ts: "2025-02-03T10:30:00Z",
                    action: "edited",
                    by: "u-999",
                    diff: { title: ["old", "new"] },
                },
            ],
            mixedArray: [
                "string",
                123,
                true,
                null,
                { obj: 1 },
                [1, 2, 3],
                { nested: { deep: ["a", 1, null, false] } },
            ],
            emptyObject: {},
            emptyArray: [],
        },
        null,
        2,
    );

    function parseJsonSafe(text: string, fallback: any) {
        const trimmed = text.trim();
        if (!trimmed) return null;
        try {
            return JSON.parse(trimmed);
        } catch {
            return fallback;
        }
    }

    function isSameValue(a: any, b: any) {
        try {
            return JSON.stringify(a) === JSON.stringify(b);
        } catch {
            return false;
        }
    }

    function portal(node: HTMLElement) {
        if (typeof document === "undefined") return {};
        const target = document.body;
        target.appendChild(node);
        return {
            destroy() {
                if (node.parentNode === target) target.removeChild(node);
            },
        };
    }

    function updatePosition() {
        if (!anchorEl || !anchorEl.isConnected) {
            onCancel();
            return;
        }
        const rect = anchorEl.getBoundingClientRect();
        const width = Math.max(rect.width + 120, 420);
        const overlayHeight = overlayEl?.offsetHeight ?? 360;
        const margin = GUTTER;

        let left = rect.right + margin;
        const fitsRight = left + width + margin <= window.innerWidth;
        if (!fitsRight) {
            left = rect.left - width - margin;
        }
        left = Math.max(
            margin,
            Math.min(left, window.innerWidth - width - margin),
        );

        let top = rect.top + rect.height / 2 - overlayHeight / 2;
        const minTop = margin;
        const maxTop = window.innerHeight - overlayHeight - margin;
        top = Math.max(minTop, Math.min(top, maxTop));

        position = { top, left, width };
    }

    async function setupEditor() {
        if (!editorContainer) return;
        // Ensure workers are registered for validation/intellisense
        if (typeof self !== "undefined" && !(self as any).MonacoEnvironment) {
            (self as any).MonacoEnvironment = {
                getWorker(_: string, label: string) {
                    if (label === "json") return new JsonWorker();
                    return new EditorWorker();
                },
            };
        }

        const monaco = await loader.init();
        monacoApi = monaco;

        const root = document.documentElement;
        const isDark = root.classList.contains("dark");
        const bg =
            getComputedStyle(root)
                .getPropertyValue("--theme-bg-secondary")
                ?.trim() || (isDark ? "#0f172a" : "#ffffff");
        const fg =
            getComputedStyle(root)
                .getPropertyValue("--theme-fg-primary")
                ?.trim() || (isDark ? "#e2e8f0" : "#0f172a");
        const selection = isDark ? "#1f2a44" : "#dbeafe";
        const highlight = isDark ? "#1f2937" : "#e5e7eb";

        monaco.editor.defineTheme("table-popover-theme", {
            base: isDark ? "vs-dark" : "vs",
            inherit: true,
            rules: [],
            colors: {
                "editor.background": bg,
                "editor.foreground": fg,
                "editor.selectionBackground": selection,
                "editor.lineHighlightBackground": highlight,
                "editorCursor.foreground": fg,
                "editorLineNumber.foreground": isDark ? "#94a3b8" : "#94a3b8",
            },
        });

        const text = buildInitialText(value);
        model = monaco.editor.createModel(text, "json");
        editor = monaco.editor.create(editorContainer, {
            model,
            theme: "table-popover-theme",
            language: "json",
            minimap: { enabled: false },
            stickyScroll: { enabled: false },
            automaticLayout: true,
            wordWrap: "on",
            scrollBeyondLastLine: false,
            lineNumbers: "on",
            tabSize: 2,
            fontSize: 13,
            fontFamily:
                "Fira Code, ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace",
        });

        // Bind save to Cmd/Ctrl+Enter inside Monaco
        editor?.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.Enter, () => {
            commitFromEditor();
        });

        editor?.onDidChangeModelContent(() => {
            validateCurrent();
        });
    }

    function clearMarkers() {
        if (monacoApi && model) {
            monacoApi.editor.setModelMarkers(model, "json-validate", []);
        }
    }

    function setErrorMarker(message: string) {
        if (monacoApi && model) {
            monacoApi.editor.setModelMarkers(model, "json-validate", [
                {
                    message,
                    severity: monacoApi.MarkerSeverity.Error,
                    startLineNumber: 1,
                    startColumn: 1,
                    endLineNumber: Math.max(1, model.getLineCount()),
                    endColumn: 1,
                },
            ]);
        }
    }

    function validateCurrent() {
        if (!editor) return { ok: true, parsed: null };
        const text = editor.getValue();
        if (!text.trim()) {
            clearMarkers();
            errorMessage = null;
            return { ok: true, parsed: null };
        }
        try {
            const parsed = JSON.parse(text);
            clearMarkers();
            errorMessage = null;
            return { ok: true, parsed };
        } catch (err: any) {
            const message = err?.message ?? "Invalid JSON";
            errorMessage = message;
            setErrorMarker(message);
            return { ok: false, error: message };
        }
    }

    function commitFromEditor() {
        if (!editor) return;
        const result = validateCurrent();
        if (!result.ok) return;
        const currentText = editor.getValue();
        const parsed = result.parsed ?? null;

        const unchangedText = currentText.trim() === originalText.trim();
        const unchangedValue = isSameValue(parsed, originalParsed);
        if (unchangedText || unchangedValue) {
            clearMarkers();
            errorMessage = null;
            onCancel();
            return;
        }

        onCommit(parsed);
    }

    function handleEditorContextMenu(event: MouseEvent) {
        if (!editor) return;
        event.preventDefault();
        editor.trigger("mouse", "editor.action.showContextMenu", null);
    }

    function handleKeydown(e: KeyboardEvent) {
        e.stopPropagation();
        if (e.key === "Escape") {
            e.preventDefault();
            onCancel();
        } else if ((e.metaKey || e.ctrlKey) && e.key === "Enter") {
            e.preventDefault();
            commitFromEditor();
        }
    }

    function handleClickOutside(event: MouseEvent) {
        const target = event.target as Node;
        if (overlayEl?.contains(target)) return;
        if (anchorEl?.contains(target)) return;
        onCancel();
    }

    onMount(() => {
        requestAnimationFrame(updatePosition);
        const handleUpdate = () => requestAnimationFrame(updatePosition);
        window.addEventListener("resize", handleUpdate);
        window.addEventListener("scroll", handleUpdate, true);
        const containerGetter:
            | (() => HTMLElement | null | undefined)
            | undefined = getContext("table-container");
        const containerEl = containerGetter?.();
        containerEl?.addEventListener("scroll", handleUpdate, {
            passive: true,
        });
        document.addEventListener("mousedown", handleClickOutside);

        setupEditor()
            .catch((err) => {
                console.error("[JsonPopoverEditor] monaco load failed", err);
                errorMessage = "Failed to load editor";
            })
            .finally(() => {
                queueMicrotask(() => {
                    overlayEl?.focus();
                    editor?.focus();
                    isVisible = true;
                });
            });

        return () => {
            window.removeEventListener("resize", handleUpdate);
            window.removeEventListener("scroll", handleUpdate, true);
            containerEl?.removeEventListener("scroll", handleUpdate);
            document.removeEventListener("mousedown", handleClickOutside);
            editor?.dispose();
            model?.dispose();
        };
    });
</script>

<div
    use:portal
    bind:this={overlayEl}
    role="dialog"
    aria-label="Edit JSON value"
    tabindex="-1"
    onkeydown={handleKeydown}
    class={cn(
        "fixed z-[1000] bg-[var(--theme-bg-secondary)] border border-[var(--theme-border-focus)] rounded-md flex flex-col p-1",
        isVisible ? "anim-pop" : "opacity-0 pointer-events-none",
    )}
    style={`top:${position.top}px;left:${position.left}px;min-width:${position.width}px;max-width:720px;min-height:420px;max-height:640px;transform-origin:center`}
    aria-hidden={!isVisible}
>
    <div class="flex-1 overflow-hidden min-h-[380px] relative">
        <div
            bind:this={editorContainer}
            class="h-full w-full min-h-[380px]"
            role="presentation"
            oncontextmenu={handleEditorContextMenu}
        ></div>

        {#if errorMessage}
            <div class="absolute inset-x-0 bottom-0 pointer-events-none">
                <div
                    class="mx-1 mb-1 rounded bg-destructive/80 text-destructive-foreground text-xs px-2 py-1 shadow-sm border border-destructive/60"
                >
                    {errorMessage}
                </div>
            </div>
        {/if}
    </div>

    <div
        class="flex items-center justify-between border-t border-[var(--theme-border-default)] px-2 py-1 gap-2 bg-[var(--theme-bg-secondary)]"
    >
        <div class="text-xs text-[var(--theme-fg-secondary)] truncate">
            Ctrl/Cmd+Enter to save · Esc to cancel
        </div>
        <div class="flex items-center gap-2">
            <button
                type="button"
                class="px-2 py-1 text-sm rounded bg-[var(--theme-bg-tertiary)] text-[var(--theme-fg-primary)] hover:bg-[var(--theme-bg-hover)] transition"
                onclick={onCancel}
            >
                Cancel
            </button>
            <button
                type="button"
                class="px-2 py-1 text-sm rounded bg-[var(--theme-accent-primary)] text-white hover:bg-[var(--theme-accent-hover)] transition"
                onclick={commitFromEditor}
            >
                Save
            </button>
        </div>
    </div>
</div>
