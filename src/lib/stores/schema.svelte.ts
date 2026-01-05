import { invoke } from "@tauri-apps/api/core";
import { toast } from "svelte-sonner";
import LoaderIcon from "@tabler/icons-svelte/icons/loader-2";
import CheckIcon from "@tabler/icons-svelte/icons/check";
import XIcon from "@tabler/icons-svelte/icons/x";
import type { Connection, MetaDatabase, MetaSchema } from "$lib/commands/types";

export class SchemaStore {
    activeConnection = $state<Connection | null>(null);
    status = $state<"idle" | "connecting" | "refreshing" | "error">("idle");
    statusMessage = $state<string>("Ready");
    databases = $state<MetaDatabase[]>([]);
    selectedDatabase = $state<string | null>(null);
    error = $state<string | null>(null);
    lastRefreshed = $state<Date | null>(null);
    windowLabel = $state<string | null>(null);
    activeSchema = $state<string | null>("public");
    private unlistenLevel: (() => void) | null = null;
    private unlistenSchema: (() => void) | null = null;
    private unlistenComplete: (() => void) | null = null;
    private unlistenError: (() => void) | null = null;

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
            if (this.unlistenSchema) this.unlistenSchema();
            if (this.unlistenComplete) this.unlistenComplete();
            if (this.unlistenError) this.unlistenError();

            const { listen } = await import("@tauri-apps/api/event");

            this.unlistenLevel = await listen("introspection:level_complete", async (event: any) => {
                const payload = event.payload;
                console.log(`[SchemaStore] Level ${payload.level} complete:`, payload);
                this.statusMessage = `Introspected Level ${payload.level}: ${payload.database || 'Metadata'}`;

                // Update Toast Progressively if applicable
                if (payload.database) {
                    const toastId = `load-db-${payload.database}`;
                    if (payload.level === 2) {
                        toast.loading(`Loading Schemas (${payload.database})...`, {
                            id: toastId,
                            description: `Found ${payload.schema_count || 0} schemas. Fetching tables...`,
                            duration: Infinity,
                        });
                    } else if (payload.level === 3) {
                        toast.loading(`Loading Tables (${payload.database})...`, {
                            id: toastId,
                            description: `Found ${payload.table_count || 0} tables. Finalizing...`,
                            duration: Infinity,
                        });
                    }
                }

                // Level 1 = Databases list changed
                const isDbListUpdate = payload.level === 1;
                await this.syncFromCache({
                    database: payload.database,
                    refreshDbList: isDbListUpdate
                });
            });

            this.unlistenSchema = await listen("introspection:schema_ready", async (event: any) => {
                const payload = event.payload;
                console.log(`[SchemaStore] Schema ready:`, payload);
                this.statusMessage = `Schema ready: ${payload.database}`;
                await this.syncFromCache({ database: payload.database, refreshDbList: false });

                if (this.status === "refreshing") {
                    this.status = "idle";
                    // Handled by loadDatabase promise resolution
                }
            });

            this.unlistenComplete = await listen("introspection:complete", async (event: any) => {
                const payload = event.payload;
                console.log(`[SchemaStore] Introspection complete:`, payload);
                this.statusMessage = "Ready";
                this.status = "idle";

                let dbName: string | undefined;
                let isGlobal = false;

                if (payload.database) {
                    dbName = payload.database;
                    isGlobal = false;
                } else if (payload.scope) {
                    if (payload.scope.type === 'database') {
                        dbName = payload.scope.name;
                    } else if (payload.scope.type === 'schema' || payload.scope.type === 'table') {
                        dbName = payload.scope.database;
                    } else {
                        isGlobal = true;
                    }
                } else {
                    isGlobal = true;
                }

                await this.syncFromCache({ database: dbName, refreshDbList: isGlobal });
            });

            this.unlistenError = await listen("introspection:error", async (event: any) => {
                const payload = event.payload;
                console.error(`[SchemaStore] Introspection error:`, payload);
                this.statusMessage = `Error: ${payload.message}`;
                this.status = "error";
                this.error = payload.message;
                toast.error("Introspection Error", { description: payload.message });
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
        this.databases = [];
        this.selectedDatabase = conn.database || null;
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

            // 4.1 If no databases found, trigger introspection (unified refresh)
            if (this.databases.length === 0) {
                const loadingToastId = toast.loading("Introspecting schema...", {
                    description: "First-time connection, discovering database structure.",
                    duration: Infinity,
                });

                try {
                    console.time("[SchemaStore] refresh_schema_unified");
                    this.statusMessage = "Discovering databases...";
                    await invoke("refresh_schema_unified", {
                        connectionId: conn.id,
                        options: {
                            scope: { type: 'global' },
                            priority_database: conn.database
                        }
                    });
                    console.timeEnd("[SchemaStore] refresh_schema_unified");

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

        // Smart check: if already cached, just fetch schemas from cache
        if (db.is_introspected) {
            console.log(`[SchemaStore] Database ${dbName} is already cached, fetching schemas from cache.`);
            await this.fetchSchemas(dbName);
            return;
        }

        if (db.is_loading || this.status === "refreshing") {
            console.log(`[SchemaStore] Database ${dbName} is already loading or store is refreshing. Skipping.`);
            return;
        }

        db.is_loading = true;
        this.status = "refreshing";

        const toastId = `load-db-${dbName}`;
        toast.loading(`Introspecting ${dbName}...`, {
            id: toastId,
            description: "Fetching schemas and tables information.",
            duration: Infinity,
        });

        try {
            console.log(`[SchemaStore] Loading database: ${dbName} (Remote Fetch)`);
            this.statusMessage = `Introspecting ${dbName}...`;
            await invoke("refresh_schema_unified", {
                connectionId: this.activeConnection.id,
                options: {
                    scope: { type: 'database', name: dbName }
                }
            });

            // Update the database in the list via refresh
            await this.fetchDatabases();
            const updatedDb = this.databases.find(d => d.name === dbName);

            // Sync completion cache
            await invoke("update_completion_schema", {
                connectionId: this.activeConnection.id,
                databases: $state.snapshot(this.databases),
                selectedDatabase: dbName
            });

            toast.success(`Database Loaded`, {
                id: toastId,
                description: `Successfully introspected ${dbName}`
            });
        } catch (e) {
            toast.error("Load Failed", {
                id: toastId,
                description: String(e)
            });
        } finally {
            this.status = "idle";
            const index = this.databases.findIndex(d => d.name === dbName);
            if (index !== -1) {
                this.databases[index].is_loading = false;
            }
        }
    }

    async fetchDatabases() {
        if (!this.activeConnection) return;
        const dbs = await invoke<MetaDatabase[]>("get_cached_databases", { connectionId: this.activeConnection.id });
        // Preserve existing branches if they were loaded
        this.databases = dbs.map(newDb => {
            const existing = this.databases.find(d => d.name === newDb.name);
            if (existing) {
                return { ...newDb, schemas: existing.schemas };
            }
            return newDb;
        });
        console.log(`[SchemaStore] fetchDatabases: found ${this.databases.length} dbs. selected: ${this.selectedDatabase}`);
    }

    async fetchSchemas(dbName: string) {
        if (!this.activeConnection) return;
        const dbIndex = this.databases.findIndex(d => d.name === dbName);
        if (dbIndex === -1) return;

        const schemas = await invoke<MetaSchema[]>("get_cached_schemas", {
            connectionId: this.activeConnection.id,
            database: dbName
        });

        console.log(`[SchemaStore] fetchSchemas(${dbName}): found ${schemas.length} schemas. is_introspected: ${this.databases[dbIndex].is_introspected}`);

        // If no schemas in cache, trigger a remote load if not already introspected
        if (schemas.length === 0 && !this.databases[dbIndex].is_introspected) {
            if (this.status !== "idle" || this.databases[dbIndex].is_loading) {
                console.log(`[SchemaStore] fetchSchemas: No schemas in cache for ${dbName}, but store is busy (${this.status}) or db is loading. Skipping auto-trigger.`);
                return;
            }
            console.log(`[SchemaStore] No schemas in cache for ${dbName}, triggering load.`);
            await this.loadDatabase(dbName);
            return;
        }

        this.databases[dbIndex].schemas = schemas.map(s => {
            const existing = this.databases[dbIndex].schemas.find(es => es.name === s.name);
            if (existing) {
                return {
                    ...s,
                    tables: existing.tables,
                    is_introspected: existing.is_introspected
                };
            }
            return { ...s, tables: [] };
        });

        // Update is_introspected if we found schemas
        if (schemas.length > 0) {
            this.databases[dbIndex].is_introspected = true;
        }
    }

    async fetchTables(dbName: string, schemaName: string) {
        if (!this.activeConnection) return;
        const dbIndex = this.databases.findIndex(d => d.name === dbName);
        if (dbIndex === -1) return;

        const schemaIndex = this.databases[dbIndex].schemas.findIndex(s => s.name === schemaName);
        if (schemaIndex === -1) return;

        const schema = this.databases[dbIndex].schemas[schemaIndex];

        console.log(`[SchemaStore] fetchTables(${dbName}, ${schemaName}) - is_introspected: ${schema.is_introspected}`);

        // If not cached, trigger a specific refresh
        // If not cached, trigger a specific refresh
        if (!schema.is_introspected) {
            // Guard: If busy, wait instead of dropping
            if (this.status !== "idle" || this.databases[dbIndex].is_loading) {
                console.log(`[SchemaStore] fetchTables: Store is busy. Waiting for idle...`);
                // Simple poll
                await this.waitForIdle();
                // Recursively retry once
                return this.fetchTables(dbName, schemaName);
            }

            console.log(`[SchemaStore] Schema ${schemaName} not cached, triggering remote fetch.`);

            this.status = "refreshing";
            // ... remote fetch logic ...
            try {
                this.statusMessage = `Introspecting ${schemaName}...`;
                await invoke("refresh_schema_unified", {
                    connectionId: this.activeConnection.id,
                    options: {
                        scope: { type: 'schema', database: dbName, schema: schemaName }
                    }
                });
                // ... rest of logic
            } catch (e) {
                console.error(`Failed to refresh schema ${schemaName}:`, e);
                toast.error(`Failed to load ${schemaName}`, { description: String(e) });
                this.status = "idle"; // Ensure reset
                return;
            } finally {
                this.status = "idle";
            }

            // Fall through to fetch from cache
        }

        console.log(`[SchemaStore] Schema cached, fetching from local cache.`);
        const tables = await invoke<any[]>("get_cached_tables", {
            connectionId: this.activeConnection.id,
            database: dbName,
            schema: schemaName
        });

        console.log(`[SchemaStore] Fetched ${tables.length} tables from cache for ${schemaName}`);

        this.databases[dbIndex].schemas[schemaIndex].tables = tables;
        this.databases[dbIndex].schemas[schemaIndex].is_introspected = true;
    }

    private syncTimeout: ReturnType<typeof setTimeout> | null = null;
    private pendingSyncDbs = new Set<string>();
    private pendingDbListRefresh = false;

    async syncFromCache(options?: { database?: string, refreshDbList?: boolean }) {
        if (!this.activeConnection) return;

        if (options?.refreshDbList) {
            this.pendingDbListRefresh = true;
        }

        this.pendingSyncDbs.add(options?.database || '__GLOBAL__');

        if (this.syncTimeout) {
            clearTimeout(this.syncTimeout);
        }

        this.syncTimeout = setTimeout(() => {
            this._performSync();
        }, 200);
    }

    private async _performSync() {
        if (!this.activeConnection) return;

        const dbsToSync = new Set(this.pendingSyncDbs);
        const startDbListRefresh = this.pendingDbListRefresh;

        this.pendingSyncDbs.clear();
        this.pendingDbListRefresh = false;
        this.syncTimeout = null;

        if (startDbListRefresh) {
            await this.fetchDatabases();
        }

        const shouldRefreshSchemas =
            dbsToSync.has('__GLOBAL__') ||
            (this.selectedDatabase && dbsToSync.has(this.selectedDatabase));

        if (shouldRefreshSchemas && this.selectedDatabase) {
            await this.fetchSchemas(this.selectedDatabase);
        }

        await invoke<void>("update_completion_schema", {
            connectionId: this.activeConnection.id,
            databases: $state.snapshot(this.databases),
            selectedDatabase: this.selectedDatabase
        });

        this.lastRefreshed = new Date();
    }

    async selectDatabase(dbName: string | null) {
        if (this.selectedDatabase === dbName) return;

        console.log(`[SchemaStore] selectDatabase: ${dbName}`);
        this.selectedDatabase = dbName;

        if (dbName && this.activeConnection) {
            await this.fetchSchemas(dbName);

            // Update completion engine focus
            await invoke("update_completion_schema", {
                connectionId: this.activeConnection.id,
                databases: $state.snapshot(this.databases),
                selectedDatabase: dbName
            });
        }
    }

    async refresh(databaseName?: string, schemaName?: string) {
        if (!this.activeConnection) return;

        console.log(`[SchemaStore] Refresh called for ${databaseName || 'all'} (current status: ${this.status})`);

        if (this.status === "refreshing") {
            console.warn("[SchemaStore] Refresh requested while already refreshing. Throttling.");
            return;
        }

        this.status = "refreshing";
        this.error = null;

        try {
            if (databaseName && schemaName) {
                // Targeted refresh
                this.statusMessage = `Refreshing ${schemaName}...`;
                await invoke("refresh_schema_unified", {
                    connectionId: this.activeConnection.id,
                    options: {
                        scope: { type: 'schema', database: databaseName, schema: schemaName },
                        force: true
                    }
                });
            } else {
                // Global progressive refresh with priority
                this.statusMessage = "Starting refresh...";
                await invoke("refresh_schema_unified", {
                    connectionId: this.activeConnection.id,
                    options: {
                        scope: { type: 'global' },
                        priority_database: this.selectedDatabase || undefined,
                        priority_schema: this.activeSchema || undefined,
                        force: true
                    }
                });
            }

            toast.success("Refresh Complete");
        } catch (e) {
            this.error = String(e);
            toast.error("Refresh Failed", { description: String(e) });
        } finally {
            this.status = "idle";
        }
    }

    async waitForIdle(timeout = 10000) {
        const start = Date.now();
        while (this.status !== "idle") {
            if (Date.now() - start > timeout) {
                console.warn("[SchemaStore] Timed out waiting for idle state.");
                break;
            }
            await new Promise(r => setTimeout(r, 100));
        }
    }
}

export const schemaStore = new SchemaStore();
