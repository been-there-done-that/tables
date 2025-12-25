# Window Management & Theming Architecture Plan

## Overview

Design and implement a professional multi-window architecture with comprehensive theming system for the Tables IDE, comparable to DataGrip and other professional database tools.

## Multi-Window Architecture

### Window Types

#### Primary Windows
1. **Main Window** (`main`)
   - Database Explorer (left sidebar)
   - Connection manager
   - Global navigation
   - Menu bar and status bar

2. **Query Editor Windows** (`query-{id}`)
   - SQL editor with syntax highlighting
   - Each query gets its own window
   - Can have multiple simultaneous editors

3. **Result Windows** (`results-{id}`)
   - Table/grid data display
   - Query execution results
   - Can dock/undock

#### Secondary Windows
4. **Tool Windows** (dockable panels)
   - Database Navigator (`navigator`)
   - Files browser (`files`)
   - Services monitor (`services`)
   - Console/terminal (`console`)
   - Find & Replace (`find`)

5. **Floating Windows**
   - Quick documentation (`docs`)
   - AI Chat panel (`ai-chat`)
   - Settings dialog (`settings`)

### Window Management Strategy

#### Option A: Multiple Independent Windows (Recommended)
- Each window is a separate Tauri WebviewWindow
- Better multi-monitor support
- More native desktop feel
- Matches DataGrip's behavior exactly

#### Option B: Single Window with Virtual Panels
- One main window with internal panel splitting
- Easier state management
- Better for single-monitor workflows

**Decision: Option A** - Multi-window approach for professional IDE experience

### State Management Architecture

#### Hybrid State Strategy

```
┌─────────────────────────────────────────────────┐
│  Rust Backend (Global State)                   │
│  - Connection pools                            │
│  - Query history                               │
│  - Database schemas (cached)                   │
│  - User settings                               │
│  - Active theme                                │
└──────────────┬─────────────────────────────────┘
               │ IPC/Events
               ▼
┌─────────────────────────────────────────────────┐
│  Frontend State per Window                     │
│  - Zustand stores (per window type)            │
│  - Window-specific UI state                    │
│  - Editor state (cursor position, selection)   │
└─────────────────────────────────────────────────┘
```

#### State Synchronization
- **Global State**: Stored in Rust backend, shared across all windows
- **Window State**: Local Zustand store, synced via Tauri events
- **Event System**: `emit('state-change', data)` broadcasts, `listen()` receives

### Docking System

#### DataGrip-style Docking

```typescript
interface ToolWindowState {
  id: string;
  windowId: string;
  dockState: 'pinned' | 'unpinned' | 'float' | 'window';
  position: 'left' | 'right' | 'bottom' | 'top';
  size: { width: number; height: number };
  isVisible: boolean;
}
```

#### Implementation Plan
- Use `tauri-plugin-window-state` for position persistence
- Custom layout management for docking panels
- Drag-and-drop for repositioning
- Save/restore layouts (like DataGrip's "Layouts" feature)

## Theming System Architecture

### Three-Tier Theming System

#### Tier 1: Native/System Theme
- Tauri native window decorations
- System color scheme detection
- Platform-specific styling

#### Tier 2: Application-Level Theme
- CSS variables for complete color palette
- Dark/Light mode switching
- Custom color schemes (Monokai, Dracula, Nord, etc.)

#### Tier 3: User Customization
- User-defined color schemes
- Theme marketplace (future)
- Per-database theme settings

### Theme Definition Format

#### JSON-based Themes (VSCode-style)

```json
{
  "id": "monokai",
  "name": "Monokai",
  "author": "Tables IDE",
  "ui": {
    "background": {
      "primary": "#272822",
      "secondary": "#1e1f29",
      "tertiary": "#32343a"
    },
    "foreground": {
      "primary": "#F8F8F2",
      "secondary": "#ABB2BF"
    },
    "accent": {
      "primary": "#A6E22E",
      "hover": "#CB6A27"
    }
  },
  "syntax": {
    "keyword": "#F92672",
    "string": "#AE81FF",
    "comment": "#75715E"
  }
}
```

#### CSS Variable Application

```css
:root {
  --theme-bg-primary: var(--current-theme-bg-primary);
  --theme-fg-primary: var(--current-theme-fg-primary);
  --theme-accent: var(--current-theme-accent-primary);
}

/* Apply theme */
[data-theme="monokai"] {
  --current-theme-bg-primary: #272822;
  --current-theme-fg-primary: #F8F8F2;
  --current-theme-accent-primary: #A6E22E;
}
```

### Custom Titlebar Implementation

#### Options Comparison

| Approach | Pros | Cons |
|----------|------|------|
| **Native Titlebar** | Simple, platform integration | Limited customization |
| **Custom Titlebar** | Full control, consistent design | More complex implementation |
| **Hybrid** | Best of both worlds | Most complex |

**Decision: Custom titlebar with @tauri-controls/svelte**

```svelte
<script>
  import { Titlebar } from '@tauri-controls/svelte';
</script>

<Titlebar windowLabel="main">
  <!-- Custom content -->
</Titlebar>
```

### Layout Persistence

#### Saved Window States

```typescript
interface WindowLayout {
  mainWindow: {
    position: { x: number; y: number };
    size: { width: number; height: number };
    isMaximized: boolean;
  };
  queryWindows: Array<{
    id: string;
    position: { x: number; y: number };
    size: { width: number; height: number };
    query: string;
    databaseId: string;
  }>;
  toolWindows: {
    [key: string]: ToolWindowState;
  };
  theme: string;
  layoutName: string;
}
```

#### Named Layouts
- "Default" - Standard layout
- "SQL Only" - Query editors only
- "Debug" - Debugging focused
- "Presentation" - Clean, minimal layout

## Technology Stack for Implementation

### Required Plugins

```json
{
  "tauri": {
    "plugins": {
      "window-state": "2.0",      // Window position persistence
      "appearance": "2.0",        // Theme management
      "window": "2.0",           // Advanced window API
      "store": "2.0"             // Settings persistence
    }
  }
}
```

### Frontend Dependencies

```json
{
  "dependencies": {
    "@tauri-controls/svelte": "^0.1.0",  // Custom titlebar
    "zustand": "^4.4.0",                 // State management
    "codemirror": "^6.0.0",             // SQL editor
    "@codemirror/lang-sql": "^6.0.0"    // SQL syntax
  }
}
```

### Development Dependencies

```json
{
  "devDependencies": {
    "@types/codemirror": "^5.60.0",
    "tailwindcss": "^4.0.0",
    "autoprefixer": "^10.4.0"
  }
}
```

## Implementation Phases

### Phase 1: Core Window Management (Week 1-2)
- Set up Tauri multi-window basics
- Implement main window + query editor window
- Basic window state persistence
- Theme plugin integration

### Phase 2: Docking System (Week 3-5)
- Tool window dock/undock functionality
- Save/restore layouts
- Window positioning and sizing
- Drag-and-drop interface

### Phase 3: Advanced Theming (Week 6-8)
- Multiple color schemes
- Syntax highlighting themes
- User theme customization
- Theme persistence

### Phase 4: Polish & Performance (Week 9-10)
- Animations and transitions
- Keyboard shortcuts
- Multi-monitor optimization
- Performance profiling

## Success Criteria

### Functional Requirements
- ✅ Multiple windows can be opened simultaneously
- ✅ Windows remember position and size
- ✅ Themes apply instantly across all windows
- ✅ Professional IDE-like window management

### Performance Requirements
- ✅ < 100ms theme switching
- ✅ < 50ms window creation
- ✅ < 10MB memory overhead for theming
- ✅ Smooth 60fps animations

### User Experience Requirements
- ✅ Intuitive window management
- ✅ Consistent theming across all windows
- ✅ Professional appearance
- ✅ Keyboard-driven workflow

## Risk Mitigation

### Technical Risks

1. **Complex State Management**
   - Mitigation: Clear separation between global/window state
   - Fallback: Local storage for critical state

2. **Performance Issues**
   - Mitigation: Profile early, optimize critical paths
   - Fallback: Simplified single-window mode

3. **Cross-Platform Inconsistencies**
   - Mitigation: Extensive testing on all platforms
   - Fallback: Platform-specific code paths

### Timeline Risks

1. **Scope Creep**
   - Mitigation: Strict feature gating, phased releases
   - Fallback: MVP with essential features only

2. **Integration Complexity**
   - Mitigation: Incremental implementation
   - Fallback: Simplify to single-window approach

## Testing Strategy

### Unit Tests
- Window creation and management
- Theme validation and application
- State synchronization

### Integration Tests
- Multi-window communication
- Theme persistence across restarts
- Layout save/restore

### End-to-End Tests
- Complete user workflows
- Performance benchmarks
- Cross-platform compatibility

## Future Enhancements

### Advanced Features
- **Theme Marketplace**: Community theme sharing
- **Window Groups**: Related windows grouped together
- **Workspace Layouts**: Project-specific window arrangements
- **AI-assisted Layouts**: Automatic optimal layouts

### Ecosystem Integration
- **Plugin System**: Third-party window types
- **Theme API**: Public API for theme development
- **Layout Sharing**: Share custom layouts between users

This plan establishes a solid foundation for professional window management and theming, creating an IDE experience comparable to industry-leading database tools.</content>
<parameter name="filePath">docs/WINDOW_MANAGEMENT_PLAN.md