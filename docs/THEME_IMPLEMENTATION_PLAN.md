# Theme System Implementation Plan

## Overview

Detailed implementation plan for a comprehensive SQLite-based theme system with device sync capabilities, built-in themes, and extensible architecture for future marketplace features.

## Database Schema Design

### Core Tables

#### Themes Table
```sql
CREATE TABLE themes (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  author TEXT,
  description TEXT,
  theme_data TEXT NOT NULL,
  is_builtin INTEGER DEFAULT 0,
  is_active INTEGER DEFAULT 0,
  created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
  updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

CREATE INDEX idx_themes_is_active ON themes(is_active);
CREATE INDEX idx_themes_is_builtin ON themes(is_builtin);
CREATE INDEX idx_themes_name ON themes(name COLLATE NOCASE);
```

#### Future Tables (Phase 2+)
```sql
-- Version history for rollbacks
CREATE TABLE theme_versions (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  theme_id TEXT NOT NULL,
  version TEXT NOT NULL,
  theme_data TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  FOREIGN KEY (theme_id) REFERENCES themes(id) ON DELETE CASCADE
);

-- User favorites
CREATE TABLE user_favorites (
  user_id TEXT,
  theme_id TEXT NOT NULL,
  created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
  PRIMARY KEY (user_id, theme_id),
  FOREIGN KEY (theme_id) REFERENCES themes(id) ON DELETE CASCADE
);

-- Community features
CREATE TABLE theme_ratings (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  theme_id TEXT NOT NULL,
  user_id TEXT,
  rating INTEGER CHECK (rating >= 1 AND rating <= 5),
  review TEXT,
  created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
  FOREIGN KEY (theme_id) REFERENCES themes(id) ON DELETE CASCADE
);
```

## Migration System

### Embedded Migrations Strategy
- **Location**: `src-tauri/migrations/` directory
- **Format**: `001_description.sql`, `002_description.sql`, etc.
- **Embedding**: `include_str!()` macro compiles SQL into binary
- **Execution**: Run on application startup, skip if already applied

### Migration Files
1. **001_create_themes_table.sql**: Basic theme table
2. **002_seed_builtin_themes.sql**: Insert Monokai, Dracula, Nord, Solarized
3. **003_add_preview_images.sql**: Theme screenshot support
4. **004_add_theme_tags.sql**: Categorization system

### Migration Runner Implementation
```rust
pub fn run_migrations(conn: &Connection) -> Result<(), String> {
    // Enable WAL mode and foreign keys
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;

    // Execute embedded migrations
    for (name, sql) in MIGRATIONS {
        println!("Running migration: {}", name);
        conn.execute_batch(sql)?;
    }

    Ok(())
}
```

## Built-in Themes

### Theme Collection
- **Monokai**: Colorful dark theme, professional coding feel
- **Dracula**: Purple-accented dark theme, popular and accessible
- **Nord**: Cool blue arctic theme, minimal and clean
- **Solarized Dark**: Carefully calibrated theme, low eye strain

### Theme JSON Structure
```json
{
  "id": "monokai",
  "name": "Monokai",
  "author": "Tables IDE",
  "description": "Colorful dark theme inspired by Monokai",
  "version": "1.0.0",
  "ui": {
    "background": {
      "primary": "#272822",
      "secondary": "#1e1f29",
      "tertiary": "#18191c",
      "hover": "#2c2e35",
      "active": "#32343f"
    },
    "foreground": {
      "primary": "#F8F8F2",
      "secondary": "#ABB2BF",
      "tertiary": "#6D728E"
    },
    "accent": {
      "primary": "#A6E22E",
      "hover": "#CB6A27",
      "active": "#FD971F"
    },
    "border": {
      "default": "#3B3F4D",
      "subtle": "#2C2F36",
      "focus": "#A6E22E"
    }
  },
  "syntax": {
    "keyword": "#F92672",
    "string": "#AE81FF",
    "number": "#AE81FF",
    "comment": "#75715E",
    "function": "#66D9EF",
    "variable": "#FD971F",
    "operator": "#F92672",
    "type": "#66D9EF"
  },
  "editor": {
    "background": "#272822",
    "foreground": "#F8F8F2",
    "selection": "#3E4451",
    "cursor": "#F8F8F2",
    "bracket": "#A6E22E"
  }
}
```

## Backend Implementation

### Tauri Commands

#### Theme Management
```rust
#[tauri::command]
async fn get_all_themes(state: State<'_, DatabaseState>) -> Result<Vec<Theme>, String>

#[tauri::command]
async fn get_active_theme(state: State<'_, DatabaseState>) -> Result<Option<Theme>, String>

#[tauri::command]
async fn set_active_theme(state: State<'_, DatabaseState>, theme_id: String) -> Result<(), String>

#[tauri::command]
async fn create_theme(state: State<'_, DatabaseState>, theme_data: String) -> Result<Theme, String>

#[tauri::command]
async fn update_theme(state: State<'_, DatabaseState>, theme_id: String, theme_data: String) -> Result<Theme, String>

#[tauri::command]
async fn delete_theme(state: State<'_, DatabaseState>, theme_id: String) -> Result<(), String>

#[tauri::command]
async fn search_themes(state: State<'_, DatabaseState>, query: String) -> Result<Vec<Theme>, String>
```

#### Device Sync
```rust
#[tauri::command]
async fn export_themes(state: State<'_, DatabaseState>) -> Result<serde_json::Value, String>

#[tauri::command]
async fn import_themes(state: State<'_, DatabaseState>, import_data: serde_json::Value) -> Result<HashMap<String, String>, String>
```

### Database State Management

#### Thread-Safe Access
```rust
pub struct DatabaseState {
    pub conn: Mutex<Connection>,
}

impl DatabaseState {
    pub fn new(conn: Connection) -> Self {
        Self { conn: Mutex::new(conn) }
    }
}

// Usage in commands
let conn = state.conn.lock().map_err(|e| format!("Failed to lock database: {}", e))?;
let conn = &*conn;
```

### Error Handling Strategy

#### Database Errors
- **Connection Failures**: Graceful degradation with cached themes
- **Migration Failures**: Detailed logging, rollback options
- **Corruption**: Automatic recovery with backup restoration

#### Theme Validation
- **JSON Parsing**: Strict validation with error messages
- **Color Formats**: Hex, RGB, HSL validation
- **Required Fields**: Schema compliance checking

## Frontend Implementation

### Theme Registry (Svelte 5)

#### Reactive State Management
```typescript
class ThemeRegistry {
  private _themes: Map<string, Theme> = new Map();
  private _activeThemeId: string = '';
  private listeners: Array<() => void> = [];

  // Reactive getters
  get themes(): Map<string, Theme> { return this._themes; }
  get activeThemeId(): string { return this._activeThemeId; }
  get activeTheme(): Theme | undefined { return this._themes.get(this._activeThemeId); }
  get activeThemeData() { /* parsed theme data */ }
  get allThemes(): Theme[] { /* sorted array */ }
  get builtinThemes(): Theme[] { /* filtered */ }
  get userThemes(): Theme[] { /* filtered */ }

  // Reactive updates
  subscribe(callback: () => void) { /* listener management */ }
  private notifyListeners() { this.listeners.forEach(cb => cb()); }

  // Database operations
  async init(): Promise<void> { /* load from DB */ }
  async setTheme(themeId: string): Promise<void> { /* update DB */ }
  async createTheme(themeData: string): Promise<Theme> { /* insert */ }
  async updateTheme(themeId: string, themeData: string): Promise<Theme> { /* update */ }
  async deleteTheme(themeId: string): Promise<void> { /* delete */ }
  async searchThemes(query: string): Promise<Theme[]> { /* search */ }
}
```

### CSS Variable System

#### Theme Application
```css
/* CSS Variables for theming */
:root {
  /* Theme variables set by JavaScript */
  --theme-bg-primary: #ffffff;
  --theme-fg-primary: #000000;
  --theme-accent-primary: #007bff;
}

/* Semantic class usage */
.bg-primary { background: var(--theme-bg-primary); }
.text-primary { color: var(--theme-fg-primary); }
.btn-primary { background: var(--theme-accent-primary); }
```

#### Dynamic Application
```typescript
export function applyThemeToDOM(themeData: ThemeData): void {
  const cssVars = generateCSSVariables(themeData);
  const root = document.documentElement;

  // Remove old theme
  root.removeAttribute('data-theme');

  // Apply CSS variables
  root.style.cssText = cssVars;

  // Set theme attribute
  root.setAttribute('data-theme', themeData.id);
}
```

### Theme Switcher Component

#### Core Features
- **Dropdown Interface**: Clean theme selection with search
- **Theme Previews**: Color swatches showing theme palette
- **Search & Filter**: Find themes by name or category
- **Active Indicators**: Visual feedback for current theme
- **Keyboard Navigation**: Full accessibility support

#### UI Implementation
```svelte
<script lang="ts">
  import { themeRegistry } from '$lib/themes/registry';

  let isOpen = $state(false);
  let searchQuery = $state('');
  let showBuiltIn = $state(true);
  let showCustom = $state(true);

  // Reactive filtered themes
  const filteredThemes = $derived(() => {
    return themeRegistry.allThemes.filter(theme => {
      const matchesSearch = theme.name.toLowerCase().includes(searchQuery.toLowerCase());
      const matchesCategory = (showBuiltIn && theme.is_builtin) || (showCustom && !theme.is_builtin);
      return matchesSearch && matchesCategory;
    });
  });

  async function handleThemeSelect(theme: Theme) {
    await themeRegistry.setTheme(theme.id);
    isOpen = false;
  }
</script>

<!-- Theme selector UI -->
```

## Device Sync Implementation

### Export Strategy
```typescript
async exportThemes(): Promise<ThemeExport> {
  const userThemes = this.allThemes.filter(t => !t.is_builtin);
  return {
    version: "1.0",
    exported_at: new Date().toISOString(),
    themes: userThemes.map(t => ({
      id: t.id,
      name: t.name,
      author: t.author,
      description: t.description,
      theme_data: t.theme_data
    }))
  };
}
```

### Import Strategy
```typescript
async importThemes(importData: ThemeImport): Promise<ImportResult> {
  const results = new Map<string, string>();

  for (const themeData of importData.themes) {
    try {
      // Validate and import theme
      await this.createTheme(JSON.stringify(themeData));
      results.set(themeData.id, 'success');
    } catch (error) {
      results.set(themeData.id, `error: ${error}`);
    }
  }

  // Reload registry
  await this.init();

  return {
    imported: [...results.values()].filter(r => r === 'success').length,
    failed: [...results.values()].filter(r => r !== 'success').length,
    total: results.size
  };
}
```

### Sync Workflow
1. **Export**: Serialize user themes to JSON
2. **Transfer**: Copy JSON file across devices
3. **Import**: Parse and insert themes into local database
4. **Resolve**: Handle conflicts with merge strategies

## Performance Optimization

### Startup Performance
- **Lazy Loading**: Themes loaded once at startup
- **In-Memory Cache**: Registry holds all themes in memory
- **Indexed Queries**: Database indexes for fast lookups

### Runtime Performance
- **Debounced Updates**: Theme changes batched to prevent thrashing
- **CSS Variables**: Fast style updates without DOM manipulation
- **Minimal Re-renders**: Reactive system prevents unnecessary updates

### Memory Management
- **Shared Strings**: Theme data stored efficiently
- **Garbage Collection**: Old theme data cleaned up
- **Memory Limits**: Reasonable caps on theme collection size

## Testing Strategy

### Unit Tests
- **Theme Validation**: JSON schema compliance
- **CSS Generation**: Variable generation accuracy
- **Registry Operations**: CRUD functionality

### Integration Tests
- **Database Operations**: SQLite interaction
- **Theme Switching**: End-to-end theme application
- **Sync Operations**: Export/import workflows

### End-to-End Tests
- **UI Interactions**: Theme switcher functionality
- **Persistence**: Themes survive app restarts
- **Performance**: Theme switching speed benchmarks

## Error Handling

### Database Errors
- **Connection Issues**: Fallback to cached/default themes
- **Migration Failures**: Clear error messages, recovery options
- **Constraint Violations**: Validation before database operations

### Theme Errors
- **Invalid JSON**: Detailed parsing error messages
- **Missing Colors**: Automatic fallback to defaults
- **Corrupt Themes**: Graceful skipping with warnings

### UI Errors
- **Loading Failures**: Fallback to safe default theme
- **Switch Failures**: Revert to previous theme
- **Sync Errors**: Non-blocking with user notifications

## Future Enhancements

### Phase 2: Advanced Features
- **Theme Editor**: Visual theme creation tool
- **Version History**: Rollback to previous theme versions
- **Favorites System**: User theme bookmarks
- **Import/Export**: Enhanced sync capabilities

### Phase 3: Community Features
- **Theme Marketplace**: Community theme sharing
- **Ratings & Reviews**: User feedback system
- **Theme Analytics**: Download and usage statistics
- **Theme Dependencies**: Composite theme support

### Phase 4: Ecosystem
- **Plugin API**: Third-party theme providers
- **Theme Packages**: Bundled theme collections
- **Cloud Sync**: Automatic cross-device synchronization
- **Theme Inheritance**: Base themes with customizations

## Deployment Strategy

### Application Bundling
- **Built-in Themes**: Included in application package
- **Migration Scripts**: Automatic database setup
- **Default Theme**: Monokai set as initial active theme

### Update Strategy
- **Backward Compatibility**: Schema migrations handle upgrades
- **Theme Preservation**: User themes survive updates
- **Rollback Support**: Version history for recovery

## Success Metrics

### Performance Targets
- **Startup Time**: < 2 seconds theme system initialization
- **Theme Switching**: < 100ms end-to-end
- **Memory Usage**: < 10MB for theme system
- **Database Queries**: < 50ms average response time

### User Experience Targets
- **Theme Discovery**: < 500ms search response time
- **Visual Feedback**: Instant theme preview on hover
- **Persistence**: 100% theme survival across restarts
- **Sync Reliability**: > 99% successful theme imports

### Code Quality Targets
- **Test Coverage**: > 90% for theme system
- **Error Rate**: < 0.1% theme operation failures
- **Bundle Size**: < 200KB additional for theme system
- **Type Safety**: 100% TypeScript coverage

## Implementation Timeline

### Phase 1: Core Implementation (Week 1-3)
- [x] Database schema and migrations
- [x] Built-in themes creation
- [x] Rust Tauri commands
- [x] Basic theme registry
- [x] CSS variable system
- [x] Theme switcher UI

### Phase 2: Advanced Features (Week 4-6)
- [ ] Device sync (export/import)
- [ ] Theme validation
- [ ] Error handling
- [ ] Performance optimization
- [ ] Comprehensive testing

### Phase 3: Polish & Documentation (Week 7-8)
- [ ] UI/UX improvements
- [ ] Accessibility compliance
- [ ] Documentation completion
- [ ] Performance monitoring

This implementation plan provides a solid foundation for a professional theme system with room for future marketplace and community features.</content>
<parameter name="filePath">docs/THEME_IMPLEMENTATION_PLAN.md