import * as monaco from 'monaco-editor';

let monacoPromise: Promise<typeof monaco> | null = null;

export function preloadMonaco(): Promise<typeof monaco> {
    if (!monacoPromise) {
        monacoPromise = Promise.resolve(monaco).then(m => {
            configureWorkers(m);
            registerLanguages(m);
            registerThemes(m);
            return m;
        });
    }
    return monacoPromise;
}

import EditorWorker from "monaco-editor/esm/vs/editor/editor.worker?worker";
import JsonWorker from "monaco-editor/esm/vs/language/json/json.worker?worker";

function configureWorkers(m: typeof monaco) {
    if (typeof self !== "undefined" && !(self as any).MonacoEnvironment) {
        (self as any).MonacoEnvironment = {
            getWorker(_: string, label: string) {
                if (label === "json") return new JsonWorker();
                return new EditorWorker();
            },
        };
    }
}

function registerLanguages(m: typeof monaco) {
    // Register custom languages or extend existing ones here
    // e.g. m.languages.registerCompletionItemProvider('sql', ...)
}

function registerThemes(m: typeof monaco) {
    // Register custom themes
    m.editor.defineTheme('custom-dark', {
        base: 'vs-dark',
        inherit: true,
        rules: [],
        colors: {
            // 'editor.background': '#1e1e1e',
        }
    });
}
