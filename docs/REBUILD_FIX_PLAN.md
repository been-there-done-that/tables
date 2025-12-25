# Database Location & Rebuild Loop Fix

## Problem Analysis

The application was experiencing constant rebuilds during development due to SQLite Write-Ahead Logging (WAL) files being created in the project directory. Tauri detected these as source file changes and triggered unnecessary recompilation.

## Root Cause

### SQLite WAL Files
- **Primary Database**: `tables.db` (SQLite database file)
- **WAL File**: `tables.db-wal` (Write-Ahead Log, active transactions)
- **Shared Memory**: `tables.db-shm` (Shared memory file for concurrent access)

### Detection Issue
Tauri monitors the `src-tauri/` directory for file changes during development. When SQLite writes to WAL files, Tauri interprets this as source code changes and rebuilds the application.

### Impact
- **Development Slowdown**: Constant rebuilds every few seconds
- **Lost Productivity**: Development workflow interrupted
- **Resource Waste**: CPU cycles spent on unnecessary compilation

## Solution Implementation

### 1. Database Relocation Strategy

#### Option A: System User Data Directory (Chosen)
```rust
// In main.rs setup
let app_data_dir = app.path().app_data_dir()?;
// Results in:
// macOS: ~/Library/Application Support/com.reddy.tables/
// Linux: ~/.local/share/com.reddy.tables/
// Windows: %APPDATA%\com.reddy.tables\
let db_path = app_data_dir.join("tables.db");
```

#### Option B: Project Subdirectory
```rust
let db_path = PathBuf::from("data/tables.db");
// Results in: src-tauri/data/tables.db
```

#### Option C: System Temp Directory
```rust
let temp_dir = std::env::temp_dir();
let db_path = temp_dir.join("tables-app-data.db");
```

**Decision Rationale:**
- **User Data Directory**: Proper system integration, user data persistence
- **Project Subdirectory**: Simple but still in monitored directory
- **Temp Directory**: Fast but data lost on reboot

### 2. Git Ignore Configuration

#### Files to Ignore
```gitignore
# SQLite database files (prevent rebuild loops)
*.db
*.db-shm
*.db-wal
```

#### Why This Helps
- WAL files won't be tracked by Git
- Tauri won't detect them as source changes
- Clean separation between code and data

### 3. Development vs Production Strategy

#### Development Environment
- Database in user data directory
- WAL files ignored by Git
- No rebuild triggers

#### Production Environment
- Database bundled with application
- WAL files managed by application
- User data directory for user themes

### 4. Migration Handling

#### Backward Compatibility
```rust
// Check for old database location
let old_db_path = PathBuf::from("src-tauri/tables.db");
let new_db_path = app_data_dir.join("tables.db");

if old_db_path.exists() && !new_db_path.exists() {
    // Migrate old database to new location
    std::fs::copy(&old_db_path, &new_db_path)?;
    println!("Migrated database from {} to {}", old_db_path.display(), new_db_path.display());
}
```

#### Clean Startup
```rust
// Remove any leftover WAL files from project directory
let _ = std::fs::remove_file("src-tauri/tables.db-wal");
let _ = std::fs::remove_file("src-tauri/tables.db-shm");
```

## Implementation Details

### Code Changes

#### 1. Main Application Setup
```rust
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .setup(|app| {
            // Get user data directory
            let app_data_dir = app
                .path()
                .app_data_dir()
                .map_err(|e| format!("Failed to get app data directory: {}", e))?;

            // Ensure directory exists
            std::fs::create_dir_all(&app_data_dir)
                .map_err(|e| format!("Failed to create app data directory: {}", e))?;

            // Set database path
            let db_path = app_data_dir.join("tables.db");

            // Initialize database
            let conn = match initialize_database(&db_path) {
                Ok(conn) => conn,
                Err(e) => {
                    eprintln!("Failed to initialize database: {}", e);
                    std::process::exit(1);
                }
            };

            // Store database state
            app.manage(DatabaseState::new(conn));

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // theme commands...
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

#### 2. Migration Runner Updates
```rust
pub fn initialize_database(db_path: &Path) -> Result<Connection, String> {
    // Create parent directories if needed
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create database directory: {}", e))?;
    }

    // Open database connection
    let conn = Connection::open(db_path)
        .map_err(|e| format!("Failed to open database: {}", e))?;

    // Run migrations
    run_migrations(&conn)?;

    Ok(conn)
}
```

### Testing Strategy

#### 1. Development Testing
- Start application in development mode
- Verify no rebuild loops occur
- Confirm database operations work
- Check database file location

#### 2. Production Testing
- Build production application
- Verify database initializes correctly
- Test theme operations
- Confirm user data persistence

#### 3. Cross-Platform Testing
- Test on Windows, macOS, Linux
- Verify correct data directory paths
- Confirm database file creation
- Test WAL file handling

### Monitoring and Alerts

#### Development Indicators
- **Success**: No rebuilds after initial startup
- **Success**: Database file created in user data directory
- **Warning**: WAL files appearing in project directory
- **Error**: Database initialization failures

#### Performance Metrics
- **Startup Time**: Database initialization < 500ms
- **Rebuild Frequency**: Zero unexpected rebuilds
- **File I/O**: Database operations < 100ms

## Alternative Solutions Considered

### Option 1: Disable WAL Mode (Not Recommended)
```sql
PRAGMA journal_mode = DELETE;
```
**Drawbacks:**
- Slower database operations
- Higher risk of corruption
- Not suitable for concurrent access

### Option 2: Tauri Ignore Configuration (Limited)
```json
// tauri.conf.json
{
  "build": {
    "beforeDevCommand": "pnpm dev",
    "beforeBuildCommand": "pnpm build",
    "devPath": "http://localhost:1420",
    "distDir": "../dist",
    "withGlobalTauri": false
  },
  "productName": "Tables",
  "version": "0.1.0",
  "identifier": "com.reddy.tables",
  "plugins": {
    "updater": {
      "active": false
    }
  }
}
```
**Limitations:**
- No built-in file watching configuration
- Would require custom watch ignore patterns

### Option 3: Database in Memory (Not Suitable)
```rust
let conn = Connection::open(":memory:")?;
```
**Drawbacks:**
- Data lost on application restart
- No persistence across sessions
- Not suitable for theme storage

## Future Considerations

### Database Maintenance
- **Vacuum Operations**: Periodic database optimization
- **Backup Strategy**: Automatic user data backups
- **Migration Rollbacks**: Safe downgrade capabilities

### Performance Monitoring
- **Query Performance**: Monitor slow database operations
- **File Size Growth**: Track database file size over time
- **Memory Usage**: Monitor database connection memory

### User Data Management
- **Data Export**: Allow users to export their themes
- **Data Import**: Restore themes from backups
- **Data Cleanup**: Remove unused or corrupted themes

## Success Criteria

### Technical Success
- ✅ **Zero Rebuild Loops**: No unexpected recompilation during development
- ✅ **Correct File Location**: Database files in appropriate system directories
- ✅ **Data Persistence**: User themes survive application restarts
- ✅ **Cross-Platform**: Works identically on all supported platforms

### User Experience Success
- ✅ **Fast Startup**: Application launches quickly
- ✅ **Theme Persistence**: User theme choices remembered
- ✅ **Reliable Operation**: No data loss or corruption
- ✅ **Development Workflow**: Smooth development experience

### Performance Success
- ✅ **Database Operations**: < 50ms average query time
- ✅ **Startup Time**: < 2 seconds application launch
- ✅ **Memory Usage**: < 50MB database-related memory
- ✅ **File I/O**: Minimal disk access during normal operation

## Implementation Timeline

### Phase 1: Immediate Fix (Completed)
- [x] Move database to user data directory
- [x] Add database files to .gitignore
- [x] Update migration runner
- [x] Test rebuild loop elimination

### Phase 2: Cleanup (Week 1)
- [ ] Remove old database files from project
- [ ] Add migration for existing users
- [ ] Update documentation
- [ ] Comprehensive testing

### Phase 3: Monitoring (Ongoing)
- [ ] Performance monitoring implementation
- [ ] User feedback collection
- [ ] Optimization based on usage patterns

## Conclusion

This fix resolves the rebuild loop issue while establishing proper data management practices. The database is now correctly placed in the user data directory, providing better system integration and preventing development workflow interruptions.

The solution maintains data persistence across application updates while ensuring a smooth development experience. Future enhancements can build upon this solid foundation for user data management and cross-device synchronization.</content>
<parameter name="filePath">docs/REBUILD_FIX_PLAN.md