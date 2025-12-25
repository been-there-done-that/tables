# Theme System Design Document

## Overview

This document outlines the design and implementation of a comprehensive theme system for the Tables IDE, a database management application built with Tauri 2, Svelte 5, and SQLite.

## Goals

- **User Experience**: Provide a seamless, professional theming experience comparable to IDEs like DataGrip, VS Code, and JetBrains IDEs
- **Extensibility**: Support built-in themes, user-created themes, and easy theme sharing
- **Performance**: Minimal impact on application startup and runtime performance
- **Persistence**: Themes persist across application restarts and can sync across devices
- **Developer Experience**: Easy to maintain, extend, and debug theme system

## Architecture Overview

### Core Components

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Frontend      │    │   Backend       │    │   Database      │
│   (Svelte 5)    │    │   (Rust/Tauri)  │    │   (SQLite)      │
│                 │    │                 │    │                 │
│ • ThemeRegistry │◄──►│ • Theme Commands│◄──►│ • themes table  │
│ • ThemeSwitcher │    │ • Migration     │    │ • Migrations    │
│ • CSS Variables │    │   Runner        │    │ • WAL mode      │
│ • Live Updates  │    │ • File I/O      │    │                 │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

### Data Flow

1. **Application Startup**: Database migrations run, themes loaded into memory
2. **Theme Switching**: User selects theme → Registry updates → CSS variables applied → UI updates
3. **Theme Persistence**: Active theme saved to database, survives restarts
4. **Theme Management**: CRUD operations for custom themes via Tauri commands

## Database Design

### Schema

```sql
-- Core themes table
CREATE TABLE themes (
  id TEXT PRIMARY KEY,                          -- Unique theme identifier
  name TEXT NOT NULL,                           -- Display name
  author TEXT,                                  -- Theme author
  description TEXT,                             -- Theme description
  theme_data TEXT NOT NULL,                     -- Full theme JSON
  is_builtin INTEGER DEFAULT 0,                 -- 1 = built-in, 0 = user
  is_active INTEGER DEFAULT 0,                  -- Currently selected theme
  created_at INTEGER NOT NULL,                  -- Creation timestamp
  updated_at INTEGER NOT NULL                   -- Last modification
);

-- Performance indexes
CREATE INDEX idx_themes_is_active ON themes(is_active);
CREATE INDEX idx_themes_is_builtin ON themes(is_builtin);
CREATE INDEX idx_themes_name ON themes(name COLLATE NOCASE);
```

### Migration Strategy

- **Embedded Migrations**: SQL files compiled into binary using `include_str!`
- **Version Control**: Sequential migration files (001_, 002_, etc.)
- **Error Handling**: Comprehensive error reporting with rollback capability
- **Idempotent**: Safe to run multiple times

### Database Location

- **Production**: User application data directory
  - macOS: `~/Library/Application Support/com.reddy.tables/tables.db`
  - Linux: `~/.local/share/com.reddy.tables/tables.db`
  - Windows: `%APPDATA%\\com.reddy.tables\\tables.db`
- **Development**: Gitignored to prevent rebuild loops

## Theme Data Structure

### Theme JSON Schema

```typescript
interface ThemeData {
  id: string;                    // Unique identifier
  name: string;                  // Display name
  author?: string;               // Theme creator
  description?: string;          // Theme description
  version?: string;              // Theme version

  ui: {                          // UI color palette
    background: ColorScale;      // Background colors
    foreground: ColorScale;      // Text colors
    accent: AccentColors;        // Interactive element colors
    border: BorderColors;        // Border colors
    scrollbar?: ScrollbarColors; // Scrollbar styling
    selection?: SelectionColors; // Text selection
    input?: InputColors;         // Form input styling
  };

  syntax: {                      // Syntax highlighting colors
    keyword: string;             // Language keywords
    string: string;              // String literals
    number: string;              // Numeric literals
    comment: string;             // Comments
    function: string;            // Function names
    variable: string;            // Variables
    operator: string;            // Operators
    type: string;                // Type names
    constant?: string;           // Constants
    property?: string;           // Object properties
    tag?: string;                // HTML/XML tags
    attribute?: string;          // Tag attributes
  };

  editor?: {                     // Code editor specific
    background?: string;         // Editor background
    foreground?: string;         // Editor text
    selection?: string;          // Editor selection
    selectionMatch?: string;     // Find matches
    lineNumber?: string;         // Line numbers
    lineNumberActive?: string;   // Active line number
    gutter?: string;             // Editor gutter
    cursor?: string;             // Cursor color
    bracket?: string;            // Matching brackets
    bracketMismatch?: string;    // Mismatched brackets
  };
}

interface ColorScale {
  primary: string;      // Main color
  secondary: string;    // Secondary color
  tertiary: string;     // Tertiary color
  hover: string;        // Hover state
  active: string;       // Active/selected state
  disabled?: string;    // Disabled state
}

interface AccentColors {
  primary: string;      // Main accent color
  hover: string;        // Hover state
  active: string;       // Active state
  subtle?: string;      // Muted accent
}

interface BorderColors {
  default: string;      // Standard borders
  subtle: string;       // Low contrast borders
  focus: string;        // Focus ring
}
```

### Built-in Themes

The application ships with 4 professionally designed themes:

1. **Monokai** - Classic dark theme with vibrant colors
2. **Dracula** - Popular dark theme with purple accents
3. **Nord** - Minimalist arctic-inspired theme
4. **Solarized Dark** - Carefully calibrated low-contrast theme

## Frontend Implementation

### Theme Registry (Svelte 5)

```typescript
class ThemeRegistry {
  // Reactive state (no Svelte runes for compatibility)
  private _themes: Map<string, Theme> = new Map();
  private _activeThemeId: string = '';
  private listeners: Array<() => void> = [];

  // Public getters
  get themes(): Map<string, Theme> { return this._themes; }
  get activeThemeId(): string { return this._activeThemeId; }
  get activeTheme(): Theme | undefined { return this._themes.get(this._activeThemeId); }

  // Computed properties
  get activeThemeData(): ThemeData | null {
    const theme = this.activeTheme;
    return theme ? JSON.parse(theme.theme_data) : null;
  }

  get allThemes(): Theme[] {
    return Array.from(this._themes.values()).sort((a, b) =>
      a.name.localeCompare(b.name));
  }

  // Reactive updates
  subscribe(callback: () => void): () => void {
    this.listeners.push(callback);
    return () => {
      this.listeners = this.listeners.filter(l => l !== callback);
    };
  }

  private notifyListeners(): void {
    this.listeners.forEach(callback => callback());
  }
}
```

### CSS Variable System

Themes are applied using CSS custom properties (variables):

```css
:root {
  /* Theme variables set by JavaScript */
  --theme-bg-primary: #ffffff;
  --theme-fg-primary: #000000;
  --theme-accent-primary: #007bff;
  /* ... more variables */
}

/* Components use semantic class names */
.bg-primary { background: var(--theme-bg-primary); }
.text-primary { color: var(--theme-fg-primary); }
.btn-primary { background: var(--theme-accent-primary); }
```

### Theme Switcher Component

A comprehensive UI for theme management:

- **Dropdown Interface**: Clean theme selection with previews
- **Search & Filter**: Find themes by name, author, or tags
- **Theme Previews**: Color swatches showing theme palette
- **Active Indicators**: Shows currently selected theme
- **Keyboard Navigation**: Full accessibility support

## Backend Implementation

### Tauri Commands

All theme operations are exposed as Tauri commands:

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

#[tauri::command]
async fn export_themes(state: State<'_, DatabaseState>) -> Result<serde_json::Value, String>

#[tauri::command]
async fn import_themes(state: State<'_, DatabaseState>, import_data: serde_json::Value) -> Result<HashMap<String, String>, String>
```

### Database State Management

Thread-safe database access using Mutex:

```rust
pub struct DatabaseState {
    pub conn: Mutex<Connection>,
}

impl DatabaseState {
    pub fn new(conn: Connection) -> Self {
        Self { conn: Mutex::new(conn) }
    }
}
```

## Performance Considerations

### Startup Performance
- **Lazy Loading**: Themes loaded once at startup, cached in memory
- **Embedded SQL**: Migrations compiled into binary, no file I/O
- **Indexed Queries**: Database indexes for fast theme lookups

### Runtime Performance
- **In-Memory Cache**: Theme registry holds all themes in memory
- **CSS Variables**: Fast CSS updates without DOM manipulation
- **Debounced Updates**: Theme changes batched to prevent thrashing

### Memory Usage
- **Shared Strings**: Theme data stored as JSON strings
- **Efficient Parsing**: JSON parsed only when needed
- **Garbage Collection**: Old theme data cleaned up automatically

## Security Considerations

### Theme Validation
- **JSON Schema**: Strict validation of theme structure
- **Color Validation**: Hex color format validation
- **Sanitization**: HTML/CSS injection prevention

### File System Security
- **User Data Directory**: Themes stored in safe, user-controlled location
- **Permission Checks**: File operations with proper error handling
- **Backup Safety**: Export/import operations are safe

## Testing Strategy

### Unit Tests
- **Theme Validation**: JSON schema compliance
- **Color Parsing**: Hex/RGB/HSL color validation
- **CSS Generation**: Variable generation accuracy

### Integration Tests
- **Database Operations**: CRUD operations with SQLite
- **Theme Switching**: End-to-end theme application
- **Persistence**: Theme survival across app restarts

### End-to-End Tests
- **UI Interactions**: Theme switcher functionality
- **Device Sync**: Export/import workflows
- **Performance**: Theme switching speed and memory usage

## Future Enhancements

### Phase 1: Core Features (Current)
- ✅ Basic theme switching
- ✅ Built-in themes
- ✅ Theme persistence
- ✅ Simple theme editor

### Phase 2: Advanced Features (Next)
- **Theme Marketplace**: Community theme sharing
- **Theme Inheritance**: Base themes with overrides
- **Advanced Editor**: Visual theme creation tool
- **Theme Analytics**: Usage statistics

### Phase 3: Ecosystem Features (Future)
- **Theme Packages**: Multiple themes in one package
- **Theme Dependencies**: Themes that extend others
- **Theme Versioning**: Semantic versioning for themes
- **Cloud Sync**: Cross-device theme synchronization

## Migration Path

### From No Themes → Theme System
1. Add theme table to existing database
2. Seed with built-in themes
3. Add theme UI components
4. Enable theme switching

### From Simple Themes → Advanced Themes
1. Extend theme schema with new fields
2. Add migration for schema updates
3. Update UI to support new features
4. Maintain backward compatibility

## Error Handling

### Database Errors
- **Connection Failures**: Graceful fallback to defaults
- **Migration Failures**: Detailed error reporting
- **Corruption**: Automatic recovery with backups

### Theme Errors
- **Invalid JSON**: Clear error messages with suggestions
- **Missing Colors**: Fallback to default values
- **Circular Dependencies**: Detection and prevention

### UI Errors
- **Theme Loading**: Fallback to safe default theme
- **Switching Failures**: Revert to previous theme
- **Persistence Errors**: Warn user but continue operation

## Accessibility

### Theme Requirements
- **High Contrast**: Minimum 4.5:1 contrast ratio
- **Color Blindness**: Support for common color vision deficiencies
- **Focus Indicators**: Clear focus rings for keyboard navigation

### UI Accessibility
- **Keyboard Navigation**: Full theme switcher keyboard support
- **Screen Readers**: Proper ARIA labels and roles
- **Color Independence**: UI works without relying on color alone

## Deployment Considerations

### Application Bundling
- **Theme Assets**: Built-in themes included in application bundle
- **Migration Scripts**: Automatic database setup on first run
- **Default Theme**: Monokai set as initial active theme

### Update Strategy
- **Backward Compatibility**: Old themes continue to work
- **Migration Safety**: Database migrations are reversible
- **Rollback Support**: Ability to revert theme changes

## Conclusion

This theme system provides a solid foundation for professional IDE theming with:

- **Scalable Architecture**: Easy to extend and maintain
- **Performance Optimized**: Minimal impact on application speed
- **User Friendly**: Intuitive theme switching and management
- **Developer Friendly**: Clean APIs and comprehensive documentation
- **Future Proof**: Designed for advanced features like marketplaces

The implementation balances complexity with usability, providing a professional theming experience while maintaining code simplicity and performance.</content>
<parameter name="filePath">docs/THEME_SYSTEM_DESIGN.md