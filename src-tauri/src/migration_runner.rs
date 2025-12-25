use rusqlite::Connection;
use std::path::Path;

/// Embedded migrations - these will be included in the binary at compile time
const MIGRATION_001: &str = include_str!("../migrations/001_create_themes_table.sql");
const MIGRATION_002: &str = include_str!("../migrations/002_seed_builtin_themes.sql");

/// List of migrations in order
const MIGRATIONS: &[(&str, &str)] = &[
    ("001_create_themes_table.sql", MIGRATION_001),
    ("002_seed_builtin_themes.sql", MIGRATION_002),
];

/// Run all database migrations
pub fn run_migrations(conn: &Connection) -> Result<(), String> {
    println!("Running database migrations...");

    // Enable WAL mode for better performance and reliability
    conn.pragma_update(None, "journal_mode", "WAL")
        .map_err(|e| format!("Failed to enable WAL mode: {}", e))?;

    // Enable foreign keys
    conn.pragma_update(None, "foreign_keys", "ON")
        .map_err(|e| format!("Failed to enable foreign keys: {}", e))?;

    // Run each migration
    for (migration_name, migration_sql) in MIGRATIONS {
        println!("Running migration: {}", migration_name);

        // Execute the migration
        conn.execute_batch(migration_sql)
            .map_err(|e| format!("Migration {} failed: {}", migration_name, e))?;

        println!("Migration {} completed", migration_name);
    }

    println!("All migrations completed successfully");
    Ok(())
}

/// Initialize database and run migrations if needed
pub fn initialize_database(db_path: &Path) -> Result<Connection, String> {
    // Create database directory if it doesn't exist
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create database directory: {}", e))?;
    }

    // Open database connection
    let conn = Connection::open(db_path)
        .map_err(|e| format!("Failed to open database: {}", e))?;

    // Run migrations on the connection
    run_migrations(&conn)?;

    Ok(conn)
}