//! Mock schema loader for tests.
//!
//! Provides a standard test schema matching the DataGrip-parity test cases.

use super::graph::{SchemaGraph, TableInfo, ColumnInfo, ForeignKey};

/// Creates a test schema for the DataGrip-parity tests.
pub struct MockSchemaLoader;

impl MockSchemaLoader {
    /// Create the standard test schema with users, orders, etc.
    pub fn create_test_schema() -> SchemaGraph {
        let mut schema = SchemaGraph::new();
        
        // ===== USERS TABLE =====
        schema.add_table(TableInfo::new("users", "public", vec![
            ColumnInfo::new("id", "integer").primary_key(),
            ColumnInfo::new("email", "varchar").not_null().indexed(),
            ColumnInfo::new("created_at", "timestamp"),
            ColumnInfo::new("created_by", "integer"),
        ]));
        
        // ===== ORDERS TABLE =====
        schema.add_table(TableInfo::new("orders", "public", vec![
            ColumnInfo::new("id", "integer").primary_key(),
            ColumnInfo::new("user_id", "integer").indexed(),
            ColumnInfo::new("amount", "decimal"),
            ColumnInfo::new("description", "text"),
            ColumnInfo::new("total", "decimal"),
            ColumnInfo::new("created_at", "timestamp"),
            ColumnInfo::new("created_by", "integer"),
        ]));
        
        // ===== USER_TEAMS TABLE (for multi-hop join tests) =====
        schema.add_table(TableInfo::new("user_teams", "public", vec![
            ColumnInfo::new("id", "integer").primary_key(),
            ColumnInfo::new("user_id", "integer").indexed(),
            ColumnInfo::new("team_id", "integer").indexed(),
        ]));
        
        // ===== TEAMS TABLE =====
        schema.add_table(TableInfo::new("teams", "public", vec![
            ColumnInfo::new("id", "integer").primary_key(),
            ColumnInfo::new("name", "varchar"),
        ]));
        
        // ===== ADMINS TABLE (for CTE shadowing test) =====
        schema.add_table(TableInfo::new("admins", "public", vec![
            ColumnInfo::new("id", "integer").primary_key(),
            ColumnInfo::new("name", "varchar"),
        ]));
        
        // ===== PAYMENTS TABLE =====
        schema.add_table(TableInfo::new("payments", "public", vec![
            ColumnInfo::new("id", "integer").primary_key(),
            ColumnInfo::new("order_id", "integer").indexed(),
            ColumnInfo::new("amount", "decimal"),
        ]));
        
        // ===== SESSIONS TABLE =====
        schema.add_table(TableInfo::new("sessions", "public", vec![
            ColumnInfo::new("id", "integer").primary_key(),
            ColumnInfo::new("user_id", "integer").indexed(),
            ColumnInfo::new("token", "varchar"),
        ]));
        
        // ===== FOREIGN KEYS =====
        
        // orders.user_id → users.id
        schema.add_foreign_key(ForeignKey {
            from_table: "orders".to_string(),
            from_column: "user_id".to_string(),
            to_table: "users".to_string(),
            to_column: "id".to_string(),
        });
        
        // user_teams.user_id → users.id
        schema.add_foreign_key(ForeignKey {
            from_table: "user_teams".to_string(),
            from_column: "user_id".to_string(),
            to_table: "users".to_string(),
            to_column: "id".to_string(),
        });
        
        // user_teams.team_id → teams.id
        schema.add_foreign_key(ForeignKey {
            from_table: "user_teams".to_string(),
            from_column: "team_id".to_string(),
            to_table: "teams".to_string(),
            to_column: "id".to_string(),
        });
        
        // payments.order_id → orders.id
        schema.add_foreign_key(ForeignKey {
            from_table: "payments".to_string(),
            from_column: "order_id".to_string(),
            to_table: "orders".to_string(),
            to_column: "id".to_string(),
        });
        
        // sessions.user_id → users.id
        schema.add_foreign_key(ForeignKey {
            from_table: "sessions".to_string(),
            from_column: "user_id".to_string(),
            to_table: "users".to_string(),
            to_column: "id".to_string(),
        });
        
        schema
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_has_all_tables() {
        let schema = MockSchemaLoader::create_test_schema();
        
        assert!(schema.get_table("users").is_some());
        assert!(schema.get_table("orders").is_some());
        assert!(schema.get_table("user_teams").is_some());
        assert!(schema.get_table("teams").is_some());
        assert!(schema.get_table("admins").is_some());
        assert!(schema.get_table("payments").is_some());
        assert!(schema.get_table("sessions").is_some());
    }

    #[test]
    fn test_fk_relationships() {
        let schema = MockSchemaLoader::create_test_schema();
        
        // orders → users
        let fk = schema.find_fk_path("orders", "users");
        assert!(fk.is_some());
        let fk = fk.unwrap();
        assert_eq!(fk.from_column, "user_id");
        assert_eq!(fk.to_column, "id");
    }
}
