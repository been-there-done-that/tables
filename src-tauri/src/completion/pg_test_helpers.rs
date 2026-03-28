//! PostgreSQL test helpers for integration tests.
//!
//! Connects to local Postgres, creates the multi-schema test fixture,
//! and returns a live SchemaGraph built from information_schema.
//!
//! The test DB is the default `postgres` database; all objects live in
//! freshly-created schemas (hr, sales, inventory) so they don't pollute
//! existing data. Setup is idempotent: DROP … IF EXISTS before CREATE.

use tokio_postgres::NoTls;
use crate::completion::schema::graph::{ColumnInfo, ForeignKey, SchemaGraph, TableInfo};

pub const PG_TEST_URL: &str = "postgres://postgres:postgres@localhost:5432/postgres";

/// The DDL to create (and recreate) the test fixture.
const SETUP_DDL: &str = r#"
-- Extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS pg_trgm;
CREATE EXTENSION IF NOT EXISTS ltree;
CREATE EXTENSION IF NOT EXISTS citext;

-- Drop and recreate schemas (idempotent)
DROP SCHEMA IF EXISTS hr       CASCADE;
DROP SCHEMA IF EXISTS sales    CASCADE;
DROP SCHEMA IF EXISTS inventory CASCADE;
CREATE SCHEMA hr;
CREATE SCHEMA sales;
CREATE SCHEMA inventory;

-- =========================================================
-- PUBLIC SCHEMA
-- =========================================================

DROP TABLE IF EXISTS public.invoices    CASCADE;
DROP TABLE IF EXISTS public.events      CASCADE;
DROP TABLE IF EXISTS public.audit_log   CASCADE;
DROP TABLE IF EXISTS public.user_roles  CASCADE;
DROP TABLE IF EXISTS public.roles       CASCADE;
DROP TABLE IF EXISTS public.users       CASCADE;

CREATE TABLE public.users (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email       CITEXT UNIQUE NOT NULL,
    name        VARCHAR(255) NOT NULL,
    created_at  TIMESTAMPTZ DEFAULT NOW(),
    metadata    JSONB DEFAULT '{}',
    tags        TEXT[] DEFAULT '{}'
);
CREATE INDEX idx_users_email_trgm ON public.users USING GIN (email gin_trgm_ops);
CREATE INDEX idx_users_tags       ON public.users USING GIN (tags);

CREATE TABLE public.roles (
    id          SERIAL PRIMARY KEY,
    name        VARCHAR(100) UNIQUE NOT NULL,
    permissions JSONB DEFAULT '[]'
);

CREATE TABLE public.user_roles (
    user_id    UUID REFERENCES public.users(id) ON DELETE CASCADE,
    role_id    INT  REFERENCES public.roles(id) ON DELETE CASCADE,
    granted_at TIMESTAMPTZ DEFAULT NOW(),
    PRIMARY KEY (user_id, role_id)
);
CREATE INDEX idx_user_roles_user_id ON public.user_roles(user_id);
CREATE INDEX idx_user_roles_role_id ON public.user_roles(role_id);

CREATE TABLE public.audit_log (
    id         BIGSERIAL PRIMARY KEY,
    table_name TEXT NOT NULL,
    operation  TEXT NOT NULL CHECK (operation IN ('INSERT','UPDATE','DELETE')),
    record_id  TEXT,
    old_values JSONB,
    new_values JSONB,
    user_id    UUID REFERENCES public.users(id),
    created_at TIMESTAMPTZ DEFAULT NOW()
);
CREATE INDEX idx_audit_log_user_id    ON public.audit_log(user_id);
CREATE INDEX idx_audit_log_created_at ON public.audit_log(created_at DESC);

CREATE TABLE public.events (
    id         BIGSERIAL PRIMARY KEY,
    user_id    UUID REFERENCES public.users(id),
    event_type TEXT NOT NULL,
    properties JSONB DEFAULT '{}',
    session_id UUID,
    created_at TIMESTAMPTZ DEFAULT NOW()
);
CREATE INDEX idx_events_user_id    ON public.events(user_id);
CREATE INDEX idx_events_session_id ON public.events(session_id);
CREATE INDEX idx_events_created_at ON public.events(created_at DESC);

-- =========================================================
-- HR SCHEMA
-- =========================================================

CREATE TABLE hr.departments (
    id         SERIAL PRIMARY KEY,
    name       VARCHAR(200) NOT NULL,
    parent_id  INT REFERENCES hr.departments(id),
    budget     NUMERIC(15,2),
    created_at TIMESTAMPTZ DEFAULT NOW()
);
CREATE INDEX idx_departments_parent ON hr.departments(parent_id);

CREATE TABLE hr.employees (
    id            SERIAL PRIMARY KEY,
    user_id       UUID REFERENCES public.users(id),
    department_id INT  REFERENCES hr.departments(id),
    manager_id    INT  REFERENCES hr.employees(id),
    title         VARCHAR(200),
    salary        NUMERIC(12,2),
    hired_at      DATE,
    is_active     BOOLEAN DEFAULT TRUE
);
CREATE INDEX idx_employees_user_id       ON hr.employees(user_id);
CREATE INDEX idx_employees_department_id ON hr.employees(department_id);
CREATE INDEX idx_employees_manager_id    ON hr.employees(manager_id);

-- =========================================================
-- SALES SCHEMA
-- =========================================================

CREATE TABLE sales.categories (
    id        SERIAL PRIMARY KEY,
    name      VARCHAR(200) NOT NULL,
    parent_id INT REFERENCES sales.categories(id),
    path      LTREE
);
CREATE INDEX idx_categories_path ON sales.categories USING GIST (path);

CREATE TABLE sales.products (
    id          SERIAL PRIMARY KEY,
    sku         VARCHAR(100) UNIQUE NOT NULL,
    name        VARCHAR(500) NOT NULL,
    description TEXT,
    price       NUMERIC(10,2),
    category_id INT REFERENCES sales.categories(id),
    created_at  TIMESTAMPTZ DEFAULT NOW()
);
CREATE INDEX idx_products_category ON sales.products(category_id);
CREATE INDEX idx_products_name_trgm ON sales.products USING GIN (name gin_trgm_ops);

CREATE TABLE sales.orders (
    id           SERIAL PRIMARY KEY,
    user_id      UUID REFERENCES public.users(id),
    status       VARCHAR(50) DEFAULT 'pending'
                     CHECK (status IN ('pending','processing','completed','cancelled','refunded')),
    total_amount NUMERIC(12,2),
    created_at   TIMESTAMPTZ DEFAULT NOW(),
    shipped_at   TIMESTAMPTZ,
    metadata     JSONB DEFAULT '{}'
);
CREATE INDEX idx_orders_user_id    ON sales.orders(user_id);
CREATE INDEX idx_orders_status     ON sales.orders(status);
CREATE INDEX idx_orders_created_at ON sales.orders(created_at DESC);

CREATE TABLE sales.order_items (
    id         SERIAL PRIMARY KEY,
    order_id   INT REFERENCES sales.orders(id) ON DELETE CASCADE,
    product_id INT REFERENCES sales.products(id),
    quantity   INT          NOT NULL CHECK (quantity > 0),
    unit_price NUMERIC(10,2) NOT NULL,
    discount   NUMERIC(5,2) DEFAULT 0 CHECK (discount >= 0 AND discount <= 100)
);
CREATE INDEX idx_order_items_order_id   ON sales.order_items(order_id);
CREATE INDEX idx_order_items_product_id ON sales.order_items(product_id);

CREATE TABLE public.invoices (
    id             SERIAL PRIMARY KEY,
    order_id       INT REFERENCES sales.orders(id) UNIQUE,
    invoice_number VARCHAR(50) UNIQUE,
    issued_at      TIMESTAMPTZ DEFAULT NOW(),
    due_at         TIMESTAMPTZ,
    paid_at        TIMESTAMPTZ,
    amount         NUMERIC(12,2)
);

-- =========================================================
-- INVENTORY SCHEMA
-- =========================================================

CREATE TABLE inventory.warehouses (
    id            SERIAL PRIMARY KEY,
    name          VARCHAR(200) NOT NULL,
    location_code VARCHAR(50) UNIQUE,
    address       TEXT
);

CREATE TABLE inventory.stock (
    id                SERIAL PRIMARY KEY,
    product_id        INT REFERENCES sales.products(id),
    warehouse_id      INT REFERENCES inventory.warehouses(id),
    quantity          INT DEFAULT 0,
    reserved_quantity INT DEFAULT 0,
    last_updated      TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE (product_id, warehouse_id)
);
CREATE INDEX idx_stock_product_id   ON inventory.stock(product_id);
CREATE INDEX idx_stock_warehouse_id ON inventory.stock(warehouse_id)
"#;

/// Connect to the test Postgres instance, run the full setup DDL,
/// then introspect `information_schema` and return a ready SchemaGraph.
pub async fn build_pg_test_schema() -> SchemaGraph {
    let (client, connection) = tokio_postgres::connect(PG_TEST_URL, NoTls)
        .await
        .expect("connect to local postgres");

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("[pg_test_helpers] connection error: {e}");
        }
    });

    // Execute DDL statement-by-statement (tokio-postgres doesn't support multi-stmt)
    for stmt in split_ddl(SETUP_DDL) {
        client.execute(stmt, &[]).await
            .unwrap_or_else(|e| panic!("DDL failed:\n{stmt}\nError: {e}"));
    }

    build_schema_graph_from_pg(&client, &["public", "hr", "sales", "inventory"]).await
}

/// Split a DDL script on semicolons, skipping blank lines.
fn split_ddl(ddl: &str) -> Vec<&str> {
    ddl.split(';')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Query `information_schema` to build a SchemaGraph.
///
/// Fetches: tables, columns (with PK flag), foreign keys, and index info.
pub async fn build_schema_graph_from_pg(
    client: &tokio_postgres::Client,
    schemas: &[&str],
) -> SchemaGraph {
    let mut graph = SchemaGraph::new();

    let schema_vec: Vec<String> = schemas.iter().map(|s| s.to_string()).collect();

    // ---- columns (+ PK detection) ----------------------------------------
    let col_rows = client.query(
        r#"
        SELECT
            c.table_schema,
            c.table_name,
            c.column_name,
            c.data_type,
            CASE WHEN pk.column_name IS NOT NULL THEN TRUE ELSE FALSE END AS is_pk
        FROM information_schema.columns c
        LEFT JOIN (
            SELECT kcu.table_schema, kcu.table_name, kcu.column_name
            FROM information_schema.table_constraints tc
            JOIN information_schema.key_column_usage kcu
              ON tc.constraint_name = kcu.constraint_name
             AND tc.table_schema    = kcu.table_schema
            WHERE tc.constraint_type = 'PRIMARY KEY'
        ) pk USING (table_schema, table_name, column_name)
        WHERE c.table_schema = ANY($1)
          AND c.table_name IN (
              SELECT table_name FROM information_schema.tables
              WHERE table_schema = ANY($1) AND table_type = 'BASE TABLE'
          )
        ORDER BY c.table_schema, c.table_name, c.ordinal_position
        "#,
        &[&schema_vec],
    )
    .await
    .expect("query columns");

    // Group columns by (schema, table)
    let mut table_map: std::collections::BTreeMap<(String, String), Vec<ColumnInfo>> =
        std::collections::BTreeMap::new();

    for row in &col_rows {
        let tschema: &str = row.get(0);
        let tname: &str   = row.get(1);
        let col_name: &str = row.get(2);
        let data_type: &str = row.get(3);
        let is_pk: bool    = row.get(4);

        table_map
            .entry((tschema.to_string(), tname.to_string()))
            .or_default()
            .push(ColumnInfo {
                name: col_name.to_string(),
                data_type: data_type.to_string(),
                is_nullable: true,   // simplified
                is_primary_key: is_pk,
                is_indexed: false,   // patched below
            });
    }

    // ---- indexed columns (single-column indexes) -------------------------
    let idx_rows = client.query(
        r#"
        SELECT
            n.nspname  AS table_schema,
            t.relname  AS table_name,
            a.attname  AS column_name
        FROM pg_index ix
        JOIN pg_class  t ON t.oid = ix.indrelid
        JOIN pg_class  i ON i.oid = ix.indexrelid
        JOIN pg_attribute a ON a.attrelid = t.oid AND a.attnum = ANY(ix.indkey)
        JOIN pg_namespace n ON n.oid = t.relnamespace
        WHERE n.nspname = ANY($1)
          AND ix.indisprimary = FALSE
          AND array_length(ix.indkey, 1) = 1
        "#,
        &[&schema_vec],
    )
    .await
    .expect("query indexes");

    let mut indexed: std::collections::HashSet<(String, String, String)> =
        std::collections::HashSet::new();
    for row in &idx_rows {
        let s: &str = row.get(0);
        let t: &str = row.get(1);
        let c: &str = row.get(2);
        indexed.insert((s.to_string(), t.to_string(), c.to_string()));
    }

    // Patch is_indexed
    for ((schema, tname), cols) in &mut table_map {
        for col in cols.iter_mut() {
            if indexed.contains(&(schema.clone(), tname.clone(), col.name.clone())) {
                col.is_indexed = true;
            }
        }
    }

    // Register tables
    for ((schema, tname), columns) in table_map {
        graph.add_table(TableInfo { name: tname, schema, columns });
    }

    // ---- foreign keys ----------------------------------------------------
    // Use pg_constraint directly to handle cross-schema FKs correctly
    let fk_rows = client.query(
        r#"
        SELECT
            src_ns.nspname  AS from_schema,
            src_cl.relname  AS from_table,
            src_at.attname  AS from_column,
            tgt_ns.nspname  AS to_schema,
            tgt_cl.relname  AS to_table,
            tgt_at.attname  AS to_column
        FROM pg_constraint con
        JOIN pg_class     src_cl ON src_cl.oid = con.conrelid
        JOIN pg_namespace src_ns ON src_ns.oid = src_cl.relnamespace
        JOIN pg_class     tgt_cl ON tgt_cl.oid = con.confrelid
        JOIN pg_namespace tgt_ns ON tgt_ns.oid = tgt_cl.relnamespace
        JOIN pg_attribute src_at ON src_at.attrelid = src_cl.oid
                                 AND src_at.attnum = ANY(con.conkey)
        JOIN pg_attribute tgt_at ON tgt_at.attrelid = tgt_cl.oid
                                 AND tgt_at.attnum = ANY(con.confkey)
        WHERE con.contype = 'f'
          AND src_ns.nspname = ANY($1)
        "#,
        &[&schema_vec],
    )
    .await
    .expect("query foreign keys");

    for row in &fk_rows {
        graph.add_foreign_key(ForeignKey {
            from_table:  row.get::<_, &str>(1).to_string(),
            from_column: row.get::<_, &str>(2).to_string(),
            to_table:    row.get::<_, &str>(4).to_string(),
            to_column:   row.get::<_, &str>(5).to_string(),
        });
    }

    graph
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn smoke_setup_creates_tables() {
        let schema = build_pg_test_schema().await;
        assert!(schema.get_table("users").is_some(),      "public.users missing");
        assert!(schema.get_table("employees").is_some(),  "hr.employees missing");
        assert!(schema.get_table("orders").is_some(),     "sales.orders missing");
        assert!(schema.get_table("stock").is_some(),      "inventory.stock missing");
        assert!(schema.get_table("audit_log").is_some(),  "public.audit_log missing");
        assert!(schema.get_table("departments").is_some(),"hr.departments missing");
        assert!(schema.get_table("categories").is_some(), "sales.categories missing");
        assert!(schema.get_table("order_items").is_some(),"sales.order_items missing");
        assert!(schema.get_table("warehouses").is_some(), "inventory.warehouses missing");
        assert!(schema.get_table("invoices").is_some(),   "public.invoices missing");
    }

    #[tokio::test]
    async fn schema_has_expected_columns() {
        let schema = build_pg_test_schema().await;

        let users = schema.get_table("users").expect("users");
        let col_names: Vec<&str> = users.columns.iter().map(|c| c.name.as_str()).collect();
        assert!(col_names.contains(&"id"),         "users.id missing");
        assert!(col_names.contains(&"email"),      "users.email missing");
        assert!(col_names.contains(&"metadata"),   "users.metadata missing");
        assert!(col_names.contains(&"tags"),       "users.tags missing");

        let employees = schema.get_table("employees").expect("employees");
        let emp_cols: Vec<&str> = employees.columns.iter().map(|c| c.name.as_str()).collect();
        assert!(emp_cols.contains(&"user_id"),       "employees.user_id missing");
        assert!(emp_cols.contains(&"department_id"), "employees.department_id missing");
        assert!(emp_cols.contains(&"manager_id"),    "employees.manager_id missing");
        assert!(emp_cols.contains(&"salary"),        "employees.salary missing");

        let order_items = schema.get_table("order_items").expect("order_items");
        let oi_cols: Vec<&str> = order_items.columns.iter().map(|c| c.name.as_str()).collect();
        assert!(oi_cols.contains(&"order_id"),   "order_items.order_id missing");
        assert!(oi_cols.contains(&"product_id"), "order_items.product_id missing");
        assert!(oi_cols.contains(&"unit_price"), "order_items.unit_price missing");
        assert!(oi_cols.contains(&"discount"),   "order_items.discount missing");
    }

    #[tokio::test]
    async fn schema_has_foreign_keys() {
        let schema = build_pg_test_schema().await;
        let fk_path = schema.find_fk_path("orders", "users");
        assert!(fk_path.is_some(), "FK path orders→users missing");
        let fk_oi = schema.find_fk_path("order_items", "orders");
        assert!(fk_oi.is_some(), "FK path order_items→orders missing");
    }

    #[tokio::test]
    async fn schema_has_indexed_columns() {
        let schema = build_pg_test_schema().await;
        let employees = schema.get_table("employees").expect("employees");
        let user_id_col = employees.columns.iter()
            .find(|c| c.name == "user_id").expect("employees.user_id");
        assert!(user_id_col.is_indexed, "employees.user_id should be indexed");

        let orders = schema.get_table("orders").expect("orders");
        let status_col = orders.columns.iter()
            .find(|c| c.name == "status").expect("orders.status");
        assert!(status_col.is_indexed, "orders.status should be indexed");
    }
}
