<script lang="ts">
    import { useMonacoEditor } from "$lib/monaco/useMonacoEditor";
    import type { EditorHandle } from "$lib/monaco/editor-types";
    import { MONACO_THEME_NAME } from "$lib/monaco/monaco-theme";
    import { settingsStore } from "$lib/stores/settings.svelte";
    import { onMount } from "svelte";

    let editorContainer: HTMLElement;
    let editorHandle = $state<EditorHandle | null>(null);
    let { fontFamily } = $props<{ fontFamily: string }>();

    // Static preview content
    const PREVIEW_CONTENT = `-- SQL Font Preview
SELECT 
    u.id, 
    u.email, 
    count(o.id) as total_orders,
    sum(o.amount) as lifetime_value
FROM users u
LEFT JOIN orders o ON u.id = o.user_id
WHERE u.status = 'active'
GROUP BY 1, 2
ORDER BY 4 DESC
LIMIT 10;`;

    // React to font changes
    $effect(() => {
        if (editorHandle?.editor) {
            const family = fontFamily.includes(" ")
                ? `"${fontFamily}"`
                : fontFamily;

            editorHandle.editor.updateOptions({
                fontFamily: family,
                fontSize: settingsStore.editorFontSize,
            });
        }
    });

    useMonacoEditor(
        {
            contextId: "font-preview-editor",
            windowId: "settings", // Use a distinct window ID for pooling
            kind: "sql",
            modelUri: "file:///settings/font-preview.sql",
            container: () => editorContainer,
            options: {
                theme: MONACO_THEME_NAME,
                minimap: { enabled: false },
                lineNumbers: "on",
                scrollBeyondLastLine: false,
                automaticLayout: true,
                // svelte-ignore state_referenced_locally
                fontFamily: fontFamily,
                fontSize: settingsStore.editorFontSize,
                readOnly: false,
                contextmenu: false,
            },
        },
        (handle) => {
            editorHandle = handle;
            if (handle.editor.getValue() !== PREVIEW_CONTENT) {
                handle.editor.setValue(PREVIEW_CONTENT);
            }
        },
    );
</script>

<div
    class="w-full h-full relative border border-border rounded-md overflow-hidden"
>
    <div bind:this={editorContainer} class="absolute inset-0"></div>
</div>
