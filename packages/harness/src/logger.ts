/** Shared structured logger — writes JSON lines to stderr for Rust to capture and forward as Tauri events. */
export function hLog(level: "debug" | "info" | "warn" | "error", tag: string, message: string): void {
    process.stderr.write(JSON.stringify({ ts: Date.now(), level, tag, message }) + "\n");
}
