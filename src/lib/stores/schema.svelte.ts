import { invoke } from "@tauri-apps/api/core";
import { toast } from "svelte-sonner";
import LoaderIcon from "@tabler/icons-svelte/icons/loader-2";
import CheckIcon from "@tabler/icons-svelte/icons/check";
import XIcon from "@tabler/icons-svelte/icons/x";
import type { Connection, MetaDatabase } from "$lib/commands/types";

export class SchemaStore {
    activeConnection = $state<Connection | null>(null);
    status = $state<"idle" | "connecting" | "refreshing" | "error">("idle");
    databases = $state<MetaDatabase[]>([]);
    selectedDatabase = $state<string | null>(null);
    error = $state<string | null>(null);
    lastRefreshed = $state<Date | null>(null);
    windowLabel = $state<string | null>(null);
    activeSchema = $state<string | null>("public");
    private unlistenLevel: (() => void) | null = null;
    private unlistenReady: (() => void) | null = null;

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

            // Setup listeners
            if (this.unlistenLevel) this.unlistenLevel();
            if (this.unlistenReady) this.unlistenReady();

            const { listen } = await import("@tauri-apps/api/event");
            this.unlistenLevel = await listen("schema:level-complete", async (event: any) => {
                console.log(`[SchemaStore] Level complete:`, event.payload);
                await this.syncFromCache();
            });

            this.unlistenReady = await listen("schema:ready", async (event: any) => {
                console.log(`[SchemaStore] Schema ready:`, event.payload);
                await this.syncFromCache();
                // Unblock UI
                if (this.status === "refreshing") {
                    this.status = "idle";
                    toast.success("Schema prioritized and ready", {
                        description: `Tables for ${event.payload.schema} are now interactive.`
                    });
                }
            });

        } catch (e) {
            console.error("[SchemaStore] Failed to restore session:", e);
        }
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

            // 4. Fetch Databases (Cached)
            console.time("[SchemaStore] fetchDatabases");
            await this.fetchDatabases();
            console.timeEnd("[SchemaStore] fetchDatabases");

            // 4.1 If no databases found, trigger introspection (progressive refresh)
            if (this.databases.length === 0) {
                const loadingToastId = toast.loading("Introspecting schema...", {
                    description: "First-time connection, discovering database structure.",
                    duration: Infinity,
                });

                try {
                    console.time("[SchemaStore] refresh_schema_progressive");
                    await invoke("refresh_schema_progressive", { connectionId: conn.id });
                    console.timeEnd("[SchemaStore] refresh_schema_progressive");

                    await this.fetchDatabases();
                    console.log(`[SchemaStore] After refresh: ${this.databases.length} databases`);

                    toast.success("Connection Established", {
                        id: loadingToastId,
                        description: `Discovered structures for ${this.databases.length} databases.`
                    });
                } catch (introError) {
                    toast.error("Introspection Failed", {
                        id: loadingToastId,
                        description: String(introError)
                    });
                }
            }

            // 5. Update Completion Engine Cache with whatever we have
            await invoke("update_completion_schema", {
                connectionId: conn.id,
                databases: $state.snapshot(this.databases),
                selectedDatabase: conn.database
            });

            // Auto-select database
            if (this.databases.length > 0) {
                const configuredDb = this.databases.find(d => d.name === conn.database);
                if (configuredDb) {
                    this.selectedDatabase = configuredDb.name;
                    await this.fetchSchemas(configuredDb.name);
                } else {
                    this.selectedDatabase = this.databases[0].name;
                    await this.fetchSchemas(this.databases[0].name);
                }
                console.log(`[SchemaStore] Auto-selected database: ${this.selectedDatabase}`);
            }

            this.status = "idle";
            this.lastRefreshed = new Date();
            toast.success("Connected", { description: `Session active for ${this.activeConnection.name}` });

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

    async fetchDatabases() {
        if (!this.activeConnection) return;
        const dbs = await invoke<MetaDatabase[]>("get_databases", { connectionId: this.activeConnection.id });
        // Preserve existing branches if they were loaded
        this.databases = dbs.map(newDb => {
            const existing = this.databases.find(d => d.name === newDb.name);
            if (existing) {
                return { ...newDb, schemas: existing.schemas, is_introspected: existing.is_introspected };
            }
            return newDb;
        });
    }

    async fetchSchemas(dbName: string) {
        if (!this.activeConnection) return;
        const dbIndex = this.databases.findIndex(d => d.name === dbName);
        if (dbIndex === -1) return;

        const schemas = await invoke<any[]>("get_schemas", {
            connectionId: this.activeConnection.id,
            database: dbName
        });

        this.databases[dbIndex].schemas = schemas.map(s => {
            const existing = this.databases[dbIndex].schemas.find(es => es.name === s.name);
            if (existing) {
                return { ...s, tables: existing.tables };
            }
            return { ...s, tables: [] };
        });
        this.databases[dbIndex].is_introspected = schemas.length > 0;
    }

    async fetchTables(dbName: string, schemaName: string) {
        if (!this.activeConnection) return;
        const dbIndex = this.databases.findIndex(d => d.name === dbName);
        if (dbIndex === -1) return;

        const schemaIndex = this.databases[dbIndex].schemas.findIndex(s => s.name === schemaName);
        if (schemaIndex === -1) return;

        const tables = await invoke<any[]>("get_tables_in_schema", {
            connectionId: this.activeConnection.id,
            database: dbName,
            schema: schemaName
        });

        this.databases[dbIndex].schemas[schemaIndex].tables = tables;
    }

    async syncFromCache() {
        if (!this.activeConnection) return;

        // In lazy mode, syncFromCache should probably refresh everything we've expanded?
        // Or just re-fetch databases and update completion engine.
        // For simplicity, we re-fetch databases and the selected database's schemas.
        await this.fetchDatabases();
        if (this.selectedDatabase) {
            await this.fetchSchemas(this.selectedDatabase);
            // Also refresh tables for any expanded schemas? Maybe overkill for syncFromCache.
        }

        await invoke<void>("update_completion_schema", {
            connectionId: this.activeConnection.id,
            databases: $state.snapshot(this.databases),
            selectedDatabase: this.selectedDatabase
        });

        this.lastRefreshed = new Date();
    }

    async refresh(databaseName?: string, schemaName?: string) {
        if (!this.activeConnection) return;

        this.status = "refreshing";
        this.error = null;

        try {
            if (databaseName && schemaName) {
                // Targeted refresh
                await invoke("refresh_schema_specific_progressive", {
                    connectionId: this.activeConnection.id,
                    databaseName,
                    schemaName
                });
            } else {
                // Global progressive refresh with priority
                await invoke("refresh_schema_progressive", {
                    connectionId: this.activeConnection.id,
                    priorityDatabase: this.selectedDatabase,
                    prioritySchema: this.activeSchema
                });
            }

            this.status = "idle";
            toast.success("Refresh Complete");
        } catch (e) {
            this.status = "idle";
            toast.error("Refresh Failed", { description: String(e) });
        }
    }

    selectDatabase(name: string) {
        if (this.databases.find(d => d.name === name)) {
            this.selectedDatabase = name;
            // Trigger load if needed (optional, or rely on tree expansion)
            this.loadDatabase(name);
        }
    }
}

export const schemaStore = new SchemaStore();
