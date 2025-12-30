import * as monaco from 'monaco-editor';
import { defineMonacoTheme } from './monaco-theme';
import 'monaco-editor/min/vs/editor/editor.main.css';
import EditorWorker from "monaco-editor/esm/vs/editor/editor.worker?worker";
import JsonWorker from "monaco-editor/esm/vs/language/json/json.worker?worker";

let monacoPromise: Promise<typeof monaco> | null = null;

/**
 * Preloads the Monaco infrastructure. 
 * Since we are bundling Monaco with Vite, we just initialize the workers 
 * and return the singleton instance.
 */
export function preloadMonaco(): Promise<typeof monaco> {
    if (!monacoPromise) {
        monacoPromise = Promise.resolve(monaco).then((m) => {
            configureWorkers(m);
            registerLanguages(m);
            registerThemes(m);
            return m;
        });
    }
    return monacoPromise;
}


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
    // Register the dynamic application theme
    defineMonacoTheme(m);

    // Register custom legacy themes if any
    m.editor.defineTheme('custom-dark', {
        base: 'vs-dark',
        inherit: true,
        rules: [],
        colors: {
            // 'editor.background': '#1e1e1e',
        }
    });
}
