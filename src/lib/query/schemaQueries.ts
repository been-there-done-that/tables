/**
 * Schema Types
 * 
 * Type definitions for database metadata.
 * These match the backend Rust types from introspection.
 */

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
