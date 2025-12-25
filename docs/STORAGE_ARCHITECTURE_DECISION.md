# Theme Storage Architecture Decision

## Problem Statement

How should we store theme data for the Tables IDE? Should we use JSON files (like VSCode/Zed) or SQLite database tables?

## Background Analysis

### VSCode Approach
- **Storage**: JSON files in extension directories
- **Active Theme**: Single string in settings.json (`"workbench.colorTheme": "theme-id"`)
- **Discovery**: Scan all extension directories for theme contributions
- **User Themes**: Must be published as extensions (complex)

### Zed Approach
- **Storage**: JSON files in `~/.config/zed/themes/`
- **Active Theme**: Theme objects in settings.json
- **Discovery**: Auto-detect files in themes directory
- **User Themes**: Drop JSON file in directory (simple)

### Our Requirements
- Device sync capability (export/import themes)
- Theme versioning and rollback
- Advanced features (favorites, ratings, marketplace)
- Professional IDE experience
- Cross-platform compatibility

## Decision Matrix

### JSON Files Approach

#### ✅ Advantages
- **Simple Implementation**: Read/write JSON files directly
- **Version Control Friendly**: Themes tracked as code
- **Easy Sharing**: Single JSON file for theme exchange
- **No Database Dependency**: Works without SQLite setup
- **Fast Startup**: No database queries on launch
- **Proven Pattern**: VSCode/Zed use this successfully

#### ❌ Disadvantages
- **Limited Querying**: Must load all files, filter in JavaScript
- **No Advanced Features**: Hard to implement:
  - Theme ratings/comments
  - User favorites
  - Download counts
  - Version history
  - Theme dependencies
- **Device Sync Complexity**: Need to sync entire directory
- **File System Issues**: Path differences across OSes, permissions
- **Search Performance**: Linear scan for large theme collections
- **Atomic Updates**: No transactions, risk of corruption

### Database Approach

#### ✅ Advantages
- **Powerful Queries**: SQL filtering, sorting, aggregation
- **Advanced Features**: Easy to add:
  - User accounts & favorites
  - Theme ratings & reviews
  - Download analytics
  - Version history with rollback
  - Theme inheritance
- **Device Sync**: Simple export/import of database tables
- **Atomic Operations**: ACID transactions prevent corruption
- **Performance**: Indexed queries scale to 1000s of themes
- **Rich Relationships**: Foreign keys, constraints, complex queries
- **Backup/Restore**: Single database file operations

#### ❌ Disadvantages
- **Database Complexity**: Schema design, migrations, connections
- **Setup Overhead**: Migration system, error handling
- **Development Effort**: More code than JSON approach
- **Query Overhead**: Database round-trips vs direct file access
- **Dependency**: Requires SQLite setup and maintenance

## Recommendation: Hybrid Database Approach

### Why Database Wins for Our Use Case

#### Critical Requirements Met
- **Device Sync**: Database export/import is trivial vs directory syncing
- **Advanced Features**: SQL queries enable marketplace features
- **Data Integrity**: ACID transactions prevent corruption
- **Scalability**: Indexes handle large theme collections
- **Rich Metadata**: Easy to add ratings, favorites, analytics

#### Our App Characteristics
- **Database IDE**: Already using SQLite for core functionality
- **Professional Tool**: Users expect advanced features
- **Cross-Platform**: Database handles OS differences automatically
- **Long-term Vision**: Marketplace potential requires database features

### Implementation Strategy

#### Phase 1: Simple Database Storage (Current)
```sql
CREATE TABLE themes (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  author TEXT,
  description TEXT,
  theme_data TEXT NOT NULL,    -- Full JSON theme object
  is_builtin INTEGER DEFAULT 0,
  is_active INTEGER DEFAULT 0,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);
```

#### Phase 2: Advanced Features (Future)
```sql
-- Version history
CREATE TABLE theme_versions (
  id INTEGER PRIMARY KEY,
  theme_id TEXT NOT NULL,
  version TEXT,
  theme_data TEXT,
  created_at INTEGER,
  FOREIGN KEY (theme_id) REFERENCES themes(id)
);

-- User favorites
CREATE TABLE user_favorites (
  user_id TEXT,
  theme_id TEXT,
  created_at INTEGER,
  PRIMARY KEY (user_id, theme_id)
);

-- Community features
CREATE TABLE theme_ratings (
  id INTEGER PRIMARY KEY,
  theme_id TEXT NOT NULL,
  user_id TEXT,
  rating INTEGER,
  review TEXT,
  created_at INTEGER
);
```

### Migration Path

#### From Database to JSON (if needed)
- Export all themes to JSON files
- Update settings to reference file paths
- Remove database dependency

#### From JSON to Database (current plan)
- Import existing JSON themes into database
- Update code to use database queries
- Add migration system for schema evolution

## Risk Assessment

### Database Approach Risks

#### High Risk - High Reward
1. **Complexity**: Database schema design and maintenance
   - **Mitigation**: Start simple, evolve incrementally
   - **Fallback**: JSON files as backup

2. **Performance**: Database queries vs direct file access
   - **Mitigation**: Profile and optimize critical paths
   - **Fallback**: In-memory caching

3. **Migration Issues**: Schema changes and data migration
   - **Mitigation**: Comprehensive testing, backup strategies
   - **Fallback**: Fresh database installation

#### Medium Risk - Medium Reward
4. **Development Time**: More code to write
   - **Mitigation**: Reuse existing patterns, comprehensive tests
   - **Fallback**: Simplified schema

5. **Learning Curve**: Team familiarity with database patterns
   - **Mitigation**: Clear documentation, code examples
   - **Fallback**: JSON approach for initial development

### JSON Approach Risks

#### Low Risk - Low Reward
1. **Feature Limitations**: Can't easily add advanced features
   - **Impact**: Major rewrite needed for marketplace
   - **Mitigation**: Plan for database migration

2. **Scalability Issues**: Performance degrades with many themes
   - **Impact**: Poor user experience with large collections
   - **Mitigation**: Database migration when needed

## Implementation Plan

### Phase 1: Core Database Storage (4 weeks)
- [x] Design simple theme table schema
- [x] Implement database migrations
- [x] Create theme CRUD operations
- [x] Build basic theme registry
- [x] Add export/import functionality

### Phase 2: Advanced Features (8 weeks)
- [ ] Theme versioning system
- [ ] User favorites management
- [ ] Theme ratings and reviews
- [ ] Download analytics
- [ ] Theme marketplace foundation

### Phase 3: Ecosystem (12 weeks)
- [ ] User accounts integration
- [ ] Community theme sharing
- [ ] Theme dependency system
- [ ] Advanced search and filtering

## Success Metrics

### Technical Success
- Database operations < 10ms average response time
- Theme switching < 100ms end-to-end
- Export/import < 5 seconds for 100 themes
- Memory usage < 50MB for theme system

### User Experience Success
- Seamless theme switching across all windows
- Reliable device sync functionality
- Intuitive theme management interface
- Fast theme discovery and installation

### Business Success
- Foundation for theme marketplace
- Competitive advantage over JSON-based competitors
- Scalable architecture for future growth
- Professional IDE experience

## Alternative: Start with JSON, Migrate Later

### Compromise Approach
1. **MVP**: JSON files for speed
2. **Migration**: Database when advanced features needed
3. **Hybrid**: Built-ins as JSON, user themes in database

### Benefits
- Faster initial development
- Proven simple approach
- Easy migration path

### Drawbacks
- Two storage systems to maintain
- Migration complexity later
- Feature limitations during MVP

## Final Decision

**Use Database Approach**

### Rationale
- Tables IDE is a database tool - using database for themes fits the product
- Advanced features are core to professional IDE experience
- Device sync is critical for user adoption
- Future marketplace potential requires database features
- Single source of truth simplifies architecture

### Implementation Timeline
- **Week 1-2**: Database schema and migrations
- **Week 3-4**: CRUD operations and theme registry
- **Week 5-6**: UI integration and testing
- **Week 7-8**: Export/import and device sync

This decision positions the theme system for long-term success while meeting immediate user needs.</content>
<parameter name="filePath">docs/STORAGE_ARCHITECTURE_DECISION.md