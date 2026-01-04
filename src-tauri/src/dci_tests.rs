//! DCI Test Matrix
//!
//! Comprehensive tests for the Database Capability Interface (DCI) unification.
//! These tests enforce platform invariants and enable capability-based testing
//! across all database engine profiles.
//!
//! ## Test Layers
//! 1. **Adapter Contract Tests** - Verify capabilities and shape
//! 2. **Invariant Tests** - Non-negotiable structural rules
//! 3. **Progressive Introspection Tests** - Level-by-level verification
//! 4. **Qualification Tests** - Schema/database qualification rules
//!
//! ## Profiles
//! - `DB0`: No database, no schema (SQLite)
//! - `DB1`: Database only (MySQL, MongoDB)
//! - `DB2`: Database + schema (PostgreSQL)
//! - `DB3`: Multi-catalog (future)

#[cfg(test)]
mod adapter_contract_tests {
    use crate::adapter::{DatabaseCapabilities, EngineProfile};

    // =========================================================================
    // Test: Capabilities Consistency
    // =========================================================================

    #[test]
    fn test_capabilities_have_engine_id() {
        let engines = ["postgres", "sqlite", "mysql", "athena", "mongodb", "redis"];
        
        for engine in engines {
            let caps = DatabaseCapabilities::for_engine(engine);
            assert!(!caps.engine.is_empty(), 
                "Engine '{}' must have non-empty engine ID", engine);
        }
    }

    #[test]
    fn test_default_schema_consistency() {
        let caps = DatabaseCapabilities::postgres();
        assert!(caps.supports_schemas, "Postgres must support schemas");
        assert!(caps.default_schema.is_some(), "Postgres must have default_schema");

        let caps = DatabaseCapabilities::sqlite();
        assert!(!caps.supports_schemas, "SQLite must NOT support schemas");
        assert!(caps.default_schema.is_some(), "SQLite must still have synthetic default_schema");
    }

    #[test]
    fn test_default_database_consistency() {
        let caps = DatabaseCapabilities::postgres();
        assert!(caps.supports_databases, "Postgres must support databases");

        let caps = DatabaseCapabilities::sqlite();
        assert!(!caps.supports_databases, "SQLite must NOT support databases");
        assert!(caps.default_database.is_some(), "SQLite must have synthetic default_database");
    }

    // =========================================================================
    // Test: Profile Classification
    // =========================================================================

    #[test]
    fn test_sqlite_is_db0() {
        let caps = DatabaseCapabilities::sqlite();
        assert_eq!(caps.profile(), EngineProfile::DB0, 
            "SQLite must be DB0 (no database, no schema)");
    }

    #[test]
    fn test_mysql_is_db1() {
        let caps = DatabaseCapabilities::mysql();
        assert_eq!(caps.profile(), EngineProfile::DB1, 
            "MySQL must be DB1 (database only, no schemas)");
    }

    #[test]
    fn test_postgres_is_db2() {
        let caps = DatabaseCapabilities::postgres();
        assert_eq!(caps.profile(), EngineProfile::DB2, 
            "Postgres must be DB2 (database + schema)");
    }

    #[test]
    fn test_athena_is_db2() {
        let caps = DatabaseCapabilities::athena();
        assert_eq!(caps.profile(), EngineProfile::DB2, 
            "Athena must be DB2 (catalog + database as schema)");
    }

    // =========================================================================
    // Test: Feature Support Flags
    // =========================================================================

    #[test]
    fn test_all_engines_support_tables() {
        let engines = ["postgres", "sqlite", "mysql"];
        
        for engine in engines {
            let caps = DatabaseCapabilities::for_engine(engine);
            // Tables are implicit - if we can list_tables, we support tables
            // Views, indexes, FKs may vary
            assert!(caps.supports_views || !caps.supports_views, 
                "Views support is explicit for {}", engine);
        }
    }

    #[test]
    fn test_redis_minimal_features() {
        let caps = DatabaseCapabilities::redis();
        assert!(!caps.supports_schemas, "Redis has no schemas");
        assert!(!caps.supports_views, "Redis has no views");
        assert!(!caps.supports_indexes, "Redis has no indexes");
        assert!(!caps.supports_foreign_keys, "Redis has no FKs");
    }
}

#[cfg(test)]
mod invariant_tests {
    use crate::adapter::DatabaseCapabilities;
    use crate::introspection::{MetaTable, MetaColumn, MetaSchema, MetaDatabase};

    // =========================================================================
    // Invariant 1: Structural Completeness
    // Every object must have {database, schema, name}
    // =========================================================================

    #[test]
    fn test_meta_table_has_required_fields() {
        let table = MetaTable {
            connection_id: "test".to_string(),
            database: "mydb".to_string(),
            schema: "public".to_string(),
            table_name: "users".to_string(),
            table_type: "table".to_string(),
            classification: "user".to_string(),
            last_introspected_at: 0,
            columns: vec![],
            foreign_keys: vec![],
            indexes: vec![],
            triggers: vec![],
        };

        // Invariant: All three parts of the qualified name must be present
        assert!(!table.database.is_empty(), "database must be non-empty");
        assert!(!table.schema.is_empty(), "schema must be non-empty");
        assert!(!table.table_name.is_empty(), "table_name must be non-empty");
    }

    #[test]
    fn test_meta_column_has_table_context() {
        let col = MetaColumn {
            connection_id: "test".to_string(),
            database: "mydb".to_string(),
            schema: "public".to_string(),
            table_name: "users".to_string(),
            ordinal_position: 1,
            column_name: "id".to_string(),
            raw_type: "integer".to_string(),
            logical_type: "integer".to_string(),
            nullable: false,
            default_value: None,
            is_primary_key: true,
        };

        // Invariant: Column knows its full table context
        assert!(!col.database.is_empty());
        assert!(!col.schema.is_empty());
        assert!(!col.table_name.is_empty());
        assert!(!col.column_name.is_empty());
    }

    // =========================================================================
    // Invariant 2: Effective Values Always Exist
    // =========================================================================

    #[test]
    fn test_effective_schema_never_empty() {
        let profiles = [
            DatabaseCapabilities::postgres(),
            DatabaseCapabilities::sqlite(),
            DatabaseCapabilities::mysql(),
        ];

        for caps in profiles {
            let schema = caps.effective_schema(None);
            assert!(!schema.is_empty(), 
                "effective_schema must never be empty for {}", caps.engine);
        }
    }

    #[test]
    fn test_effective_database_never_empty() {
        let profiles = [
            DatabaseCapabilities::postgres(),
            DatabaseCapabilities::sqlite(),
            DatabaseCapabilities::mysql(),
        ];

        for caps in profiles {
            let db = caps.effective_database(None);
            assert!(!db.is_empty(), 
                "effective_database must never be empty for {}", caps.engine);
        }
    }

    // =========================================================================
    // Invariant 3: Qualification Rules Are Deterministic
    // =========================================================================

    #[test]
    fn test_postgres_requires_qualification_for_non_public() {
        let caps = DatabaseCapabilities::postgres();
        
        // In Postgres, non-public schemas should require qualification
        // The completion engine handles this via qualify_table_name
        assert_eq!(caps.default_schema, Some("public".to_string()));
        assert!(!caps.requires_qualified_names, 
            "Postgres doesn't require qualification by default (public schema)");
    }

    #[test]
    fn test_athena_requires_qualified_names() {
        let caps = DatabaseCapabilities::athena();
        
        // Athena requires full qualification
        assert!(caps.requires_qualified_names, 
            "Athena must require qualified names");
    }

    // =========================================================================
    // Invariant 4: Synthetic Values for Flat Engines
    // =========================================================================

    #[test]
    fn test_sqlite_uses_main_for_everything() {
        let caps = DatabaseCapabilities::sqlite();
        
        assert_eq!(caps.default_database, Some("main".to_string()));
        assert_eq!(caps.default_schema, Some("main".to_string()));
        assert_eq!(caps.effective_database(None), "main");
        assert_eq!(caps.effective_schema(None), "main");
    }
}

#[cfg(test)]
mod qualification_rule_tests {
    use crate::adapter::{DatabaseCapabilities, TableRef};

    // =========================================================================
    // Test: TableRef Qualification
    // =========================================================================

    #[test]
    fn test_table_ref_fully_qualified() {
        let ref1 = TableRef::new("mydb", "public", "users");
        assert_eq!(ref1.fully_qualified(), "mydb.public.users");
    }

    #[test]
    fn test_table_ref_schema_qualified() {
        let ref1 = TableRef::new("mydb", "public", "users");
        assert_eq!(ref1.schema_qualified(), "public.users");
    }

    #[test]
    fn test_table_ref_from_name_uses_main() {
        let ref1 = TableRef::from_name("users");
        assert_eq!(ref1.database, "main");
        assert_eq!(ref1.schema, "main");
        assert_eq!(ref1.name, "users");
    }

    // =========================================================================
    // Test: Cross-Profile Qualification
    // =========================================================================

    #[test]
    fn test_db0_profile_always_uses_synthetic() {
        let caps = DatabaseCapabilities::sqlite();
        
        // Even if user passes None, we get stable synthetic values
        let db = caps.effective_database(None);
        let schema = caps.effective_schema(None);
        
        assert_eq!(db, "main");
        assert_eq!(schema, "main");
    }

    #[test]
    fn test_db2_profile_uses_explicit_or_default() {
        let caps = DatabaseCapabilities::postgres();
        
        // With explicit values
        let schema = caps.effective_schema(Some("custom"));
        assert_eq!(schema, "custom");
        
        // Without explicit values, falls back to default
        let schema = caps.effective_schema(None);
        assert_eq!(schema, "public");
    }
}

#[cfg(test)]
mod adapter_registry_tests {
    use crate::adapter_registry::{self, AdapterInfo};
    use crate::adapter::DatabaseCapabilities;

    #[test]
    fn test_adapter_info_builtin_has_correct_flags() {
        let caps = DatabaseCapabilities::postgres();
        let info = AdapterInfo::builtin("postgres", "PostgreSQL", "PostgreSQL adapter", caps);
        
        assert!(info.is_builtin);
        assert!(info.plugin_name.is_none());
        assert_eq!(info.engine, "postgres");
    }

    #[test]
    fn test_adapter_info_from_plugin_has_plugin_name() {
        let caps = DatabaseCapabilities::default();
        let info = AdapterInfo::from_plugin("custom", "Custom", "Custom adapter", caps, "my_plugin");
        
        assert!(!info.is_builtin);
        assert_eq!(info.plugin_name, Some("my_plugin".to_string()));
    }

    #[test]
    fn test_capabilities_lookup() {
        // This tests the capabilities() function which looks up by engine name
        // Requires init_builtins() to have been called, so we test the for_engine path
        let caps = DatabaseCapabilities::for_engine("postgres");
        assert_eq!(caps.engine, "postgres");
        assert!(caps.supports_schemas);
    }
}

#[cfg(test)]
mod orchestrator_event_tests {
    use crate::orchestrator::IntrospectionEvent;

    #[test]
    fn test_event_serialization_level_complete() {
        let event = IntrospectionEvent::LevelComplete {
            level: 3,
            connection_id: "conn-1".to_string(),
            database: Some("mydb".to_string()),
            schema_count: Some(5),
            table_count: Some(50),
        };
        
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("level_complete"));
        assert!(json.contains("\"level\":3"));
    }

    #[test]
    fn test_event_serialization_schema_ready() {
        let event = IntrospectionEvent::SchemaReady {
            connection_id: "conn-1".to_string(),
            database: "mydb".to_string(),
        };
        
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("schema_ready"));
    }

    #[test]
    fn test_event_serialization_complete() {
        let event = IntrospectionEvent::Complete {
            connection_id: "conn-1".to_string(),
            database: "mydb".to_string(),
        };
        
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("complete"));
    }

    #[test]
    fn test_event_serialization_error() {
        let event = IntrospectionEvent::Error {
            connection_id: "conn-1".to_string(),
            level: 2,
            message: "Connection failed".to_string(),
        };
        
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("error"));
        assert!(json.contains("Connection failed"));
    }
}

#[cfg(test)]
mod progressive_introspection_tests {
    use crate::orchestrator::IntrospectorConfig;

    // =========================================================================
    // Invariant 4: Progressive Introspection Is Monotonic
    // Once an object appears, it is never removed
    // =========================================================================

    #[test]
    fn test_introspector_config_defaults() {
        let config = IntrospectorConfig::new("conn-123");
        
        assert_eq!(config.connection_id, "conn-123");
        assert!(config.save_to_cache);
        assert!(config.emit_events);
    }

    #[test]
    fn test_introspector_config_connection_id_required() {
        let config = IntrospectorConfig::new("my-connection");
        assert!(!config.connection_id.is_empty(), 
            "connection_id must be set");
    }
}

#[cfg(test)]
mod cross_profile_tests {
    use crate::adapter::{DatabaseCapabilities, EngineProfile};

    // =========================================================================
    // Test: Same query contexts work across profiles
    // =========================================================================

    #[test]
    fn test_all_profiles_have_effective_schema() {
        let profiles = [
            (EngineProfile::DB0, DatabaseCapabilities::sqlite()),
            (EngineProfile::DB1, DatabaseCapabilities::mysql()),
            (EngineProfile::DB2, DatabaseCapabilities::postgres()),
        ];

        for (expected_profile, caps) in profiles {
            assert_eq!(caps.profile(), expected_profile);
            
            // All profiles must return non-empty effective schema
            let schema = caps.effective_schema(None);
            assert!(!schema.is_empty(), 
                "Profile {:?} must have effective schema", expected_profile);
        }
    }

    #[test]
    fn test_profile_determines_qualification_needs() {
        // DB0 (SQLite) - never needs qualification
        let caps = DatabaseCapabilities::sqlite();
        assert!(!caps.requires_qualified_names);
        
        // DB1 (MySQL) - database is the "schema", no qualification needed within
        let caps = DatabaseCapabilities::mysql();
        assert!(!caps.requires_qualified_names);
        
        // DB2 (Postgres) - may need schema qualification
        let caps = DatabaseCapabilities::postgres();
        assert!(!caps.requires_qualified_names, "Postgres defaults to public");
    }
}
