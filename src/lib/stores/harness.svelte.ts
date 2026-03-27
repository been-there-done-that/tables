import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";

let _port = $state<number | null>(null);

// Register event listener first (handles case where harness starts after frontend)
listen<number>("harness://ready", (e) => {
    console.log("[harness] event received, port =", e.payload);
    _port = e.payload;
});

// Then immediately check via command (handles case where harness started before frontend loaded)
invoke<number | null>("get_harness_port").then((port) => {
    console.log("[harness] get_harness_port command returned:", port);
    if (port !== null && port !== undefined) {
        _port = port;
    }
}).catch((e) => {
    console.warn("[harness] get_harness_port failed:", e);
});

export const harnessStore = {
    get port() {
        return _port;
    },
};
