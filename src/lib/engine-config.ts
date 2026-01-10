/**
 * Engine Configuration
 * 
 * Centralized configuration for different database engines.
 * Add new engine types here as support is added.
 */

export type EngineType =
    | "postgres"
    | "postgresql"
    | "sqlite"
    | "mysql"
    | "mariadb"
    | "sqlserver"
    | "mssql"
    | "oracle"
    | "mongodb"
    | "redis"
    | "athena";

export interface EngineDefaults {
    /** Default database name when none is selected */
    defaultDatabase: string;
    /** Default schema name when none is selected */
    defaultSchema: string;
    /** Whether this engine supports multiple databases */
    supportsMultipleDatabases: boolean;
    /** Whether this engine supports schemas */
    supportsSchemas: boolean;
    /** Display name for the engine */
    displayName: string;
}

/**
 * Engine configuration map
 * Add new engines here as they are supported
 */
const ENGINE_CONFIG: Record<string, EngineDefaults> = {
    // PostgreSQL
    postgres: {
        defaultDatabase: "postgres",
        defaultSchema: "public",
        supportsMultipleDatabases: true,
        supportsSchemas: true,
        displayName: "PostgreSQL",
    },
    postgresql: {
        defaultDatabase: "postgres",
        defaultSchema: "public",
        supportsMultipleDatabases: true,
        supportsSchemas: true,
        displayName: "PostgreSQL",
    },

    // SQLite
    sqlite: {
        defaultDatabase: "main",
        defaultSchema: "main",
        supportsMultipleDatabases: false,
        supportsSchemas: false,
        displayName: "SQLite",
    },

    // MySQL / MariaDB
    mysql: {
        defaultDatabase: "",  // No default, use first available
        defaultSchema: "",    // MySQL uses database as schema
        supportsMultipleDatabases: true,
        supportsSchemas: false,  // Schemas are aliases for databases in MySQL
        displayName: "MySQL",
    },
    mariadb: {
        defaultDatabase: "",
        defaultSchema: "",
        supportsMultipleDatabases: true,
        supportsSchemas: false,
        displayName: "MariaDB",
    },

    // SQL Server
    sqlserver: {
        defaultDatabase: "master",
        defaultSchema: "dbo",
        supportsMultipleDatabases: true,
        supportsSchemas: true,
        displayName: "SQL Server",
    },
    mssql: {
        defaultDatabase: "master",
        defaultSchema: "dbo",
        supportsMultipleDatabases: true,
        supportsSchemas: true,
        displayName: "SQL Server",
    },

    // Oracle
    oracle: {
        defaultDatabase: "",  // Oracle doesn't have databases in the PostgreSQL sense
        defaultSchema: "",    // Default to user's schema
        supportsMultipleDatabases: false,
        supportsSchemas: true,
        displayName: "Oracle",
    },

    // MongoDB (document DB, different paradigm)
    mongodb: {
        defaultDatabase: "test",
        defaultSchema: "",  // Collections, not schemas
        supportsMultipleDatabases: true,
        supportsSchemas: false,
        displayName: "MongoDB",
    },

    // Redis (key-value, different paradigm)
    redis: {
        defaultDatabase: "0",  // Redis uses numeric database IDs
        defaultSchema: "",
        supportsMultipleDatabases: true,
        supportsSchemas: false,
        displayName: "Redis",
    },

    // AWS Athena
    athena: {
        defaultDatabase: "default",
        defaultSchema: "",
        supportsMultipleDatabases: true,
        supportsSchemas: false,
        displayName: "AWS Athena",
    },
};

/**
 * Default configuration for unknown engines
 */
const DEFAULT_ENGINE_CONFIG: EngineDefaults = {
    defaultDatabase: "",
    defaultSchema: "public",
    supportsMultipleDatabases: true,
    supportsSchemas: true,
    displayName: "Unknown",
};

/**
 * Get engine configuration for a given engine type
 * @param engine - The engine type identifier
 * @returns Engine configuration with defaults
 */
export function getEngineConfig(engine: string | undefined | null): EngineDefaults {
    if (!engine) return DEFAULT_ENGINE_CONFIG;

    const normalizedEngine = engine.toLowerCase().trim();
    return ENGINE_CONFIG[normalizedEngine] || DEFAULT_ENGINE_CONFIG;
}

/**
 * Get the default database for an engine
 * @param engine - The engine type identifier
 * @returns Default database name
 */
export function getDefaultDatabase(engine: string | undefined | null): string {
    return getEngineConfig(engine).defaultDatabase;
}

/**
 * Get the default schema for an engine
 * @param engine - The engine type identifier
 * @returns Default schema name
 */
export function getDefaultSchema(engine: string | undefined | null): string {
    return getEngineConfig(engine).defaultSchema;
}

/**
 * Check if an engine supports schemas
 * @param engine - The engine type identifier
 * @returns true if engine supports schemas
 */
export function engineSupportsSchemas(engine: string | undefined | null): boolean {
    return getEngineConfig(engine).supportsSchemas;
}

/**
 * Check if an engine supports multiple databases
 * @param engine - The engine type identifier
 * @returns true if engine supports multiple databases
 */
export function engineSupportsMultipleDatabases(engine: string | undefined | null): boolean {
    return getEngineConfig(engine).supportsMultipleDatabases;
}

/**
 * Get display name for an engine
 * @param engine - The engine type identifier
 * @returns Human-readable engine name
 */
export function getEngineDisplayName(engine: string | undefined | null): string {
    return getEngineConfig(engine).displayName;
}
