
#[cfg(test)]
mod tests {
    use crate::adapter_registry;
    use crate::adapter::{DatabaseAdapter, AdapterQueryResult};
    use serde_json::json;
    use std::sync::Arc;

    async fn create_sqlite_adapter() -> Arc<dyn DatabaseAdapter + Send + Sync> {
        // Ensure built-ins are registered for tests
        adapter_registry::init_builtins();

        let config = json!({
            "mode": "memory",
            "file": ":memory:" 
        });
        
        let adapter = adapter_registry::create("sqlite", config).expect("Failed to create SQLite adapter");
        Arc::new(adapter)
    }

    #[tokio::test]
    async fn test_standard_backend_flow() {
        // 1. Setup Adapter
        let adapter = create_sqlite_adapter().await;
        
        // 2. Connection
        // SQLite doesn't strictly require explicit connect() for in-memory if initialized correctly, 
        // but we follow the contract.
        adapter.connect().await.expect("Failed to connect");
        assert!(adapter.is_connected(), "Adapter should be connected");

        // 3. Create Table
        let create_table_sql = "
            CREATE TABLE users (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                email TEXT UNIQUE,
                age INTEGER
            );
        ";
        let _ = adapter.execute(create_table_sql).await.expect("Failed to create table");

        // 4. Insert Data
        let insert_sql = "INSERT INTO users (name, email, age) VALUES ('Alice', 'alice@example.com', 30)";
        let affected = adapter.execute(insert_sql).await.expect("Failed to insert data");
        assert_eq!(affected, 1, "Should insert 1 row");

        let insert_sql_2 = "INSERT INTO users (name, email, age) VALUES ('Bob', 'bob@example.com', 25)";
        let affected_2 = adapter.execute(insert_sql_2).await.expect("Failed to insert data 2");
        assert_eq!(affected_2, 1, "Should insert 1 row");

        // 5. Select Data
        let select_sql = "SELECT id, name, email, age FROM users ORDER BY id ASC";
        let result: AdapterQueryResult = adapter.query(select_sql).await.expect("Failed to select data");
        
        assert_eq!(result.rows.len(), 2, "Should return 2 rows");
        
        // Verify columns
        assert_eq!(result.columns.len(), 4);
        assert_eq!(result.columns[1].name, "name");

        // Verify row data (Alice)
        let row1 = &result.rows[0];
        assert_eq!(row1["name"], "Alice");
        assert_eq!(row1["age"], 30);

        // Verify row data (Bob)
        let row2 = &result.rows[1];
        assert_eq!(row2["name"], "Bob");
        assert_eq!(row2["age"], 25);

        // 6. Update Data
        let update_sql = "UPDATE users SET age = 31 WHERE name = 'Alice'";
        let affected_update = adapter.execute(update_sql).await.expect("Failed to update data");
        assert_eq!(affected_update, 1);

        // Verify Update
        let verify_update_sql = "SELECT age FROM users WHERE name = 'Alice'";
        let update_result = adapter.query(verify_update_sql).await.expect("Failed to verify update");
        assert_eq!(update_result.rows[0]["age"], 31);

        // 7. Transaction Test (Manual)
        // Note: Adapter interface might not expose explicit transaction objects yet, 
        // effectively testing if the underlying connection supports transactional commands.
        adapter.execute("BEGIN TRANSACTION").await.expect("Failed to begin transaction");
        
        adapter.execute("INSERT INTO users (name, email, age) VALUES ('Charlie', 'charlie@example.com', 40)").await.expect("Failed to insert in tx");
        
        adapter.execute("ROLLBACK").await.expect("Failed to rollback");

        // Verify Rollback
        let verify_rollback_sql = "SELECT count(*) as count FROM users WHERE name = 'Charlie'";
        let rollback_result = adapter.query(verify_rollback_sql).await.expect("Failed to verify rollback");
        // Depending on how count is returned (number or string), check strictly or loosely
        // Usually, SQL count returns an integer.
        let count_val = rollback_result.rows[0]["count"].as_i64().or(rollback_result.rows[0][0].as_i64()).unwrap_or(0);
        assert_eq!(count_val, 0, "Charlie should not exist after rollback");

        // 8. Delete Data
        let delete_sql = "DELETE FROM users WHERE name = 'Bob'";
        let affected_delete = adapter.execute(delete_sql).await.expect("Failed to delete data");
        assert_eq!(affected_delete, 1);

        let final_count_sql = "SELECT count(*) as count FROM users";
        let final_result = adapter.query(final_count_sql).await.expect("Failed to get final count");
         let final_count_val = final_result.rows[0]["count"].as_i64().or(final_result.rows[0][0].as_i64()).unwrap_or(0);
        assert_eq!(final_count_val, 1, "Should have 1 user left");

         // 9. Teardown
         adapter.disconnect().await.expect("Failed to disconnect");
    }
}
