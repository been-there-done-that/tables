-- Level 4: Functions (and procedures/aggregates/window functions)
CREATE TABLE IF NOT EXISTS meta_functions (
    function_id      INTEGER PRIMARY KEY AUTOINCREMENT,
    schema_id        INTEGER NOT NULL REFERENCES meta_schemas(schema_id) ON DELETE CASCADE,
    connection_id    TEXT NOT NULL,
    database         TEXT NOT NULL,
    schema           TEXT NOT NULL,
    name             TEXT NOT NULL,
    oid              INTEGER NOT NULL,
    language         TEXT NOT NULL DEFAULT '',
    kind             TEXT NOT NULL CHECK(kind IN ('Function','Procedure','Aggregate','Window')),
    return_type      TEXT NOT NULL DEFAULT '',
    arguments        TEXT NOT NULL DEFAULT '[]',
    definition       TEXT NOT NULL DEFAULT '',
    security_definer INTEGER NOT NULL DEFAULT 0,
    volatility       TEXT NOT NULL DEFAULT 'volatile',
    UNIQUE(schema_id, oid)  -- oid is globally unique per pg instance; name is denormalized
);
CREATE INDEX IF NOT EXISTS idx_meta_functions_schema ON meta_functions(schema_id);
CREATE INDEX IF NOT EXISTS idx_meta_functions_lookup ON meta_functions(connection_id, database, schema);

-- Level 4: Sequences
CREATE TABLE IF NOT EXISTS meta_sequences (
    sequence_id   INTEGER PRIMARY KEY AUTOINCREMENT,
    schema_id     INTEGER NOT NULL REFERENCES meta_schemas(schema_id) ON DELETE CASCADE,
    connection_id TEXT NOT NULL,
    database      TEXT NOT NULL,
    schema        TEXT NOT NULL,
    name          TEXT NOT NULL,
    data_type     TEXT NOT NULL DEFAULT 'bigint',
    start_value   INTEGER NOT NULL DEFAULT 1,
    min_value     INTEGER NOT NULL DEFAULT 1,
    max_value     INTEGER NOT NULL DEFAULT 9223372036854775807,
    increment_by  INTEGER NOT NULL DEFAULT 1,
    cycle         INTEGER NOT NULL DEFAULT 0,
    cache_size    INTEGER NOT NULL DEFAULT 1,
    last_value    INTEGER,
    UNIQUE(schema_id, name)
);
CREATE INDEX IF NOT EXISTS idx_meta_sequences_schema ON meta_sequences(schema_id);
CREATE INDEX IF NOT EXISTS idx_meta_sequences_lookup ON meta_sequences(connection_id, database, schema);

-- Level 4: Constraints
CREATE TABLE IF NOT EXISTS meta_constraints (
    constraint_id INTEGER PRIMARY KEY AUTOINCREMENT,
    table_id      INTEGER NOT NULL REFERENCES meta_tables(table_id) ON DELETE CASCADE,
    connection_id TEXT NOT NULL,
    database      TEXT NOT NULL,
    schema        TEXT NOT NULL,
    table_name    TEXT NOT NULL,
    name          TEXT NOT NULL,
    kind          TEXT NOT NULL CHECK(kind IN ('PrimaryKey','ForeignKey','Unique','Check','Exclusion')),
    definition    TEXT NOT NULL DEFAULT '',
    columns       TEXT NOT NULL DEFAULT '[]',
    UNIQUE(table_id, name)
);
CREATE INDEX IF NOT EXISTS idx_meta_constraints_table ON meta_constraints(table_id);
CREATE INDEX IF NOT EXISTS idx_meta_constraints_lookup ON meta_constraints(connection_id, database, schema, table_name);
