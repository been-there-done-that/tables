<script lang="ts">
    import PopoverShell from "./PopoverShell.svelte";
    import { cn } from "$lib/utils";
    import { useMonacoEditor } from "$lib/monaco/useMonacoEditor";
    import type { EditorHandle } from "$lib/monaco/editor-types";
    import IconCheck from "@tabler/icons-svelte/icons/check";
    import IconX from "@tabler/icons-svelte/icons/x";
    import type * as Monaco from "monaco-editor";
    import { MONACO_THEME_NAME } from "$lib/monaco/monaco-theme";
    import { windowState } from "$lib/stores/window.svelte";
    import { settingsStore } from "$lib/stores/settings.svelte";

    interface Props {
        value: any;
        anchorEl?: HTMLElement | null;
        onCommit: (newValue: any) => void;
        onCancel: () => void;
    }

    let { value, anchorEl, onCommit, onCancel }: Props = $props();

    let editorContainer: HTMLElement | null = null;
    let editorInstance: Monaco.editor.IStandaloneCodeEditor | null = null;

    let isVisible = $state(false);
    let errorMessage = $state<string | null>(null);

    const isMac =
        typeof navigator !== "undefined" && navigator.userAgent.includes("Mac");

    // svelte-ignore state_referenced_locally
    const originalText = buildInitialText(value);
    // svelte-ignore state_referenced_locally
    const originalParsed = parseJsonSafe(originalText, value);

    const modelUri = `json://popover/${crypto.randomUUID()}`;

    function buildInitialText(val: any): string {
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

    useMonacoEditor(
        {
            contextId: `json-popover-${crypto.randomUUID()}`,
            windowId: windowState.label,
            kind: "json",
            modelUri: modelUri,
            container: () => editorContainer,
            options: {
                theme: MONACO_THEME_NAME,
                minimap: { enabled: false },
                stickyScroll: { enabled: false },
                automaticLayout: true,
                wordWrap: "off",
                scrollBeyondLastLine: false,
                lineNumbers: "on",
                tabSize: 2,
                fontSize: settingsStore.editorFontSize,
                fontFamily: settingsStore.editorFontFamily,
                scrollbar: {
                    horizontal: "auto",
                    vertical: "auto",
                },
                padding: { bottom: 28 },
            },
        },
        (handle) => {
            setupEditorInteractions(handle);
        },
    );

    // Reactive font settings
    $effect(() => {
        if (editorInstance) {
            editorInstance.updateOptions({
                fontFamily: settingsStore.editorFontFamily,
                fontSize: settingsStore.editorFontSize,
            });
        }
    });

    function setupEditorInteractions(handle: EditorHandle) {
        const editor = handle.editor;
        editorInstance = editor;

        const text = buildInitialText(value);
        if (editor.getValue() !== text) {
            editor.setValue(text);
        }

        requestAnimationFrame(() => {
            editor.layout();
            setTimeout(() => editor.layout(), 50);
        });

        import("monaco-editor").then((monaco) => {
            editor.addCommand(
                monaco.KeyMod.CtrlCmd | monaco.KeyCode.Enter,
                () => {
                    commitFromEditor();
                },
            );
        });

        editor.onDidChangeModelContent(() => {
            validateCurrent();
        });

        isVisible = true;
        editor.focus();
    }

    function validateCurrent() {
        if (!editorInstance) return { ok: true, parsed: null };
        const text = editorInstance.getValue();
        if (!text.trim()) {
            errorMessage = null;
            return { ok: true, parsed: null };
        }
        try {
            const parsed = JSON.parse(text);
            errorMessage = null;
            return { ok: true, parsed };
        } catch (err: any) {
            const message = err?.message ?? "Invalid JSON";
            errorMessage = message;
            return { ok: false, error: message };
        }
    }

    function commitFromEditor() {
        if (!editorInstance) return;
        const result = validateCurrent();
        if (!result.ok) return;
        const currentText = editorInstance.getValue();
        const parsed = result.parsed ?? null;
        const unchangedText = currentText.trim() === originalText.trim();
        const unchangedValue = isSameValue(parsed, originalParsed);
        if (unchangedText || unchangedValue) {
            onCancel();
            return;
        }
        onCommit(parsed);
    }

    function handleEditorContextMenu(event: MouseEvent) {
        editorInstance?.trigger("mouse", "editor.action.showContextMenu", null);
    }

    function handleKeydown(e: KeyboardEvent) {
        if ((e.metaKey || e.ctrlKey) && e.key === "Enter") {
            e.preventDefault();
            commitFromEditor();
        }
    }
</script>

<PopoverShell
    {anchorEl}
    {onCancel}
    minWidth={520}
    maxWidth={720}
    minHeight={200}
    maxHeight={640}
>
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <div
        class="flex-1 overflow-hidden min-h-[180px] relative"
        role="group"
        onkeydown={handleKeydown}
    >
        <div
            bind:this={editorContainer}
            class="absolute inset-0"
            role="presentation"
            oncontextmenu={handleEditorContextMenu}
        ></div>
        {#if errorMessage}
            <div class="absolute inset-x-0 bottom-0 pointer-events-none z-10">
                <div
                    class="mx-1 mb-1 rounded bg-destructive/80 text-destructive-foreground text-[10px] px-2 py-1 shadow-sm border border-destructive/60 backdrop-blur-sm"
                >
                    {errorMessage}
                </div>
            </div>
        {/if}

        <div
            class="absolute bottom-1 left-0 right-0 flex items-center justify-center gap-2 pointer-events-none z-10"
        >
            <button
                type="button"
                class="flex items-center gap-1.5 px-2 py-0.5 rounded border border-transparent hover:border-accent/10 hover:bg-muted text-foreground-muted transition-colors active:scale-95 group/btn pointer-events-auto"
                onclick={onCancel}
            >
                <span
                    class="text-[9px] font-medium px-1 rounded bg-black/5 dark:bg-white/5 border border-black/5 dark:border-white/5 text-foreground-muted/60"
                    >Esc</span
                >
                <IconX
                    class="size-3.5 opacity-60 group-hover/btn:opacity-100"
                />
            </button>

            <button
                type="button"
                class="flex items-center gap-1.5 px-2 py-0.5 rounded text-accent border border-transparent hover:border-accent/10 hover:bg-accent/10 transition-colors active:scale-95 group/btn pointer-events-auto"
                onclick={commitFromEditor}
            >
                <span
                    class="text-[9px] font-medium px-1 rounded bg-accent/10 border border-accent/20 text-accent/80"
                    >{isMac ? "⌘↵" : "Ctrl↵"}</span
                >
                <IconCheck
                    class="size-3.5 opacity-80 group-hover/btn:opacity-100"
                />
            </button>
        </div>
    </div>
</PopoverShell>
