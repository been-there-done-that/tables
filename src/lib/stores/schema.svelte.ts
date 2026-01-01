import { invoke } from "@tauri-apps/api/core";
import { toast } from "svelte-sonner";
import type { MetaSchema } from "$lib/commands/types";

export class SchemaStore {
    activeConnectionId = $state<string | null>(null);
    status = $state<"idle" | "connecting" | "refreshing" | "error">("idle");
    schemas = $state<MetaSchema[]>([]);
    error = $state<string | null>(null);
    activeSchema = $state<String | null>(null)

    async connect(id: string) {
        this.status = "connecting";
        this.activeConnectionId = id;
        this.error = null;
        this.schemas = [];

        try {
            //Verify connection working
            await invoke("test_connection_by_id", { id });

            // Fetch cached schema
            this.schemas = await invoke("get_schema", { connectionId: id });

            this.status = "idle";

            if (this.schemas.length === 0) {
                toast.info("No schema found. Click 'Refresh Schema' to introspect.");
            }
        } catch (e) {
            this.error = String(e);
            this.status = "error";
            toast.error("Failed to connect: " + this.error);
        }
    }

    async refresh() {
        if (!this.activeConnectionId) return;

        this.status = "refreshing";
        this.error = null;

        try {
            await invoke("refresh_schema", { connectionId: this.activeConnectionId });
            this.schemas = await invoke("get_schema", { connectionId: this.activeConnectionId });
            this.status = "idle";
            toast.success("Schema refreshed");
        } catch (e) {
            this.error = String(e);
            this.status = "error";
            toast.error("Failed to refresh schema: " + this.error);
        }
    }
}

export const schemaStore = new SchemaStore();
