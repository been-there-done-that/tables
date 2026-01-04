-- Migration to support robust foreign keys with multi-column support and cross-schema references.
-- Since PK changed, we must drop and recreate.

DROP TABLE IF EXISTS meta_foreign_keys;

CREATE TABLE meta_foreign_keys (
    connection_id TEXT NOT NULL,
    database TEXT NOT NULL DEFAULT 'main',
    schema TEXT NOT NULL DEFAULT 'main',
    table_name TEXT NOT NULL,
    column_name TEXT NOT NULL,
    ref_schema TEXT NOT NULL DEFAULT 'main',
    ref_table TEXT NOT NULL,
    ref_column TEXT NOT NULL,
    constraint_name TEXT NOT NULL,
    seq_no INTEGER NOT NULL DEFAULT 1,
    PRIMARY KEY (connection_id, database, schema, table_name, constraint_name, seq_no),
    FOREIGN KEY (connection_id, database, schema, table_name) REFERENCES meta_tables(connection_id, database, schema, table_name) ON DELETE CASCADE
);
