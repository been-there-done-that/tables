// src/lib/stores/harnessLog.svelte.ts
import { listen } from "@tauri-apps/api/event";

export interface HarnessLogEntry {
    ts: number;
    level: "debug" | "info" | "warn" | "error";
    tag: string;
    message: string;
}

const MAX_ENTRIES = 200;

function createHarnessLogStore() {
    let entries = $state<HarnessLogEntry[]>([]);
    let unlisten: (() => void) | undefined;

    // Start listening immediately when this module is imported
    listen<HarnessLogEntry>("harness://log", (event) => {
        const e = event.payload;
        const msg = `[harness] [${e.tag}] ${e.message}`;
        if (e.level === "error")      console.error(msg);
        else if (e.level === "warn")  console.warn(msg);
        else if (e.level === "debug") console.debug(msg);
        else                          console.log(msg);
        entries = [...entries.slice(-(MAX_ENTRIES - 1)), e];
    }).then(fn => { unlisten = fn; });

    return {
        get entries() { return entries; },
        clear() { entries = []; },
        destroy() { unlisten?.(); },
    };
}

export const harnessLogStore = createHarnessLogStore();
