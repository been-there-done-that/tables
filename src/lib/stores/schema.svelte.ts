import { invoke } from "@tauri-apps/api/core";
import { toast } from "svelte-sonner";
import type { Connection, MetaDatabase } from "$lib/commands/types";

export class SchemaStore {
    activeConnection = $state<Connection | null>(null);
    status = $state<"idle" | "connecting" | "refreshing" | "error">("idle");
    databases = $state<MetaDatabase[]>([]);
    error = $state<string | null>(null);
    lastRefreshed = $state<Date | null>(null);
    windowLabel = $state<string | null>(null);

    async initialize(label: string) {
        this.windowLabel = label;
        console.log(`[SchemaStore] Initializing for window: ${label} `);

        try {
            // Check if there's a persisted session for this window
            const persistedId = await invoke<string | null>("get_window_session", { windowLabel: label });
            if (persistedId) {
                console.log(`[SchemaStore] Found persisted session: ${persistedId} `);
                // Load connection metadata
                const conn = await invoke<Connection>("get_connection_metadata", { id: persistedId });
                if (conn) {
                    await this.connect(conn);
                }
            }
        } catch (e) {
            console.error("[SchemaStore] Failed to restore session:", e);
        }
    }

    async connect(conn: Connection) {
        const previousId = this.activeConnection?.id;

        this.status = "connecting";
        this.error = null;
        this.activeConnection = conn;
        this.databases = []; // Clear previous schemas immediately
        this.lastRefreshed = null;

        try {
            // 1. Validate connection
            await invoke("test_connection_by_id", { id: conn.id });

            // 2. Mark as active in backend (with persistence)
            await invoke("mark_connection_active", {
                id: conn.id,
                windowLabel: this.windowLabel
            });

            // 3. Mark previous as inactive if it's different
            if (previousId && previousId !== conn.id) {
                invoke("mark_connection_inactive", {
                    id: previousId,
                    windowLabel: this.windowLabel
                }).catch(console.error);
            }

            // 4. Fetch Schema (Cached)
            const data = await invoke<MetaDatabase[]>("get_schema", { connectionId: conn.id });

            // 5. Update Completion Engine Cache
            await invoke("update_completion_schema", {
                connectionId: conn.id,
                databases: data
            });

            this.databases = data;
            this.status = "idle";
            this.lastRefreshed = new Date();

            if (data.length === 0) {
                toast.success("Connected", { description: "No schema found. Try refreshing." });
            } else {
                toast.success("Connected", { description: `Loaded ${data.length} schemas.` });
            }

        } catch (e) {
            this.status = "error";
            this.error = String(e);
            toast.error("Connection Failed", { description: String(e) });

            invoke("mark_connection_inactive", {
                id: conn.id,
                windowLabel: this.windowLabel
            }).catch(console.error);
        }
    }

    async disconnect() {
        if (!this.activeConnection) return;

        const id = this.activeConnection.id;
        this.activeConnection = null;
        this.databases = [];
        this.status = "idle";

        try {
            await invoke("mark_connection_inactive", {
                id,
                windowLabel: this.windowLabel
            });
            toast.success("Disconnected");
        } catch (e) {
            console.error("Disconnect failed:", e);
        }
    }

    async loadDatabase(dbName: string) {
        if (!this.activeConnection) return;

        const db = this.databases.find(d => d.name === dbName);
        if (!db) return;

        // If already connected and has schemas, don't reload automatically
        if (db.is_connected && db.schemas.length > 0) return;

        db.is_loading = true;

        try {
            console.log(`[SchemaStore] Loading database: ${dbName}`);
            const updatedDb = await invoke<MetaDatabase>("introspect_database", {
                connectionId: this.activeConnection.id,
                databaseName: dbName
            });

            // Update the database in the list
            const index = this.databases.findIndex(d => d.name === dbName);
            if (index !== -1) {
                // Ensure we keep is_connected true if it was returned so by backend
                this.databases[index] = { ...updatedDb, is_loading: false };
            }

            // Sync completion cache
            await invoke("update_completion_schema", {
                connectionId: this.activeConnection.id,
                databases: this.databases
            });

            toast.success(`Database Loaded`, { description: `Successfully loaded schemas for ${dbName}` });
        } catch (e) {
            // Restore loading state
            const index = this.databases.findIndex(d => d.name === dbName);
            if (index !== -1) {
                this.databases[index].is_loading = false;
            }
            toast.error("Load Failed", { description: String(e) });
        }
    }

    async refresh() {
        if (!this.activeConnection) return;

        this.status = "refreshing";
        this.error = null;

        try {
            await invoke("refresh_schema", { connectionId: this.activeConnection.id });
            const data = await invoke<MetaDatabase[]>("get_schema", { connectionId: this.activeConnection.id });

            // Sync completion cache
            await invoke<void>("update_completion_schema", {
                connectionId: this.activeConnection.id,
                databases: data
            });

            this.databases = data;
            this.status = "idle";
            this.lastRefreshed = new Date();
            toast.success("Schema Refreshed");
        } catch (e) {
            this.status = "idle";
            toast.error("Refresh Failed", { description: String(e) });
        }
    }
}

export const schemaStore = new SchemaStore();
