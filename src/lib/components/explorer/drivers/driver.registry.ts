/**
 * Driver Registry
 * 
 * Factory for creating database drivers based on connection engine type.
 */

import type { DatabaseDriver } from './driver.types';
import { PostgresDriver } from './PostgresDriver';
import { SqliteDriver } from './SqliteDriver';

export type SupportedEngine = 'postgres' | 'sqlite';

/**
 * Create a database driver for the given engine type.
 * 
 * @param engine - Database engine type
 * @param connectionId - Unique connection identifier
 * @param database - Database name to scope the driver to
 * @returns DatabaseDriver instance
 * @throws Error if engine is not supported
 */
export function createDriver(
    engine: SupportedEngine | string,
    connectionId: string,
    database: string
): DatabaseDriver {
    switch (engine) {
        case 'postgres':
            return new PostgresDriver(connectionId, database);
        case 'sqlite':
            return new SqliteDriver(connectionId, database);
        default:
            throw new Error(`Unsupported database engine: ${engine}`);
    }
}

/**
 * Check if an engine type is supported.
 */
export function isEngineSupported(engine: string): engine is SupportedEngine {
    return ['postgres', 'sqlite'].includes(engine);
}
