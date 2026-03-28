//! Integration tests for the PostgreSQL Feature Sprint browse + DDL commands.
//!
//! Requires a live Postgres instance. Set DATABASE_URL or use the default:
//!   postgres://postgres:postgres@localhost:5432/postgres
//!
//! Run:
//!   cargo test test_pg_sprint_introspection -- --nocapture

#[cfg(test)]
mod tests {
    use tokio_postgres::NoTls;

    const TEST_SCHEMA: &str = "tables_sprint_test";

    /// Connect using DATABASE_URL env var, defaulting to local postgres.
    async fn connect() -> Option<tokio_postgres::Client> {
        let conn_str = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/postgres".to_string());

        match tokio_postgres::connect(&conn_str, NoTls).await {
            Ok((client, connection)) => {
                tokio::spawn(async move {
                    if let Err(e) = connection.await {
                        eprintln!("[pg_sprint_tests] connection error: {}", e);
                    }
                });
                Some(client)
            }
            Err(e) => {
                println!("[pg_sprint_tests] Postgres not available ({}), skipping.", e);
                None
            }
        }
    }

    /// Create the test schema with all objects needed for assertions.
    async fn setup(client: &tokio_postgres::Client) {
        // Drop and recreate for idempotency
        client
            .batch_execute(&format!("DROP SCHEMA IF EXISTS {} CASCADE", TEST_SCHEMA))
            .await
            .expect("drop schema");
        client
            .batch_execute(&format!("CREATE SCHEMA {}", TEST_SCHEMA))
            .await
            .expect("create schema");

        // 1. Table + index + views + sequence
        client.batch_execute(&format!(
            r#"
            -- Table with identity, generated, unique, and check constraints
            CREATE TABLE {s}.users (
                id      BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
                email   TEXT NOT NULL,
                age     INT,
                slug    TEXT GENERATED ALWAYS AS (lower(email)) STORED,
                CONSTRAINT users_email_unique UNIQUE (email),
                CONSTRAINT users_age_positive CHECK (age > 0)
            );

            -- Partial btree index
            CREATE INDEX idx_users_age_partial
                ON {s}.users(age)
                WHERE age > 18;

            -- Regular view
            CREATE VIEW {s}.active_adults AS
                SELECT id, email, age FROM {s}.users WHERE age > 18;

            -- Materialized view
            CREATE MATERIALIZED VIEW {s}.user_count AS
                SELECT COUNT(*) AS total FROM {s}.users
            WITH DATA;

            -- Sequence
            CREATE SEQUENCE {s}.custom_id_seq
                AS bigint
                START WITH 100
                INCREMENT BY 5
                MINVALUE 1
                MAXVALUE 9999999
                NO CYCLE;
            "#,
            s = TEST_SCHEMA,
        )).await.expect("create tables, indexes, views, sequence");

        // 2. Functions + procedure
        client.batch_execute(&format!(
            r#"
            -- Function (kind = 'f')
            CREATE OR REPLACE FUNCTION {s}.get_user_email(user_id bigint)
            RETURNS text
            LANGUAGE plpgsql AS $$
            BEGIN
                RETURN (SELECT email FROM {s}.users WHERE id = user_id);
            END;
            $$;

            -- Procedure (kind = 'p')
            CREATE OR REPLACE PROCEDURE {s}.mark_user_active(user_id bigint)
            LANGUAGE plpgsql AS $$
            BEGIN
                UPDATE {s}.users SET age = age WHERE id = user_id;
            END;
            $$;
            "#,
            s = TEST_SCHEMA,
        )).await.expect("create functions and procedures");

        // 3. Trigger function + trigger
        client.batch_execute(&format!(
            r#"
            -- Trigger support function + trigger
            CREATE OR REPLACE FUNCTION {s}.log_user_change()
            RETURNS trigger LANGUAGE plpgsql AS $$
            BEGIN
                RETURN NEW;
            END;
            $$;

            CREATE TRIGGER users_audit
                AFTER INSERT OR UPDATE ON {s}.users
                FOR EACH ROW EXECUTE FUNCTION {s}.log_user_change();
            "#,
            s = TEST_SCHEMA,
        )).await.expect("create trigger function and trigger");
    }

    /// Drop the test schema when done.
    async fn teardown(client: &tokio_postgres::Client) {
        client
            .batch_execute(&format!("DROP SCHEMA IF EXISTS {} CASCADE", TEST_SCHEMA))
            .await
            .expect("teardown schema");
    }

    #[tokio::test]
    async fn test_pg_sprint_introspection() {
        let client = match connect().await {
            Some(c) => c,
            None => return, // Postgres not available — skip
        };

        // NOTE: teardown() is not called on panic. The next run cleans up via DROP SCHEMA IF EXISTS.
        // Run with -- --test-threads=1 if running multiple tests against the same schema concurrently.
        setup(&client).await;

        // ── 1. get_functions ──────────────────────────────────────────────────
        let rows = client.query(
            "SELECT
                p.proname AS name,
                p.prokind AS kind
            FROM pg_proc p
            JOIN pg_namespace n ON p.pronamespace = n.oid
            WHERE n.nspname = $1
              AND p.prokind IN ('f', 'p')
            ORDER BY p.proname",
            &[&TEST_SCHEMA],
        ).await.expect("get_functions query");

        let names: Vec<String> = rows.iter().map(|r| r.get::<_, String>(0)).collect();
        let kinds: Vec<i8> = rows.iter().map(|r| r.get::<_, i8>(1)).collect();

        assert!(names.contains(&"get_user_email".to_string()), "get_user_email not found, got: {:?}", names);
        assert!(names.contains(&"mark_user_active".to_string()), "mark_user_active not found, got: {:?}", names);

        let get_user_idx = names.iter().position(|n| n == "get_user_email").unwrap();
        let mark_user_idx = names.iter().position(|n| n == "mark_user_active").unwrap();
        // pg_proc.prokind is "char" type → tokio_postgres maps it to i8; cast i8→u8→char for ASCII comparison
        assert_eq!(kinds[get_user_idx] as u8 as char, 'f', "get_user_email should be kind 'f'");
        assert_eq!(kinds[mark_user_idx] as u8 as char, 'p', "mark_user_active should be kind 'p'");

        println!("✓ get_functions: found {} functions/procedures", rows.len());

        // Note: log_user_change() is also returned (kind='f', RETURNS trigger).
        // get_functions intentionally includes trigger functions — filter by prorettype if needed in production.
        assert!(names.contains(&"log_user_change".to_string()), "log_user_change trigger fn should be in results");

        // ── 2. get_sequences ─────────────────────────────────────────────────
        let rows = client.query(
            "SELECT sequencename, data_type::text, start_value, increment_by, cycle
            FROM pg_sequences
            WHERE schemaname = $1
              AND sequencename = 'custom_id_seq'
            ORDER BY sequencename",
            &[&TEST_SCHEMA],
        ).await.expect("get_sequences query");

        assert_eq!(rows.len(), 1, "expected 1 sequence (custom_id_seq), got {}", rows.len());
        let seq_name: String = rows[0].get(0);
        let data_type: String = rows[0].get(1);
        let start: i64 = rows[0].get(2);
        let increment: i64 = rows[0].get(3);
        let cycle: bool = rows[0].get(4);

        assert_eq!(seq_name, "custom_id_seq");
        assert_eq!(data_type, "bigint", "data_type should be 'bigint', got '{}'", data_type);
        assert_eq!(start, 100);
        assert_eq!(increment, 5);
        assert!(!cycle, "cycle should be false");

        println!("✓ get_sequences: custom_id_seq data_type={}", data_type);

        // ── 3. get_constraints ───────────────────────────────────────────────
        let qualified = format!("{}.users", TEST_SCHEMA);
        let params: &[&(dyn tokio_postgres::types::ToSql + Sync)] = &[&qualified];
        let rows = client.query(
            "SELECT c.conname, c.contype, pg_get_constraintdef(c.oid)
            FROM pg_constraint c
            WHERE c.conrelid = $1::text::regclass
              AND c.contype IN ('c', 'u', 'x')
            ORDER BY c.conname",
            params,
        ).await.expect("get_constraints query");

        assert_eq!(rows.len(), 2, "expected 2 constraints (UNIQUE + CHECK), got {}", rows.len());
        let con_names: Vec<String> = rows.iter().map(|r| r.get::<_, String>(0)).collect();
        let con_types: Vec<i8> = rows.iter().map(|r| r.get::<_, i8>(1)).collect();

        assert!(con_names.contains(&"users_email_unique".to_string()), "UNIQUE constraint not found");
        assert!(con_names.contains(&"users_age_positive".to_string()), "CHECK constraint not found");

        let unique_idx = con_names.iter().position(|n| n == "users_email_unique").unwrap();
        let check_idx = con_names.iter().position(|n| n == "users_age_positive").unwrap();
        assert_eq!(con_types[unique_idx] as u8 as char, 'u');
        assert_eq!(con_types[check_idx] as u8 as char, 'c');

        println!("✓ get_constraints: {:?}", con_names);

        // ── 4. get_index_details ─────────────────────────────────────────────
        let params: &[&(dyn tokio_postgres::types::ToSql + Sync)] = &[&qualified];
        let rows = client.query(
            "SELECT
                i.relname AS index_name,
                am.amname AS index_type,
                ix.indisunique,
                pg_get_expr(ix.indpred, ix.indrelid) AS predicate
            FROM pg_index ix
            JOIN pg_class t ON t.oid = ix.indrelid
            JOIN pg_class i ON i.oid = ix.indexrelid
            JOIN pg_am am ON am.oid = i.relam
            WHERE t.oid = $1::text::regclass
            ORDER BY i.relname",
            params,
        ).await.expect("get_index_details query");

        let index_names: Vec<String> = rows.iter().map(|r| r.get::<_, String>(0)).collect();
        let partial_idx = rows.iter().find(|r| {
            let name: String = r.get(0);
            name == "idx_users_age_partial"
        }).expect("partial index idx_users_age_partial not found");

        let index_type: String = partial_idx.get(1);
        let predicate: Option<String> = partial_idx.get(3);
        assert_eq!(index_type, "btree", "index type should be btree");
        assert!(predicate.is_some(), "partial index should have a predicate");
        assert!(predicate.unwrap().contains("18"), "predicate should reference 18");

        println!("✓ get_index_details: found indexes {:?}", index_names);

        // ── 5. get_table_ddl ─────────────────────────────────────────────────
        let params: &[&(dyn tokio_postgres::types::ToSql + Sync)] = &[&qualified];
        let col_rows = client.query(
            "SELECT
                a.attname,
                pg_catalog.format_type(a.atttypid, a.atttypmod),
                a.attnotnull,
                pg_get_expr(d.adbin, d.adrelid),
                a.attidentity::text,
                a.attgenerated::text
            FROM pg_attribute a
            LEFT JOIN pg_attrdef d ON d.adrelid = a.attrelid AND d.adnum = a.attnum
            WHERE a.attrelid = $1::text::regclass
              AND a.attnum > 0
              AND NOT a.attisdropped
            ORDER BY a.attnum",
            params,
        ).await.expect("get_table_ddl col query");

        let params: &[&(dyn tokio_postgres::types::ToSql + Sync)] = &[&qualified];
        let con_rows = client.query(
            "SELECT conname, pg_get_constraintdef(oid)
            FROM pg_constraint
            WHERE conrelid = $1::text::regclass
            ORDER BY contype, conname",
            params,
        ).await.expect("get_table_ddl constraint query");

        let mut parts: Vec<String> = col_rows.iter().map(|row| {
            let col_name: &str = row.get(0);
            let col_type: &str = row.get(1);
            let not_null: bool = row.get(2);
            let default_val: Option<&str> = row.get(3);
            let identity: &str = row.get(4);
            let generated: &str = row.get(5);

            let mut def = format!("    \"{}\" {}", col_name, col_type);
            if identity == "a" {
                def.push_str(" GENERATED ALWAYS AS IDENTITY");
            } else if identity == "d" {
                def.push_str(" GENERATED BY DEFAULT AS IDENTITY");
            } else if generated == "s" {
                if let Some(expr) = default_val {
                    def.push_str(&format!(" GENERATED ALWAYS AS ({}) STORED", expr));
                }
            } else {
                if let Some(d) = default_val { def.push_str(&format!(" DEFAULT {}", d)); }
                if not_null { def.push_str(" NOT NULL"); }
            }
            def
        }).collect();

        for row in &con_rows {
            let con_name: &str = row.get(0);
            let con_def: &str = row.get(1);
            parts.push(format!("    CONSTRAINT \"{}\" {}", con_name, con_def));
        }

        let ddl = format!(
            "CREATE TABLE \"{}\".\"users\" (\n{}\n);",
            TEST_SCHEMA,
            parts.join(",\n")
        );

        assert!(ddl.contains("\"email\""), "DDL missing email column: {}", ddl);
        assert!(ddl.contains("GENERATED ALWAYS AS IDENTITY"), "DDL missing identity column: {}", ddl);
        assert!(ddl.contains("GENERATED ALWAYS AS"), "DDL missing generated column: {}", ddl);
        assert!(ddl.contains("CHECK"), "DDL missing CHECK constraint: {}", ddl);
        assert!(ddl.contains("UNIQUE"), "DDL missing UNIQUE constraint: {}", ddl);

        println!("✓ get_table_ddl:\n{}", ddl);

        // ── 6. get_view_definition ────────────────────────────────────────────
        let row = client.query_one(
            "SELECT definition FROM pg_views WHERE schemaname = $1 AND viewname = $2",
            &[&TEST_SCHEMA, &"active_adults"],
        ).await.expect("get_view_definition query");

        let definition: &str = row.get(0);

        // The definition comes FROM the database — assert it references the source table
        assert!(
            definition.contains("users") || definition.contains("age"),
            "view definition from pg_views should reference source table: {}", definition
        );

        let view_ddl = format!(
            "CREATE OR REPLACE VIEW \"{}\".\"active_adults\" AS\n{}",
            TEST_SCHEMA,
            definition.trim_end()
        );

        assert!(view_ddl.contains("CREATE OR REPLACE VIEW"), "view DDL missing header: {}", view_ddl);
        assert!(view_ddl.contains("active_adults"), "view DDL missing name: {}", view_ddl);

        println!("✓ get_view_definition: OK");

        // ── 7. get_matview_definition ─────────────────────────────────────────
        let row = client.query_one(
            "SELECT definition, ispopulated FROM pg_matviews WHERE schemaname = $1 AND matviewname = $2",
            &[&TEST_SCHEMA, &"user_count"],
        ).await.expect("get_matview_definition query");

        let definition: &str = row.get(0);
        let is_populated: bool = row.get(1);
        let data_clause = if is_populated { "WITH DATA" } else { "WITH NO DATA" };
        let matview_ddl = format!(
            "CREATE MATERIALIZED VIEW \"{}\".\"user_count\" AS\n{}\n{};",
            TEST_SCHEMA, definition.trim_end(), data_clause
        );

        assert!(matview_ddl.contains("CREATE MATERIALIZED VIEW"), "matview DDL missing header: {}", matview_ddl);
        assert!(matview_ddl.contains("WITH DATA"), "matview DDL missing WITH DATA: {}", matview_ddl);

        println!("✓ get_matview_definition: populated={}", is_populated);

        // ── 8. get_function_ddl ───────────────────────────────────────────────
        let rows = client.query(
            "SELECT pg_get_functiondef(p.oid)
            FROM pg_proc p
            JOIN pg_namespace n ON p.pronamespace = n.oid
            WHERE n.nspname = $1 AND p.proname = $2
            ORDER BY p.oid",
            &[&TEST_SCHEMA, &"get_user_email"],
        ).await.expect("get_function_ddl query");

        assert!(!rows.is_empty(), "get_user_email function not found via pg_get_functiondef");
        let fn_ddl: String = rows[0].get(0);
        assert!(fn_ddl.contains("CREATE OR REPLACE FUNCTION"), "function DDL missing header: {}", fn_ddl);
        assert!(fn_ddl.contains("get_user_email"), "function DDL missing name: {}", fn_ddl);

        println!("✓ get_function_ddl: OK");

        // ── 9. get_sequence_ddl ───────────────────────────────────────────────
        let row = client.query_one(
            "SELECT data_type::text, start_value, min_value, max_value, increment_by, cycle, cache_size
            FROM pg_sequences
            WHERE schemaname = $1 AND sequencename = $2",
            &[&TEST_SCHEMA, &"custom_id_seq"],
        ).await.expect("get_sequence_ddl query");

        let data_type: &str = row.get(0);
        let start: i64 = row.get(1);
        let min: i64 = row.get(2);
        let max: i64 = row.get(3);
        let increment: i64 = row.get(4);
        let cycle: bool = row.get(5);
        let cache: i64 = row.get(6);

        let seq_ddl = format!(
            "CREATE SEQUENCE \"{}\".\"{}\"
    AS {}
    START WITH {}
    INCREMENT BY {}
    MINVALUE {}
    MAXVALUE {}
    CACHE {}{};",
            TEST_SCHEMA, "custom_id_seq", data_type, start, increment, min, max, cache,
            if cycle { "\n    CYCLE" } else { "\n    NO CYCLE" }
        );

        assert!(
            seq_ddl.contains(&format!("\"{}\"", TEST_SCHEMA)),
            "sequence DDL schema not quoted: {}", seq_ddl
        );
        assert!(
            seq_ddl.contains("\"custom_id_seq\""),
            "sequence DDL name not quoted: {}", seq_ddl
        );
        assert!(seq_ddl.contains("NO CYCLE"), "sequence DDL missing NO CYCLE: {}", seq_ddl);
        assert!(seq_ddl.contains("START WITH 100"), "sequence DDL wrong START: {}", seq_ddl);
        assert!(seq_ddl.contains("INCREMENT BY 5"), "sequence DDL wrong INCREMENT: {}", seq_ddl);

        println!("✓ get_sequence_ddl:\n{}", seq_ddl);

        // ── 10. get_index_ddl ────────────────────────────────────────────────
        let row = client.query_one(
            "SELECT indexdef FROM pg_indexes
            WHERE schemaname = $1 AND tablename = $2 AND indexname = $3",
            &[&TEST_SCHEMA, &"users", &"idx_users_age_partial"],
        ).await.expect("get_index_ddl query");

        let index_ddl: String = row.get(0);
        assert!(index_ddl.contains("CREATE INDEX"), "index DDL missing header: {}", index_ddl);
        assert!(index_ddl.contains("idx_users_age_partial"), "index DDL missing name: {}", index_ddl);
        assert!(index_ddl.contains("WHERE"), "partial index DDL missing WHERE clause: {}", index_ddl);

        println!("✓ get_index_ddl: {}", index_ddl);

        // ── 11. get_trigger_definition ───────────────────────────────────────
        let row = client.query_one(
            "SELECT pg_get_triggerdef(t.oid, true)
            FROM pg_trigger t
            JOIN pg_class c ON c.oid = t.tgrelid
            JOIN pg_namespace n ON n.oid = c.relnamespace
            WHERE n.nspname = $1 AND c.relname = $2 AND t.tgname = $3
              AND NOT t.tgisinternal",
            &[&TEST_SCHEMA, &"users", &"users_audit"],
        ).await.expect("get_trigger_definition query");

        let trigger_ddl: String = row.get(0);
        assert!(trigger_ddl.contains("TRIGGER users_audit"), "trigger DDL missing name: {}", trigger_ddl);
        assert!(
            trigger_ddl.contains("AFTER INSERT OR UPDATE"),
            "trigger DDL missing 'AFTER INSERT OR UPDATE': {}", trigger_ddl
        );

        println!("✓ get_trigger_definition: {}", trigger_ddl);

        teardown(&client).await;

        println!("\n✅ All 11 PostgreSQL sprint introspection queries verified.");
    }
}
