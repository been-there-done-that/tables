import { invoke } from "@tauri-apps/api/core";
import { toast } from "svelte-sonner";
import LoaderIcon from "@tabler/icons-svelte/icons/loader-2";
import CheckIcon from "@tabler/icons-svelte/icons/check";
import XIcon from "@tabler/icons-svelte/icons/x";
import type { Connection, MetaDatabase } from "$lib/commands/types";
import { settingsStore } from "./settings.svelte";

export class SchemaStore {
    activeConnection = $state<Connection | null>(null);
    status = $state<"idle" | "connecting" | "refreshing" | "error">("idle");
    databases = $state<MetaDatabase[]>([]);
    selectedDatabase = $state<string | null>(null);
    error = $state<string | null>(null);
    lastRefreshed = $state<Date | null>(null);
    windowLabel = $state<string | null>(null);
    activeSchema = $state<string | null>("public");
    activeConnectionIds = $state<string[]>([]);
    private unlistenActive: (() => void) | null = null;

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

        // Listen for active connection changes from backend
        import("@tauri-apps/api/event").then(({ listen }) => {
            listen<string[]>("active-connections-changed", (event) => {
                this.activeConnectionIds = event.payload;
            }).then(un => this.unlistenActive = un);
        });

        // Initial fetch
        this.refreshActiveConnections();
    }

    async connect(conn: Connection) {
        const previousId = this.activeConnection?.id;

        // Guard against duplicate concurrent connects to same connection
        if (this.status === "connecting" && this.activeConnection?.id === conn.id) {
            console.log(`[SchemaStore] Already connecting to ${conn.id}, skipping duplicate`);
            return;
        }

        this.status = "connecting";
        this.error = null;
        this.activeConnection = conn;
        this.databases = []; // Clear previous schemas immediately
        this.selectedDatabase = null;
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
            console.time("[SchemaStore] get_schema");
            let data = await invoke<MetaDatabase[]>("get_schema", { connectionId: conn.id });
            console.timeEnd("[SchemaStore] get_schema");
            console.log(`[SchemaStore] get_schema returned ${data.length} databases`);

            // 4.1 If cache is empty (first-time connection), trigger introspection
            if (data.length === 0) {
                const loadingToastId = toast.loading("Introspecting schema...", {
                    description: "First-time connection, discovering database structure.",
                    duration: Infinity,
                });

                try {
                    console.time("[SchemaStore] refresh_schema");
                    await invoke("refresh_schema", { connectionId: conn.id });
                    console.timeEnd("[SchemaStore] refresh_schema");

                    console.time("[SchemaStore] get_schema (after refresh)");
                    data = await invoke<MetaDatabase[]>("get_schema", { connectionId: conn.id });
                    console.timeEnd("[SchemaStore] get_schema (after refresh)");
                    console.log(`[SchemaStore] After refresh: ${data.length} databases`);

                    toast.success("Schema Loaded", {
                        id: loadingToastId,
                        description: `Discovered ${data.length} databases.`
                    });
                } catch (introError) {
                    toast.error("Introspection Failed", {
                        id: loadingToastId,
                        description: String(introError)
                    });
                }
            }

            // 5. Update Completion Engine Cache
            console.time("[SchemaStore] update_completion_schema");
            await invoke("update_completion_schema", {
                connectionId: conn.id,
                databases: data,
                selectedDatabase: conn.database  // Filter by configured database
            });
            console.timeEnd("[SchemaStore] update_completion_schema");

            console.time("[SchemaStore] state update");
            this.databases = data;

            // Auto-select database
            if (this.databases.length > 0) {
                // Wait for settings if they are still loading to ensure we have the persisted choice
                await settingsStore.waitForInit();

                // 1. Try persisted database from settings
                const persistedDb = settingsStore.selectedDatabase ? this.databases.find(d => d.name === settingsStore.selectedDatabase) : null;

                // 2. Try configured database from connection
                const configuredDb = this.databases.find(d => d.name === conn.database);

                if (persistedDb) {
                    this.selectedDatabase = persistedDb.name;
                    console.log(`[SchemaStore] Restored persisted database: ${this.selectedDatabase}`);
                } else if (configuredDb) {
                    this.selectedDatabase = configuredDb.name;
                    console.log(`[SchemaStore] Selected configured database: ${this.selectedDatabase}`);
                } else {
                    // 3. Fallback to first available
                    this.selectedDatabase = this.databases[0].name;
                    console.log(`[SchemaStore] Fallback to first database: ${this.selectedDatabase}`);
                }
            }

            this.status = "idle";
            this.lastRefreshed = new Date();
            console.timeEnd("[SchemaStore] state update");
            console.log(`[SchemaStore] Status changed to: ${this.status}, databases count: ${this.databases.length}`);

            if (data.length === 0) {
                toast.success("Connected", { description: "No schema found. Try refreshing." });
            } else {
                toast.success("Connected", { description: `Loaded ${data.length} databases.` });
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
        this.selectedDatabase = null;
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

        // If already introspected, just show it (the FileTree handle the state)
        if (db.is_introspected) return;

        db.is_loading = true;
        const toastId = toast.loading(`Introspecting ${dbName}...`, {
            description: "Fetching schemas and tables information.",
            duration: Infinity,
        });

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

            // Sync completion cache - use the just-loaded database
            await invoke("update_completion_schema", {
                connectionId: this.activeConnection.id,
                databases: this.databases,
                selectedDatabase: dbName
            });

            toast.success(`Database Loaded`, {
                id: toastId,
                description: `Successfully introspected ${dbName}`
            });
        } catch (e) {
            // Restore loading state
            const index = this.databases.findIndex(d => d.name === dbName);
            if (index !== -1) {
                this.databases[index].is_loading = false;
            }
            toast.error("Load Failed", {
                id: toastId,
                description: String(e)
            });
        }
    }

    async refresh() {
        if (!this.activeConnection) return;

        // Optimistic update: mark all as loading
        this.databases = this.databases.map(d => ({ ...d, is_loading: true }));
        this.status = "refreshing"; // Keep this for titlebar spinner
        this.error = null;

        try {
            await invoke("refresh_schema", { connectionId: this.activeConnection.id });
            const data = await invoke<MetaDatabase[]>("get_schema", { connectionId: this.activeConnection.id });

            // Sync completion cache
            await invoke<void>("update_completion_schema", {
                connectionId: this.activeConnection.id,
                databases: data,
                selectedDatabase: this.selectedDatabase
            });

            this.databases = data;
            this.status = "idle";
            this.lastRefreshed = new Date();
            toast.success("Schema Refreshed");
        } catch (e) {
            this.status = "idle";
            toast.error("Refresh Failed", { description: String(e) });
            toast.error("Refresh Failed", { description: String(e) });
        }
    }

    selectDatabase(name: string) {
        if (this.databases.find(d => d.name === name)) {
            this.selectedDatabase = name;
            // Persist choice to settings
            settingsStore.selectedDatabase = name;
            // Trigger load if needed (optional, or rely on tree expansion)
            this.loadDatabase(name);
        }
    }

    async refreshActiveConnections() {
        try {
            this.activeConnectionIds = await invoke<string[]>("get_active_connections");
        } catch (e) {
            console.error("[SchemaStore] Failed to fetch active connections:", e);
        }
    }

    isConnectionBusy(connId: string) {
        // A connection is busy if it's active in another window
        // (i.e. it's in the global active list but isn't our window's active connection)
        return this.activeConnectionIds.includes(connId) && this.activeConnection?.id !== connId;
    }

    cleanup() {
        if (this.unlistenActive) {
            this.unlistenActive();
            this.unlistenActive = null;
        }
    }
}

export const schemaStore = new SchemaStore();
