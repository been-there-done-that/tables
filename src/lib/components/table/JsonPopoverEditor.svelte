<script lang="ts">
    import { getContext, onMount } from "svelte";
    import { cn } from "$lib/utils";
    import { portal } from "$lib/actions/portal";
    import { focusTrap } from "$lib/actions/focus-trap";
    import { useMonacoEditor } from "$lib/monaco/useMonacoEditor";
    import type { EditorHandle } from "$lib/monaco/editor-types";
    import type * as Monaco from "monaco-editor";
    import { MONACO_THEME_NAME } from "$lib/monaco/monaco-theme";

    interface Props {
        value: any;
        anchorEl?: HTMLElement | null;
        onCommit: (newValue: any) => void;
        onCancel: () => void;
    }

    let { value, anchorEl, onCommit, onCancel }: Props = $props();

    let overlayEl: HTMLElement | null = null;
    let editorContainer: HTMLElement | null = null;
    let currentHandle: EditorHandle | null = null;
    let editorInstance: Monaco.editor.IStandaloneCodeEditor | null = null;

    // UI State
    let position = $state({ top: 0, left: 0, width: 520 });
    let isVisible = $state(false);
    let placement = $state<"left" | "right">("right");
    let arrowOffset = $state(0);
    let errorMessage = $state<string | null>(null);

    const GUTTER = 4;

    const DEFAULT_JSON = JSON.stringify(
        {
            id: "root-123",
            meta: { createdAt: "2025-01-15T12:34:56.789Z" },
        },
        null,
        2,
    );

    const originalText = buildInitialText(value);
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
            contextId: "json-popover",
            windowId: "main",
            kind: "json",
            modelUri: modelUri,
            container: () => editorContainer,
            options: {
                theme: MONACO_THEME_NAME,
                minimap: { enabled: false },
                stickyScroll: { enabled: false },
                automaticLayout: true,
                wordWrap: "off", // Enable horizontal scrolling
                scrollBeyondLastLine: false,
                lineNumbers: "on",
                tabSize: 2,
                fontSize: 12, // Reduced from 13
                scrollbar: {
                    horizontal: "auto",
                    vertical: "auto",
                },
                fontFamily:
                    "Fira Code, ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace",
            },
        },
        (handle) => {
            currentHandle = handle;
            setupEditorInteractions(handle);
        },
    );

    function setupEditorInteractions(handle: EditorHandle) {
        const editor = handle.editor;
        editorInstance = editor;

        const text = buildInitialText(value);
        if (editor.getValue() !== text) {
            editor.setValue(text);
        }

        // Ensure explicit layout after visibility
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

        // Show editor
        isVisible = true;
        editor.focus();
    }

    // ... validation and commit functions (unchanged) ...
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
        // Don't close if clicking monaco overlays
        if ((target as HTMLElement).closest?.(".monaco-aria-container")) return;
        onCancel();
    }
    // ... portal ...

    // RESTORED Positioning Logic
    function updatePosition() {
        if (!anchorEl || !anchorEl.isConnected) {
            onCancel();
            return;
        }
        const rect = anchorEl.getBoundingClientRect();
        const width = 520;
        const overlayHeight = overlayEl?.offsetHeight ?? 360;
        const margin = 8;

        let left = rect.right + margin;
        placement = "right";

        const fitsRight = left + width + margin <= window.innerWidth;
        if (!fitsRight) {
            left = rect.left - width - margin;
            placement = "left";
        }
        left = Math.max(
            margin,
            Math.min(left, window.innerWidth - width - margin),
        );

        let top = rect.top + rect.height / 2 - overlayHeight / 2;
        const minTop = margin;
        const maxTop = window.innerHeight - overlayHeight - margin;
        top = Math.max(minTop, Math.min(top, maxTop));

        // Calculate arrow vertical offset
        const anchorCenterY = rect.top + rect.height / 2;
        const minArrow = 12;
        const maxArrow = overlayHeight - 12;
        arrowOffset = Math.max(
            minArrow,
            Math.min(anchorCenterY - top, maxArrow),
        );

        position = { top, left, width };
    }

    onMount(() => {
        requestAnimationFrame(updatePosition);
        const handleUpdate = () => requestAnimationFrame(updatePosition);
        window.addEventListener("resize", handleUpdate);
        window.addEventListener("scroll", handleUpdate, true);
        const containerGetter: any = getContext("table-container");
        const containerEl = containerGetter?.();
        containerEl?.addEventListener("scroll", handleUpdate, {
            passive: true,
        });
        document.addEventListener("mousedown", handleClickOutside);

        return () => {
            window.removeEventListener("resize", handleUpdate);
            window.removeEventListener("scroll", handleUpdate, true);
            containerEl?.removeEventListener("scroll", handleUpdate);
            document.removeEventListener("mousedown", handleClickOutside);
        };
    });
</script>

<div
    use:portal
    use:focusTrap
    bind:this={overlayEl}
    data-placement={placement}
    role="dialog"
    aria-label="Edit JSON value"
    tabindex="-1"
    onkeydown={handleKeydown}
    class={cn(
        "popover-editor fixed rounded-lg shadow-[0_10px_40px_-10px_rgba(0,0,0,0.5)] flex flex-col p-1",
        "bg-surface border border-accent/20 ring-1 ring-accent/10",
        isVisible ? "anim-pop opacity-100" : "opacity-0 pointer-events-none",
    )}
    style={`top:${position.top}px;left:${position.left}px;min-width:${position.width}px;max-width:720px;min-height:200px;max-height:640px;transform-origin:center;z-index:1000;--arrow-top:${arrowOffset}px`}
    aria-hidden={!isVisible}
>
    <div class="flex-1 overflow-hidden min-h-[180px] relative">
        <div
            bind:this={editorContainer}
            class="absolute inset-0 h-full w-full"
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
        class="flex items-center justify-between border-t border-border px-2 py-1 gap-2 bg-surface"
    >
        <div class="text-xs text-foreground-muted truncate">
            Ctrl/Cmd+Enter to save · Esc to cancel
        </div>
        <div class="flex items-center gap-2">
            <button
                type="button"
                class="px-2 py-1 text-sm rounded bg-tertiary text-foreground hover:bg-muted transition"
                onclick={onCancel}
            >
                Cancel
            </button>
            <button
                type="button"
                class="px-2 py-1 text-sm rounded bg-accent text-accent-foreground hover:bg-accent-hover transition"
                onclick={commitFromEditor}
            >
                Save
            </button>
        </div>
    </div>
</div>
