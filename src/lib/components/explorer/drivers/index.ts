/**
 * Explorer Drivers
 * 
 * Re-export driver interfaces, types, and implementations.
 */

// Types and interfaces
export type {
    ExplorerNode,
    ExplorerNodeMetadata,
    DatabaseDriver,
    NodeType,
    IconType,
    MetaSchema,
    MetaTable,
    MetaColumn,
    MetaForeignKey,
    MetaIndex,
    MetaTrigger,
} from './driver.types';

export { getIconForNodeType } from './driver.types';

// Driver implementations
export { PostgresDriver } from './PostgresDriver';
export { SqliteDriver } from './SqliteDriver';

// Registry
export { createDriver, isEngineSupported, type SupportedEngine } from './driver.registry';
