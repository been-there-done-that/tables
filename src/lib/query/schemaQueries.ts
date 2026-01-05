/**
 * Schema Query Hooks
 * 
 * TanStack Query hooks for lazy loading database schema data.
 * These hooks provide automatic caching, deduplication, and stale-while-revalidate.
 */

import { createQuery, type CreateQueryOptions } from '@tanstack/svelte-query';
import { invoke } from '@tauri-apps/api/core';

// Types matching the backend
export interface MetaDatabase {
    name: string;
    is_connected: boolean;
    is_introspected: boolean;
    schemas: MetaSchema[];
}

export interface MetaSchema {
    name: string;
    schema_type: string;
    is_introspected: boolean;
    tables: MetaTable[];
}

export interface MetaTable {
    connection_id: string;
    database: string;
    schema: string;
    table_name: string;
    table_type: string;
    classification: string;
    last_introspected_at: number;
    columns: MetaColumn[];
    foreign_keys: MetaForeignKey[];
    indexes: MetaIndex[];
    triggers: MetaTrigger[];
}

export interface MetaColumn {
    connection_id: string;
    database: string;
    schema: string;
    table_name: string;
    ordinal_position: number;
    column_name: string;
    raw_type: string;
    logical_type: string;
    nullable: boolean;
    default_value: string | null;
    is_primary_key: boolean;
}

export interface MetaForeignKey {
    connection_id: string;
    database: string;
    schema: string;
    table_name: string;
    column_name: string;
    ref_schema: string;
    ref_table: string;
    ref_column: string;
    constraint_name: string | null;
    constraint_hash: string;
    seq_no: number;
}

export interface MetaIndex {
    connection_id: string;
    database: string;
    schema: string;
    table_name: string;
    index_name: string;
    is_unique: boolean;
}

export interface MetaTrigger {
    connection_id: string;
    database: string;
    schema: string;
    table_name: string;
    trigger_name: string;
    event: string;
    timing: string;
}

/**
 * Query hook for fetching databases.
 * Uses lazy loading - fetches from cache first, remote if needed.
 * 
 * TanStack Svelte Query expects an accessor function, not a plain object.
 */
export function useDatabases(connectionId: () => string | null | undefined) {
    return createQuery(() => ({
        queryKey: ['databases', connectionId()] as const,
        queryFn: async () => {
            const id = connectionId();
            if (!id) throw new Error('No connection ID');
            return invoke<MetaDatabase[]>('get_databases_lazy', { connectionId: id });
        },
        staleTime: 60_000, // 60 seconds
        enabled: !!connectionId(),
    }));
}

/**
 * Query hook for fetching schemas within a database.
 * Uses lazy loading - fetches from cache first, remote if needed.
 */
export function useSchemas(
    connectionId: () => string | null | undefined,
    database: () => string | null | undefined
) {
    return createQuery(() => ({
        queryKey: ['schemas', connectionId(), database()] as const,
        queryFn: async () => {
            const connId = connectionId();
            const db = database();
            if (!connId || !db) throw new Error('Missing parameters');
            return invoke<MetaSchema[]>('get_schemas_lazy', { connectionId: connId, database: db });
        },
        staleTime: 60_000,
        enabled: !!connectionId() && !!database(),
    }));
}

/**
 * Query hook for fetching tables within a schema.
 * Uses lazy loading - fetches from cache first, remote if needed.
 */
export function useTables(
    connectionId: () => string | null | undefined,
    database: () => string | null | undefined,
    schema: () => string | null | undefined
) {
    return createQuery(() => ({
        queryKey: ['tables', connectionId(), database(), schema()] as const,
        queryFn: async () => {
            const connId = connectionId();
            const db = database();
            const sch = schema();
            if (!connId || !db || !sch) throw new Error('Missing parameters');
            return invoke<MetaTable[]>('get_tables_lazy', { connectionId: connId, database: db, schema: sch });
        },
        staleTime: 30_000, // 30 seconds - tables may change more frequently
        enabled: !!connectionId() && !!database() && !!schema(),
    }));
}

/**
 * Query hook for fetching table details (columns, indexes, FKs).
 * Uses the existing cached command.
 */
export function useTableDetails(
    connectionId: () => string | null | undefined,
    database: () => string | null | undefined,
    schema: () => string | null | undefined,
    tableName: () => string | null | undefined
) {
    return createQuery(() => ({
        queryKey: ['tableDetails', connectionId(), database(), schema(), tableName()] as const,
        queryFn: async () => {
            const connId = connectionId();
            const db = database();
            const sch = schema();
            const table = tableName();
            if (!connId || !db || !sch || !table) throw new Error('Missing parameters');
            return invoke<any>('get_cached_table_details', {
                connectionId: connId,
                database: db,
                schema: sch,
                tableName: table,
            });
        },
        staleTime: 30_000,
        enabled: !!connectionId() && !!database() && !!schema() && !!tableName(),
    }));
}
