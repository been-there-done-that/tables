import { invoke } from "@tauri-apps/api/core";
import { toast } from "svelte-sonner";
import type { Connection, MetaSchema } from "$lib/commands/types";

export class SchemaStore {
    activeConnection = $state<Connection | null>(null);
    status = $state<"idle" | "connecting" | "refreshing" | "error">("idle");
    schemas = $state<MetaSchema[]>([]);
    error = $state<string | null>(null);

    async connect(conn: Connection) {
        const previousId = this.activeConnection?.id;

        this.status = "connecting";
        this.error = null;
        this.activeConnection = conn;
        this.schemas = []; // Clear previous schemas immediately

        try {
            // 1. Validate connection
            await invoke("test_connection_by_id", { id: conn.id }); // Using the ID version

            // 2. Mark as active in backend
            await invoke("mark_connection_active", { id: conn.id });

            // 3. Mark previous as inactive if it's different
            if (previousId && previousId !== conn.id) {
                // We don't await this to avoid blocking UI, or maybe we should? 
                // It's safer to just fire and forget or await if quick.
                invoke("mark_connection_inactive", { id: previousId }).catch(console.error);
            }

            // 4. Fetch Schema (Cached)
            const result = await invoke<MetaSchema[]>("get_schema", { connectionId: conn.id });

            this.schemas = result;
            this.status = "idle";

            if (result.length === 0) {
                toast.success("Connected", { description: "No schema found. Try refreshing." });
            } else {
                toast.success("Connected", { description: `Loaded ${result.length} schemas.` });
            }

        } catch (e) {
            this.status = "error";
            this.error = String(e);
            toast.error("Connection Failed", { description: String(e) });

            // Revert active status if failed?
            // Maybe not necessary if UI shows error, but let's be clean.
            invoke("mark_connection_inactive", { id: conn.id }).catch(console.error);
        }
    }

    async refresh() {
        if (!this.activeConnection) return;

        this.status = "refreshing";
        this.error = null;

        try {
            await invoke("refresh_schema", { connectionId: this.activeConnection.id });
            const result = await invoke<MetaSchema[]>("get_schema", { connectionId: this.activeConnection.id });
            this.schemas = result;
            this.status = "idle";
            toast.success("Schema Refreshed");
        } catch (e) {
            this.status = "idle"; // Go back to idle/error
            toast.error("Refresh Failed", { description: String(e) });
        }
    }
}

export const schemaStore = new SchemaStore();
