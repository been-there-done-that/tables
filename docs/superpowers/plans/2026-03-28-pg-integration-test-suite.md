# PostgreSQL Integration Test Suite Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a comprehensive integration test suite that creates a real multi-schema PostgreSQL database, introspects it into a live SchemaGraph, and exercises every meaningful SQL construct (CTEs with N levels of nesting, all JOIN types, window functions, LATERAL, extensions, CTE shadowing, recursive CTEs) for both completion and diagnostic correctness.

**Architecture:** Two new Rust files: `pg_test_helpers.rs` provides async helpers to create/tear-down the test schema in `postgres://postgres:postgres@localhost:5432/postgres` and build a live SchemaGraph from `information_schema`. `pg_integration_tests.rs` houses all completion and diagnostic tests; it shares a single `OnceLock<SchemaGraph>` so the DB is introspected once per test binary run. Each completion test calls the same `complete(sql, schema)` helper used in `tests.rs`, but with the live schema instead of the mock.

**Tech Stack:** Rust + `tokio-postgres` (already in deps), `tokio::sync::OnceLock`, `sql_scope::resolve` + `run_diagnostics`, `crate::completion::engines::PostgresEngine`, `information_schema` introspection queries.

---

## Schema being created

The test creates three extra schemas (`hr`, `sales`, `inventory`) plus objects in `public`:

| Schema | Tables | FK relationships |
|--------|--------|-----------------|
| public | `users`, `roles`, `user_roles`, `audit_log`, `events`, `invoices` | user_roles→users, user_roles→roles, audit_log→users, events→users, invoices→orders |
| hr | `departments` (self-ref), `employees` (self-ref manager) | employees→users, employees→departments |
| sales | `categories` (self-ref), `products`, `orders`, `order_items` | orders→users, order_items→orders, order_items→products, products→categories |
| inventory | `warehouses`, `stock` | stock→products, stock→warehouses |

Extensions installed: `uuid-ossp`, `pg_trgm`, `ltree`, `citext`.

---

## File Map

| File | Action | Responsibility |
|------|--------|----------------|
| `src-tauri/src/completion/pg_test_helpers.rs` | Create | Async DB setup, information_schema introspection, SchemaGraph builder |
| `src-tauri/src/completion/pg_integration_tests.rs` | Create | All completion + diagnostic integration tests |
| `src-tauri/src/lib.rs` | Modify | Add `#[cfg(test)] mod pg_test_helpers;` and `#[cfg(test)] mod pg_integration_tests;` (inside `completion` module declaration) |

---

## Task 1: pg_test_helpers — DB setup + live SchemaGraph builder

**Files:**
- Create: `src-tauri/src/completion/pg_test_helpers.rs`
- Modify: `src-tauri/src/completion/mod.rs` (add `#[cfg(test)] pub mod pg_test_helpers;`)

- [ ] **Step 1: Write the failing test (connection + schema creation smoke test)**

```rust
// At the bottom of pg_test_helpers.rs — this will fail until the module exists
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn smoke_setup_creates_tables() {
        let schema = build_pg_test_schema().await;
        // Should have tables from all four schemas
        assert!(schema.get_table("users").is_some(), "public.users missing");
        assert!(schema.get_table("employees").is_some(), "hr.employees missing");
        assert!(schema.get_table("orders").is_some(), "sales.orders missing");
        assert!(schema.get_table("stock").is_some(), "inventory.stock missing");
    }

    #[tokio::test]
    async fn schema_has_foreign_keys() {
        let schema = build_pg_test_schema().await;
        // orders.user_id → users.id
        let fk_path = schema.find_fk_path("orders", "users");
        assert!(fk_path.is_some(), "FK path orders→users missing");
    }

    #[tokio::test]
    async fn schema_has_indexed_columns() {
        let schema = build_pg_test_schema().await;
        let employees = schema.get_table("employees").expect("employees table");
        let user_id_col = employees.columns.iter().find(|c| c.name == "user_id").expect("user_id");
        assert!(user_id_col.is_indexed, "employees.user_id should be indexed");
    }
}
```

- [ ] **Step 2: Run to verify it fails**

```bash
cd src-tauri && cargo test --lib "pg_test_helpers" 2>&1 | head -20
```
Expected: compile error — `pg_test_helpers` module not found.

- [ ] **Step 3: Create `src-tauri/src/completion/pg_test_helpers.rs`**

```rust
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
CREATE INDEX idx_stock_warehouse_id ON inventory.stock(warehouse_id);
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
        &[&schemas],
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
        &[&schemas],
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
    let fk_rows = client.query(
        r#"
        SELECT
            kcu.table_name  AS from_table,
            kcu.column_name AS from_column,
            ccu.table_name  AS to_table,
            ccu.column_name AS to_column
        FROM information_schema.table_constraints tc
        JOIN information_schema.key_column_usage kcu
          ON tc.constraint_name = kcu.constraint_name
         AND tc.table_schema    = kcu.table_schema
        JOIN information_schema.constraint_column_usage ccu
          ON tc.constraint_name = ccu.constraint_name
         AND tc.table_schema    = ccu.table_schema
        WHERE tc.constraint_type = 'FOREIGN KEY'
          AND tc.table_schema    = ANY($1)
        "#,
        &[&schemas],
    )
    .await
    .expect("query foreign keys");

    for row in &fk_rows {
        graph.add_foreign_key(ForeignKey {
            from_table:  row.get::<_, &str>(0).to_string(),
            from_column: row.get::<_, &str>(1).to_string(),
            to_table:    row.get::<_, &str>(2).to_string(),
            to_column:   row.get::<_, &str>(3).to_string(),
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
        // Direct FK: orders.user_id → users.id
        let fk_path = schema.find_fk_path("orders", "users");
        assert!(fk_path.is_some(), "FK path orders→users missing");
        // Direct FK: order_items.order_id → orders.id
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
```

- [ ] **Step 4: Register the module in `src-tauri/src/completion/mod.rs`**

Add to the bottom of `mod.rs`:
```rust
#[cfg(test)]
pub mod pg_test_helpers;
```

- [ ] **Step 5: Run tests**

```bash
cd src-tauri && cargo test --lib "pg_test_helpers" -- --nocapture 2>&1 | tail -20
```
Expected:
```
test completion::pg_test_helpers::tests::smoke_setup_creates_tables ... ok
test completion::pg_test_helpers::tests::schema_has_expected_columns ... ok
test completion::pg_test_helpers::tests::schema_has_foreign_keys ... ok
test completion::pg_test_helpers::tests::schema_has_indexed_columns ... ok
test result: ok. 4 passed; 0 failed
```

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/completion/pg_test_helpers.rs src-tauri/src/completion/mod.rs
git commit -m "test(completion): add pg_test_helpers — live schema builder from postgres"
```

---

## Task 2: Completion integration tests on complex multi-CTE SQL

**Files:**
- Create: `src-tauri/src/completion/pg_integration_tests.rs`
- Modify: `src-tauri/src/completion/mod.rs` (add `#[cfg(test)] pub mod pg_integration_tests;`)

The test SQL strings in this task use `|` to mark cursor positions, exactly like `tests.rs`.

- [ ] **Step 1: Write failing test stubs**

```rust
// pg_integration_tests.rs
#[cfg(test)]
mod pg_completion_tests {
    use super::*;
    use crate::completion::pg_test_helpers::build_pg_test_schema;

    // One stub test — will fail until the module exists
    #[tokio::test]
    async fn pg_placeholder_fails() {
        panic!("pg_integration_tests not yet implemented");
    }
}
```

Add `#[cfg(test)] pub mod pg_integration_tests;` to `mod.rs`.

- [ ] **Step 2: Verify it fails**

```bash
cd src-tauri && cargo test --lib "pg_completion_tests::pg_placeholder" 2>&1 | tail -5
```
Expected: `FAILED — pg_integration_tests not yet implemented`.

- [ ] **Step 3: Write the full `pg_integration_tests.rs`**

Replace the stub with the full file below. This file has three sections:
- A shared `OnceLock<SchemaGraph>` so setup runs once per binary
- A `complete_pg(sql)` helper function
- 20 targeted completion assertions

```rust
//! PostgreSQL completion integration tests.
//!
//! These tests connect to a real Postgres instance, create a multi-schema
//! test fixture, introspect it into a SchemaGraph, then run the completion
//! engine against SQL strings containing complex constructs.
//!
//! Run with:
//!   cargo test --lib "pg_completion_tests" -- --nocapture
//!
//! Requires: postgres://postgres:postgres@localhost:5432/postgres running.

use std::sync::OnceLock;
use crate::completion::schema::graph::SchemaGraph;
use crate::completion::engines::{PostgresEngine, CompletionEngineVariant};
use crate::completion::items::CompletionItem;
use crate::completion::context::Context;
use crate::completion::parsing::parse_sql;
use crate::completion::pg_test_helpers::build_pg_test_schema;

// --------------------------------------------------------------------------
// Shared schema: initialised once across all tests in this binary.
// --------------------------------------------------------------------------

static PG_SCHEMA: OnceLock<SchemaGraph> = OnceLock::new();

/// Get (or lazily initialise) the shared live SchemaGraph.
/// Panics if Postgres is unreachable — intentional: test setup must work.
async fn pg_schema() -> &'static SchemaGraph {
    if PG_SCHEMA.get().is_none() {
        let schema = build_pg_test_schema().await;
        // Ignore the error from set() — a concurrent test may have beaten us.
        let _ = PG_SCHEMA.set(schema);
    }
    PG_SCHEMA.get().unwrap()
}

// --------------------------------------------------------------------------
// Completion helper
// --------------------------------------------------------------------------

/// Run the PostgresEngine at the `|` cursor marker in `sql`.
/// Returns completion labels in score order.
fn complete_labels(sql: &str, schema: &SchemaGraph) -> Vec<String> {
    complete_items(sql, schema).into_iter().map(|i| i.label).collect()
}

fn complete_items(sql: &str, schema: &SchemaGraph) -> Vec<CompletionItem> {
    let cursor = sql.find('|').expect("SQL must contain | for cursor position");
    let source   = sql.replace('|', "");
    let scope_sql = sql.replace('|', "x");

    let tree = parse_sql(&source, None);
    let scope_tree = sql_scope::resolve(&scope_sql, sql_scope::Dialect::Postgres, schema)
        .or_else(|_| sql_scope::resolve(&source,   sql_scope::Dialect::Postgres, schema))
        .unwrap_or_default();

    let context = Context::analyze(&source, tree.as_ref(), cursor);
    let engine  = PostgresEngine::new();
    engine.complete(&scope_tree, &context, schema, Some("public"), None)
}

fn has_labels(items: &[String], expected: &[&str]) -> bool {
    expected.iter().all(|e| items.iter().any(|i| i == *e))
}

fn lacks_labels(items: &[String], unexpected: &[&str]) -> bool {
    !unexpected.iter().any(|e| items.iter().any(|i| i == *e))
}

// ==========================================================================
// GROUP A: Direct alias → real table columns (public schema)
// ==========================================================================

#[tokio::test]
async fn pg_a1_users_alias_dot() {
    let schema = pg_schema().await;
    // Simple alias for public.users
    let labels = complete_labels("SELECT u.| FROM users u", schema);
    assert!(has_labels(&labels, &["id", "email", "name", "metadata", "tags"]),
        "Expected users columns, got: {:?}", labels);
    assert!(lacks_labels(&labels, &["order_id", "salary"]),
        "Should not include columns from other tables: {:?}", labels);
}

#[tokio::test]
async fn pg_a2_order_items_alias_dot() {
    let schema = pg_schema().await;
    let labels = complete_labels(
        "SELECT oi.| FROM sales.order_items oi", schema);
    assert!(has_labels(&labels, &["id", "order_id", "product_id", "quantity", "unit_price", "discount"]),
        "Expected order_items columns, got: {:?}", labels);
}

#[tokio::test]
async fn pg_a3_cross_schema_employees_dot() {
    let schema = pg_schema().await;
    let labels = complete_labels(
        "SELECT e.| FROM hr.employees e", schema);
    assert!(has_labels(&labels, &["id", "user_id", "department_id", "manager_id", "salary", "is_active"]),
        "Expected employees columns, got: {:?}", labels);
}

#[tokio::test]
async fn pg_a4_join_condition_alias_dot() {
    let schema = pg_schema().await;
    // After dot in JOIN ON clause — should see right-hand table columns
    let labels = complete_labels(
        "SELECT * FROM sales.orders o JOIN users u ON o.user_id = u.|", schema);
    assert!(has_labels(&labels, &["id", "email", "name"]),
        "Expected users columns in JOIN condition, got: {:?}", labels);
}

#[tokio::test]
async fn pg_a5_multi_join_specific_alias_dot() {
    let schema = pg_schema().await;
    // Three-way join — cursor after specific alias
    let labels = complete_labels(
        "SELECT w.| FROM inventory.stock s \
         JOIN inventory.warehouses w ON s.warehouse_id = w.id \
         JOIN sales.products p ON s.product_id = p.id",
        schema,
    );
    assert!(has_labels(&labels, &["id", "name", "location_code", "address"]),
        "Expected warehouses columns, got: {:?}", labels);
    assert!(lacks_labels(&labels, &["quantity", "unit_price"]),
        "Should not show columns from other joined tables: {:?}", labels);
}

// ==========================================================================
// GROUP B: CTE aliases
// ==========================================================================

#[tokio::test]
async fn pg_b1_simple_cte_dot() {
    let schema = pg_schema().await;
    // CTE explicitly selects columns — those become the CTE's projection
    let sql = r#"
WITH active AS (
    SELECT id, email, name, created_at
    FROM users
    WHERE created_at > NOW() - INTERVAL '30 days'
)
SELECT a.| FROM active a"#;
    let labels = complete_labels(sql, schema);
    assert!(has_labels(&labels, &["id", "email", "name", "created_at"]),
        "Expected CTE projected columns, got: {:?}", labels);
    // metadata and tags were not selected into the CTE
    assert!(lacks_labels(&labels, &["metadata", "tags"]),
        "CTE only projected 4 columns, should not see metadata/tags: {:?}", labels);
}

#[tokio::test]
async fn pg_b2_cte_wildcard_inherits_all_columns() {
    let schema = pg_schema().await;
    // SELECT * CTE — should expose all source table columns
    let sql = r#"
WITH all_orders AS (
    SELECT * FROM sales.orders
)
SELECT ao.| FROM all_orders ao"#;
    let labels = complete_labels(sql, schema);
    assert!(has_labels(&labels, &["id", "user_id", "status", "total_amount", "created_at", "metadata"]),
        "CTE with SELECT * should expose all orders columns, got: {:?}", labels);
}

#[tokio::test]
async fn pg_b3_chained_cte_inherits_projected_columns() {
    let schema = pg_schema().await;
    // cte_b references cte_a — cursor in cte_b should see cte_a's projected columns
    let sql = r#"
WITH
cte_a AS (
    SELECT id, salary, department_id
    FROM hr.employees
    WHERE is_active = TRUE
),
cte_b AS (
    SELECT ca.| FROM cte_a ca
)
SELECT * FROM cte_b"#;
    let labels = complete_labels(sql, schema);
    assert!(has_labels(&labels, &["id", "salary", "department_id"]),
        "cte_b should see cte_a projected columns, got: {:?}", labels);
    assert!(lacks_labels(&labels, &["manager_id", "hired_at"]),
        "cte_b should NOT see columns cte_a didn't project: {:?}", labels);
}

#[tokio::test]
async fn pg_b4_cte_shadows_real_table() {
    let schema = pg_schema().await;
    // A CTE named 'orders' shadows the real sales.orders table
    let sql = r#"
WITH orders AS (
    SELECT id, user_id, total_amount
    FROM sales.orders
    WHERE status = 'completed'
)
SELECT o.| FROM orders o"#;
    let labels = complete_labels(sql, schema);
    // CTE only projected 3 columns
    assert!(has_labels(&labels, &["id", "user_id", "total_amount"]),
        "Should see CTE projected columns, got: {:?}", labels);
    assert!(lacks_labels(&labels, &["status", "shipped_at", "metadata"]),
        "CTE shadows real table — should NOT see non-projected columns: {:?}", labels);
}

#[tokio::test]
async fn pg_b5_recursive_cte_columns() {
    let schema = pg_schema().await;
    // Recursive CTE — cursor in the final SELECT referencing the CTE
    let sql = r#"
WITH RECURSIVE dept_tree AS (
    SELECT id, name, parent_id, 0 AS depth
    FROM hr.departments
    WHERE parent_id IS NULL
    UNION ALL
    SELECT d.id, d.name, d.parent_id, dt.depth + 1
    FROM hr.departments d
    INNER JOIN dept_tree dt ON d.parent_id = dt.id
)
SELECT dt.| FROM dept_tree dt"#;
    let labels = complete_labels(sql, schema);
    // Anchor SELECT projected: id, name, parent_id, depth
    assert!(has_labels(&labels, &["id", "name", "parent_id", "depth"]),
        "Recursive CTE should expose anchor columns, got: {:?}", labels);
}

#[tokio::test]
async fn pg_b6_cte_referencing_another_cte_in_join() {
    let schema = pg_schema().await;
    // Deep chain: cte_c joins cte_a and cte_b — cursor after cte_b alias dot
    let sql = r#"
WITH
emp_base AS (
    SELECT id AS emp_id, user_id, salary, department_id
    FROM hr.employees WHERE is_active = TRUE
),
dept_info AS (
    SELECT id AS dept_id, name AS dept_name, budget
    FROM hr.departments
),
enriched AS (
    SELECT eb.emp_id, di.|
    FROM emp_base eb
    JOIN dept_info di ON eb.department_id = di.dept_id
)
SELECT * FROM enriched"#;
    let labels = complete_labels(sql, schema);
    assert!(has_labels(&labels, &["dept_id", "dept_name", "budget"]),
        "Should see dept_info CTE columns at di. cursor, got: {:?}", labels);
}

// ==========================================================================
// GROUP C: N-level nesting (3+ CTE levels, subqueries in CTEs)
// ==========================================================================

#[tokio::test]
async fn pg_c1_four_level_cte_chain_cursor_at_level4() {
    let schema = pg_schema().await;
    // 4-level chain: l4 references l3 which references l2 which references l1
    let sql = r#"
WITH
l1 AS (SELECT id, user_id, status, total_amount FROM sales.orders),
l2 AS (SELECT l1.id, l1.user_id, l1.total_amount FROM l1 WHERE l1.status = 'completed'),
l3 AS (SELECT l2.id, l2.user_id, l2.total_amount * 1.1 AS adjusted FROM l2),
l4 AS (SELECT l3.| FROM l3)
SELECT * FROM l4"#;
    let labels = complete_labels(sql, schema);
    assert!(has_labels(&labels, &["id", "user_id", "adjusted"]),
        "l4 should see l3 projected columns, got: {:?}", labels);
}

#[tokio::test]
async fn pg_c2_correlated_subquery_cursor_inside() {
    let schema = pg_schema().await;
    // Cursor inside a correlated subquery — should see the outer alias
    let sql = r#"
SELECT
    u.id,
    u.email,
    (SELECT COUNT(o.id) FROM sales.orders o WHERE o.user_id = u.|)
FROM users u"#;
    let labels = complete_labels(sql, schema);
    assert!(has_labels(&labels, &["id", "email", "name", "metadata"]),
        "Correlated subquery should see outer alias u (users), got: {:?}", labels);
}

#[tokio::test]
async fn pg_c3_lateral_subquery_sees_outer_alias() {
    let schema = pg_schema().await;
    // LATERAL — cursor inside the lateral sees the left side's alias
    let sql = r#"
SELECT u.id, latest_order.id AS order_id
FROM users u
CROSS JOIN LATERAL (
    SELECT o.| FROM sales.orders o
    WHERE o.user_id = u.id
    ORDER BY o.created_at DESC
    LIMIT 1
) latest_order"#;
    let labels = complete_labels(sql, schema);
    assert!(has_labels(&labels, &["id", "user_id", "status", "total_amount", "created_at"]),
        "LATERAL inner should see sales.orders columns at o. cursor, got: {:?}", labels);
}

#[tokio::test]
async fn pg_c4_cte_with_window_function_and_dot_cursor() {
    let schema = pg_schema().await;
    // CTE body uses window functions; cursor is still in the CTE body at oi.
    let sql = r#"
WITH ranked_items AS (
    SELECT
        oi.|,
        ROW_NUMBER() OVER (PARTITION BY oi.order_id ORDER BY oi.unit_price DESC) AS rn
    FROM sales.order_items oi
)
SELECT * FROM ranked_items WHERE rn = 1"#;
    let labels = complete_labels(sql, schema);
    assert!(has_labels(&labels, &["id", "order_id", "product_id", "quantity", "unit_price", "discount"]),
        "CTE body with window function: should see order_items cols at oi. cursor, got: {:?}", labels);
}

// ==========================================================================
// GROUP D: FROM clause table suggestions
// ==========================================================================

#[tokio::test]
async fn pg_d1_from_clause_suggests_tables() {
    let schema = pg_schema().await;
    let labels = complete_labels("SELECT * FROM |", schema);
    assert!(has_labels(&labels, &["users", "roles", "audit_log", "events", "invoices"]),
        "FROM clause should suggest public tables, got: {:?}", labels);
}

#[tokio::test]
async fn pg_d2_join_clause_suggests_tables() {
    let schema = pg_schema().await;
    let labels = complete_labels("SELECT * FROM users u JOIN |", schema);
    assert!(has_labels(&labels, &["roles", "audit_log", "events"]),
        "JOIN clause should suggest tables, got: {:?}", labels);
}

// ==========================================================================
// GROUP E: WHERE / function argument completions with live schema
// ==========================================================================

#[tokio::test]
async fn pg_e1_where_clause_column_suggestions() {
    let schema = pg_schema().await;
    let labels = complete_labels("SELECT * FROM users u WHERE u.|", schema);
    assert!(has_labels(&labels, &["id", "email", "name", "created_at", "metadata", "tags"]),
        "WHERE clause should suggest users columns, got: {:?}", labels);
}

#[tokio::test]
async fn pg_e2_sum_function_argument_columns() {
    let schema = pg_schema().await;
    let labels = complete_labels(
        "SELECT SUM(oi.|) FROM sales.order_items oi", schema);
    assert!(has_labels(&labels, &["quantity", "unit_price", "discount"]),
        "SUM() arg should suggest numeric columns, got: {:?}", labels);
}

#[tokio::test]
async fn pg_e3_count_distinct_argument() {
    let schema = pg_schema().await;
    let labels = complete_labels(
        "SELECT COUNT(DISTINCT o.|) FROM sales.orders o", schema);
    assert!(has_labels(&labels, &["id", "user_id", "status", "total_amount"]),
        "COUNT(DISTINCT) arg should suggest orders columns, got: {:?}", labels);
}
```

- [ ] **Step 4: Register the module in `mod.rs`**

Add to `src-tauri/src/completion/mod.rs`:
```rust
#[cfg(test)]
pub mod pg_integration_tests;
```

- [ ] **Step 5: Run tests**

```bash
cd src-tauri && cargo test --lib "pg_completion_tests" -- --nocapture 2>&1 | tail -30
```
Expected:
```
test completion::pg_integration_tests::pg_completion_tests::pg_a1_users_alias_dot ... ok
test completion::pg_integration_tests::pg_completion_tests::pg_a2_order_items_alias_dot ... ok
...
test result: ok. 20 passed; 0 failed
```

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/completion/pg_integration_tests.rs src-tauri/src/completion/mod.rs
git commit -m "test(completion): add 20 live-schema completion integration tests"
```

---

## Task 3: Diagnostic integration tests

**Files:**
- Modify: `src-tauri/src/completion/pg_integration_tests.rs` (add `pg_diagnostic_tests` module)

- [ ] **Step 1: Write failing diagnostic test stub**

Add to the bottom of `pg_integration_tests.rs`:
```rust
#[cfg(test)]
mod pg_diagnostic_tests {
    #[tokio::test]
    async fn pg_diag_placeholder() {
        panic!("diagnostic tests not yet implemented");
    }
}
```

- [ ] **Step 2: Run to verify it fails**

```bash
cd src-tauri && cargo test --lib "pg_diag_placeholder" 2>&1 | tail -5
```

- [ ] **Step 3: Replace stub with full diagnostic test module**

Replace the `pg_diagnostic_tests` module with:

```rust
#[cfg(test)]
mod pg_diagnostic_tests {
    use super::*;
    use sql_scope::{run_diagnostics, DiagSeverity};

    fn diagnostics(sql: &str, schema: &SchemaGraph) -> Vec<sql_scope::ScopeDiagnostic> {
        let scope_tree = sql_scope::resolve(sql, sql_scope::Dialect::Postgres, schema)
            .unwrap_or_default();
        run_diagnostics(&scope_tree, schema, sql)
    }

    fn has_warning(diags: &[sql_scope::ScopeDiagnostic], fragment: &str) -> bool {
        diags.iter().any(|d| d.message.contains(fragment) && d.severity == DiagSeverity::Warning)
    }

    fn has_no_warning_containing(diags: &[sql_scope::ScopeDiagnostic], fragment: &str) -> bool {
        !diags.iter().any(|d| d.message.contains(fragment) && d.severity == DiagSeverity::Warning)
    }

    // ------------------------------------------------------------------
    // D1: Clean queries must produce ZERO false-positive warnings
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn pg_d1_clean_simple_query_no_warnings() {
        let schema = pg_schema().await;
        let diags = diagnostics("SELECT id, email FROM users WHERE created_at > NOW()", schema);
        assert!(diags.is_empty(), "clean query should have no diagnostics, got: {:?}", diags);
    }

    #[tokio::test]
    async fn pg_d2_cross_schema_join_no_false_positive() {
        let schema = pg_schema().await;
        let sql = "SELECT u.email, e.salary FROM users u JOIN hr.employees e ON u.id = e.user_id";
        let diags = diagnostics(sql, schema);
        // Neither users nor employees should fire an unknown-table warning
        assert!(has_no_warning_containing(&diags, "users"),     "users is a real table, no warning expected");
        assert!(has_no_warning_containing(&diags, "employees"), "employees is a real table, no warning expected");
    }

    #[tokio::test]
    async fn pg_d3_cte_name_not_flagged_as_unknown_table() {
        let schema = pg_schema().await;
        let sql = r#"
WITH active_users AS (SELECT id, email FROM users WHERE created_at > NOW() - INTERVAL '30 days')
SELECT au.id FROM active_users au"#;
        let diags = diagnostics(sql, schema);
        assert!(has_no_warning_containing(&diags, "active_users"),
            "CTE name should not be flagged as unknown table, got: {:?}", diags);
    }

    #[tokio::test]
    async fn pg_d4_recursive_cte_no_false_positive() {
        let schema = pg_schema().await;
        let sql = r#"
WITH RECURSIVE dept_tree AS (
    SELECT id, name, parent_id FROM hr.departments WHERE parent_id IS NULL
    UNION ALL
    SELECT d.id, d.name, d.parent_id
    FROM hr.departments d
    INNER JOIN dept_tree dt ON d.parent_id = dt.id
)
SELECT * FROM dept_tree"#;
        let diags = diagnostics(sql, schema);
        assert!(has_no_warning_containing(&diags, "dept_tree"),
            "Recursive CTE self-reference must not fire warning, got: {:?}", diags);
    }

    #[tokio::test]
    async fn pg_d5_cte_shadowing_real_table_no_false_positive() {
        let schema = pg_schema().await;
        // A CTE named 'users' that shadows the real users table
        let sql = r#"
WITH users AS (
    SELECT id, email, name FROM users WHERE is_active = TRUE
)
SELECT u.id FROM users u"#;
        // The CTE 'users' references 'users' (the real table) in its body —
        // this is valid SQL; the outer users CTE should NOT fire a warning.
        let diags = diagnostics(sql, schema);
        assert!(has_no_warning_containing(&diags, "users"),
            "CTE named users should not produce spurious warning, got: {:?}", diags);
    }

    #[tokio::test]
    async fn pg_d6_chained_ctes_no_false_positives() {
        let schema = pg_schema().await;
        let sql = r#"
WITH
a AS (SELECT id, user_id, total_amount FROM sales.orders WHERE status = 'completed'),
b AS (SELECT a.user_id, SUM(a.total_amount) AS revenue FROM a GROUP BY a.user_id),
c AS (SELECT u.email, b.revenue FROM users u JOIN b ON u.id = b.user_id)
SELECT * FROM c ORDER BY revenue DESC"#;
        let diags = diagnostics(sql, schema);
        // None of a, b, c, users, orders should fire a warning
        for name in &["a", "b", "c", "users", "orders"] {
            assert!(has_no_warning_containing(&diags, name),
                "CTE '{}' should not fire unknown-table warning, got: {:?}", name, diags);
        }
    }

    #[tokio::test]
    async fn pg_d7_lateral_join_no_false_positive() {
        let schema = pg_schema().await;
        let sql = r#"
SELECT u.id, latest.total_amount
FROM users u
CROSS JOIN LATERAL (
    SELECT o.total_amount
    FROM sales.orders o
    WHERE o.user_id = u.id
    ORDER BY o.created_at DESC LIMIT 1
) latest"#;
        let diags = diagnostics(sql, schema);
        assert!(diags.is_empty() || has_no_warning_containing(&diags, "latest"),
            "LATERAL alias should not fire warning, got: {:?}", diags);
    }

    #[tokio::test]
    async fn pg_d8_subquery_alias_not_flagged() {
        let schema = pg_schema().await;
        let sql = r#"
SELECT outer_q.email, outer_q.total
FROM (
    SELECT u.email, SUM(o.total_amount) AS total
    FROM users u
    LEFT JOIN sales.orders o ON u.id = o.user_id
    GROUP BY u.email
) outer_q
ORDER BY outer_q.total DESC"#;
        let diags = diagnostics(sql, schema);
        assert!(has_no_warning_containing(&diags, "outer_q"),
            "Subquery alias should not fire unknown-table warning, got: {:?}", diags);
    }

    // ------------------------------------------------------------------
    // D9: Intentional broken queries MUST produce warnings
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn pg_d9_unknown_table_fires_warning() {
        let schema = pg_schema().await;
        let sql = "SELECT id FROM completely_nonexistent_table_xyz";
        let diags = diagnostics(sql, schema);
        assert!(has_warning(&diags, "completely_nonexistent_table_xyz"),
            "Unknown table must produce a warning, got: {:?}", diags);
    }

    #[tokio::test]
    async fn pg_d10_unknown_table_in_join_fires_warning() {
        let schema = pg_schema().await;
        let sql = "SELECT * FROM users u JOIN ghost_table g ON u.id = g.user_id";
        let diags = diagnostics(sql, schema);
        assert!(has_warning(&diags, "ghost_table"),
            "Unknown table in JOIN must produce warning, got: {:?}", diags);
    }

    // ------------------------------------------------------------------
    // D11-D12: Full mega-query must produce ZERO false positives
    // ------------------------------------------------------------------

    const MEGA_QUERY: &str = r#"
WITH RECURSIVE

dept_tree AS (
    SELECT
        d.id,
        d.name,
        d.parent_id,
        d.budget,
        0 AS depth,
        ARRAY[d.id] AS path
    FROM hr.departments d
    WHERE d.parent_id IS NULL
    UNION ALL
    SELECT
        d.id,
        d.name,
        d.parent_id,
        d.budget,
        dt.depth + 1,
        dt.path || d.id
    FROM hr.departments d
    INNER JOIN dept_tree dt ON d.parent_id = dt.id
    WHERE dt.depth < 10
),

active_employees AS (
    SELECT
        e.id,
        e.user_id,
        e.department_id,
        e.salary,
        e.title,
        e.hired_at,
        dt.name AS dept_name,
        dt.depth AS dept_depth
    FROM hr.employees e
    INNER JOIN dept_tree dt ON e.department_id = dt.id
    WHERE e.is_active = TRUE
),

user_permissions AS (
    SELECT
        ur.user_id,
        ARRAY_AGG(r.name ORDER BY r.name) AS role_names,
        COUNT(ur.role_id)                  AS role_count,
        BOOL_OR(r.name = 'admin')          AS is_admin
    FROM user_roles ur
    INNER JOIN roles r ON ur.role_id = r.id
    GROUP BY ur.user_id
),

order_stats AS (
    SELECT
        o.user_id,
        COUNT(o.id)                                                 AS total_orders,
        SUM(o.total_amount)                                         AS lifetime_value,
        AVG(o.total_amount)                                         AS avg_order_value,
        COUNT(o.id) FILTER (WHERE o.status = 'completed')          AS completed_orders,
        SUM(o.total_amount) FILTER (WHERE o.status = 'completed')  AS completed_revenue,
        MAX(o.created_at)                                           AS last_order_at
    FROM sales.orders o
    GROUP BY o.user_id
),

product_revenue AS (
    SELECT
        p.id          AS product_id,
        p.sku,
        p.name        AS product_name,
        c.name        AS category_name,
        SUM(oi.quantity)                                   AS units_sold,
        SUM(oi.quantity * oi.unit_price * (1 - oi.discount/100)) AS net_revenue
    FROM sales.products p
    LEFT JOIN sales.categories c ON p.category_id = c.id
    LEFT JOIN sales.order_items oi ON p.id = oi.product_id
    LEFT JOIN sales.orders o ON oi.order_id = o.id AND o.status != 'cancelled'
    GROUP BY p.id, p.sku, p.name, c.name
),

inventory_view AS (
    SELECT
        COALESCE(s.product_id, pr.product_id)  AS product_id,
        w.name                                  AS warehouse_name,
        COALESCE(s.quantity, 0)                 AS on_hand,
        COALESCE(s.reserved_quantity, 0)        AS reserved,
        COALESCE(s.quantity, 0)
            - COALESCE(s.reserved_quantity, 0)  AS available,
        pr.net_revenue
    FROM inventory.stock s
    FULL JOIN product_revenue pr ON s.product_id = pr.product_id
    LEFT JOIN inventory.warehouses w ON s.warehouse_id = w.id
),

session_summaries AS (
    SELECT
        e.user_id,
        e.session_id,
        COUNT(e.id)                                    AS event_count,
        MIN(e.created_at)                              AS session_start,
        MAX(e.created_at)                              AS session_end,
        ARRAY_AGG(DISTINCT e.event_type ORDER BY e.event_type) AS event_types
    FROM events e
    WHERE e.created_at >= NOW() - INTERVAL '30 days'
    GROUP BY e.user_id, e.session_id
),

-- This CTE is named 'users' — it intentionally shadows public.users
users AS (
    SELECT
        u.id,
        u.email,
        u.name,
        u.metadata->>'subscription_tier'  AS subscription_tier,
        COALESCE(u.metadata->>'locale', 'en') AS locale,
        ae.salary,
        ae.dept_name,
        COALESCE(up.role_count, 0)         AS role_count,
        COALESCE(up.is_admin, FALSE)       AS is_admin,
        COALESCE(os.total_orders, 0)       AS total_orders,
        COALESCE(os.lifetime_value, 0)     AS lifetime_value
    FROM public.users u
    LEFT JOIN active_employees ae ON u.id  = ae.user_id
    LEFT JOIN user_permissions up ON u.id  = up.user_id
    LEFT JOIN order_stats os       ON u.id = os.user_id
)

SELECT
    u.id,
    u.email,
    u.name,
    u.subscription_tier,
    u.locale,
    u.is_admin,
    u.dept_name,
    u.total_orders,
    u.lifetime_value,

    ROW_NUMBER() OVER (ORDER BY u.lifetime_value DESC NULLS LAST)                   AS value_rank,
    DENSE_RANK() OVER (PARTITION BY u.subscription_tier ORDER BY u.total_orders DESC) AS tier_rank,
    SUM(u.lifetime_value) OVER (PARTITION BY u.subscription_tier)                   AS tier_total_ltv,
    LAG(u.lifetime_value) OVER (PARTITION BY u.dept_name ORDER BY u.lifetime_value DESC) AS prev_ltv,

    (
        SELECT al.operation || ': ' || al.table_name
        FROM audit_log al
        WHERE al.user_id = u.id
        ORDER BY al.created_at DESC
        LIMIT 1
    ) AS last_audit_action,

    EXISTS (
        SELECT 1 FROM session_summaries ss
        WHERE ss.user_id = u.id AND ss.event_count > 10
    ) AS is_power_user,

    CASE
        WHEN u.lifetime_value > 10000 THEN 'platinum'
        WHEN u.lifetime_value > 1000  THEN 'gold'
        WHEN u.lifetime_value > 100   THEN 'silver'
        ELSE 'bronze'
    END AS customer_tier,

    top_products.product_id,
    top_products.net_revenue AS top_product_revenue

FROM users u
LEFT JOIN LATERAL (
    SELECT iv.product_id, iv.net_revenue
    FROM inventory_view iv
    ORDER BY iv.net_revenue DESC NULLS LAST
    LIMIT 1
) top_products ON TRUE

WHERE u.total_orders >= 0
  AND u.email NOT ILIKE '%@spam.%'
  AND u.locale IN ('en', 'fr', 'de', 'es')
  AND (u.is_admin = TRUE OR u.lifetime_value > 0)

ORDER BY value_rank ASC, u.lifetime_value DESC NULLS LAST
LIMIT 100 OFFSET 0
"#;

    #[tokio::test]
    async fn pg_d11_mega_query_zero_false_positives() {
        let schema = pg_schema().await;
        let diags = diagnostics(MEGA_QUERY, schema);
        let warnings: Vec<_> = diags.iter()
            .filter(|d| d.severity == DiagSeverity::Warning)
            .collect();
        assert!(warnings.is_empty(),
            "Mega query should produce 0 false-positive warnings, got: {:?}", warnings);
    }

    #[tokio::test]
    async fn pg_d12_mega_query_with_injected_unknown_table_fires_warning() {
        let schema = pg_schema().await;
        // Inject an unknown table reference into the mega query
        let broken = MEGA_QUERY.replace(
            "FROM hr.departments d",
            "FROM definitely_not_a_real_table d",
        );
        let diags = diagnostics(&broken, schema);
        assert!(has_warning(&diags, "definitely_not_a_real_table"),
            "Injected unknown table must produce warning, got: {:?}", diags);
    }
}
```

- [ ] **Step 4: Run diagnostic tests**

```bash
cd src-tauri && cargo test --lib "pg_diagnostic_tests" -- --nocapture 2>&1 | tail -20
```
Expected:
```
test completion::pg_integration_tests::pg_diagnostic_tests::pg_d1_clean_simple_query_no_warnings ... ok
...
test completion::pg_integration_tests::pg_diagnostic_tests::pg_d12_mega_query_with_injected_unknown_table_fires_warning ... ok
test result: ok. 12 passed; 0 failed
```

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/completion/pg_integration_tests.rs
git commit -m "test(completion): add 12 live-schema diagnostic integration tests + mega-query fixture"
```

---

## Task 4: Extension + edge-case tests (JSONB, pg_trgm, arrays, LTREE, PostgreSQL syntax)

**Files:**
- Modify: `src-tauri/src/completion/pg_integration_tests.rs` (add `pg_extension_tests` module)

- [ ] **Step 1: Write failing stub**

Add to `pg_integration_tests.rs`:
```rust
#[cfg(test)]
mod pg_extension_tests {
    #[tokio::test]
    async fn pg_ext_placeholder() {
        panic!("extension tests not yet implemented");
    }
}
```

- [ ] **Step 2: Verify it fails**

```bash
cd src-tauri && cargo test --lib "pg_ext_placeholder" 2>&1 | tail -5
```

- [ ] **Step 3: Replace stub with full extension test module**

```rust
#[cfg(test)]
mod pg_extension_tests {
    use super::*;
    use sql_scope::{run_diagnostics, DiagSeverity};

    fn diagnostics(sql: &str, schema: &SchemaGraph) -> Vec<sql_scope::ScopeDiagnostic> {
        let scope_tree = sql_scope::resolve(sql, sql_scope::Dialect::Postgres, schema)
            .unwrap_or_default();
        run_diagnostics(&scope_tree, schema, sql)
    }

    // ------------------------------------------------------------------
    // JSONB operators: ->, ->>, @>, ?, ?|, ?&
    // Parser must not crash; columns visible after the expression
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn pg_ext1_jsonb_arrow_operator_no_crash() {
        let schema = pg_schema().await;
        // -> and ->> operators in WHERE — parser must survive, diagnostics clean
        let sql = r#"
SELECT u.id, u.metadata->>'subscription_tier' AS tier
FROM users u
WHERE u.metadata->>'locale' = 'en'
  AND u.metadata @> '{"active": true}'::jsonb"#;
        let diags = diagnostics(sql, schema);
        assert!(diags.iter().all(|d| d.severity != DiagSeverity::Warning),
            "JSONB operators should not produce warnings, got: {:?}", diags);
    }

    #[tokio::test]
    async fn pg_ext2_jsonb_in_cte_no_false_positive() {
        let schema = pg_schema().await;
        // CTE that uses jsonb_build_object and -> operators
        let sql = r#"
WITH json_summary AS (
    SELECT
        u.id,
        u.email,
        jsonb_build_object(
            'name', u.name,
            'locale', u.metadata->>'locale',
            'tags', u.tags
        ) AS summary
    FROM users u
    WHERE u.metadata ? 'subscription_tier'
)
SELECT js.id, js.email, js.summary FROM json_summary js"#;
        let diags = diagnostics(sql, schema);
        assert!(diags.iter().all(|d| d.severity != DiagSeverity::Warning),
            "jsonb_build_object CTE should produce no warnings, got: {:?}", diags);
    }

    #[tokio::test]
    async fn pg_ext3_jsonb_dot_completion_in_cte_body() {
        let schema = pg_schema().await;
        // Cursor at u. inside a CTE body that also has JSONB expressions
        let sql = r#"
WITH enriched AS (
    SELECT u.|
    FROM users u
    WHERE u.metadata @> '{"premium": true}'::jsonb
)
SELECT * FROM enriched"#;
        let labels = complete_labels(sql, schema);
        assert!(has_labels(&labels, &["id", "email", "name", "metadata", "tags"]),
            "JSONB WHERE clause should not break column completion, got: {:?}", labels);
    }

    // ------------------------------------------------------------------
    // Array operators: @>, &&, = ANY(...)
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn pg_ext4_array_operators_no_crash() {
        let schema = pg_schema().await;
        let sql = r#"
SELECT u.id, u.email, u.tags
FROM users u
WHERE u.tags @> ARRAY['premium']
  AND u.tags && ARRAY['active', 'verified']
  AND 'admin' = ANY(u.tags)"#;
        let diags = diagnostics(sql, schema);
        assert!(diags.iter().all(|d| d.severity != DiagSeverity::Warning),
            "Array operators should produce no warnings, got: {:?}", diags);
    }

    #[tokio::test]
    async fn pg_ext5_array_in_cte_body_completion_works() {
        let schema = pg_schema().await;
        let sql = r#"
WITH tagged_users AS (
    SELECT u.id, u.email, array_length(u.tags, 1) AS tag_count
    FROM users u
    WHERE u.tags && ARRAY['premium']
)
SELECT tu.| FROM tagged_users tu"#;
        let labels = complete_labels(sql, schema);
        assert!(has_labels(&labels, &["id", "email", "tag_count"]),
            "CTE with array ops should project columns correctly, got: {:?}", labels);
    }

    // ------------------------------------------------------------------
    // pg_trgm: % operator, similarity(), word_similarity()
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn pg_ext6_trgm_similarity_in_where_no_crash() {
        let schema = pg_schema().await;
        let sql = r#"
SELECT p.id, p.name, similarity(p.name, 'laptop') AS score
FROM sales.products p
WHERE p.name % 'laptop'
ORDER BY score DESC"#;
        let diags = diagnostics(sql, schema);
        assert!(diags.iter().all(|d| d.severity != DiagSeverity::Warning),
            "pg_trgm % operator should not crash or warn, got: {:?}", diags);
    }

    #[tokio::test]
    async fn pg_ext7_trgm_inside_cte_completion_works() {
        let schema = pg_schema().await;
        // CTE uses similarity(); cursor is in final SELECT at alias.
        let sql = r#"
WITH fuzzy_products AS (
    SELECT p.id, p.name, p.price, similarity(p.name, 'phone') AS match_score
    FROM sales.products p
    WHERE p.name % 'phone'
)
SELECT fp.| FROM fuzzy_products fp"#;
        let labels = complete_labels(sql, schema);
        assert!(has_labels(&labels, &["id", "name", "price", "match_score"]),
            "CTE with trgm should project columns correctly, got: {:?}", labels);
    }

    // ------------------------------------------------------------------
    // LTREE: @>, <@, ~ (lquery), ? (array of lqueries)
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn pg_ext8_ltree_operators_no_crash() {
        let schema = pg_schema().await;
        let sql = r#"
SELECT c.id, c.name, c.path
FROM sales.categories c
WHERE c.path <@ 'Electronics'::ltree
  AND c.path ~ '*.Phones.*'"#;
        let diags = diagnostics(sql, schema);
        assert!(diags.iter().all(|d| d.severity != DiagSeverity::Warning),
            "LTREE operators should not crash or warn, got: {:?}", diags);
    }

    #[tokio::test]
    async fn pg_ext9_ltree_inside_cte_completion_works() {
        let schema = pg_schema().await;
        let sql = r#"
WITH phone_categories AS (
    SELECT c.id, c.name, c.path, c.parent_id
    FROM sales.categories c
    WHERE c.path ~ '*.Phones.*'
)
SELECT pc.| FROM phone_categories pc"#;
        let labels = complete_labels(sql, schema);
        assert!(has_labels(&labels, &["id", "name", "path", "parent_id"]),
            "LTREE CTE should project columns, got: {:?}", labels);
    }

    // ------------------------------------------------------------------
    // PostgreSQL-specific syntax: DISTINCT ON, FETCH FIRST,
    // FOR UPDATE, RETURNING, FILTER(...), WITHIN GROUP
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn pg_ext10_distinct_on_no_crash() {
        let schema = pg_schema().await;
        let sql = r#"
SELECT DISTINCT ON (o.user_id)
    o.user_id, o.id AS order_id, o.total_amount, o.created_at
FROM sales.orders o
ORDER BY o.user_id, o.created_at DESC"#;
        let diags = diagnostics(sql, schema);
        assert!(diags.iter().all(|d| d.severity != DiagSeverity::Warning),
            "DISTINCT ON should not produce warnings, got: {:?}", diags);
    }

    #[tokio::test]
    async fn pg_ext11_filter_aggregate_no_crash() {
        let schema = pg_schema().await;
        let sql = r#"
SELECT
    o.user_id,
    COUNT(o.id)                                           AS total,
    COUNT(o.id) FILTER (WHERE o.status = 'completed')    AS completed,
    SUM(o.total_amount) FILTER (WHERE o.status != 'cancelled') AS revenue
FROM sales.orders o
GROUP BY o.user_id"#;
        let diags = diagnostics(sql, schema);
        assert!(diags.iter().all(|d| d.severity != DiagSeverity::Warning),
            "FILTER aggregate should not produce warnings, got: {:?}", diags);
    }

    #[tokio::test]
    async fn pg_ext12_percentile_within_group_no_crash() {
        let schema = pg_schema().await;
        let sql = r#"
SELECT
    o.user_id,
    PERCENTILE_CONT(0.5) WITHIN GROUP (ORDER BY o.total_amount) AS median_order,
    PERCENTILE_DISC(0.9) WITHIN GROUP (ORDER BY o.total_amount) AS p90_order
FROM sales.orders o
GROUP BY o.user_id"#;
        let diags = diagnostics(sql, schema);
        assert!(diags.iter().all(|d| d.severity != DiagSeverity::Warning),
            "WITHIN GROUP aggregate should not produce warnings, got: {:?}", diags);
    }

    #[tokio::test]
    async fn pg_ext13_all_window_functions_in_cte_no_crash() {
        let schema = pg_schema().await;
        let sql = r#"
WITH ranked_orders AS (
    SELECT
        o.id,
        o.user_id,
        o.total_amount,
        o.created_at,
        ROW_NUMBER()   OVER (PARTITION BY o.user_id ORDER BY o.created_at DESC) AS rn,
        RANK()         OVER (PARTITION BY o.user_id ORDER BY o.total_amount DESC) AS amt_rank,
        DENSE_RANK()   OVER (ORDER BY o.total_amount DESC) AS global_rank,
        PERCENT_RANK() OVER (ORDER BY o.total_amount)      AS pct_rank,
        NTILE(4)       OVER (ORDER BY o.total_amount)      AS quartile,
        LAG(o.total_amount, 1, 0)  OVER (PARTITION BY o.user_id ORDER BY o.created_at) AS prev_order,
        LEAD(o.total_amount, 1, 0) OVER (PARTITION BY o.user_id ORDER BY o.created_at) AS next_order,
        FIRST_VALUE(o.total_amount) OVER (PARTITION BY o.user_id ORDER BY o.created_at
            ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW) AS first_order,
        SUM(o.total_amount) OVER (PARTITION BY o.user_id
            ROWS BETWEEN 2 PRECEDING AND CURRENT ROW) AS rolling_3
    FROM sales.orders o
)
SELECT ro.| FROM ranked_orders ro WHERE ro.rn = 1"#;
        let diags = diagnostics(sql, schema);
        assert!(diags.iter().all(|d| d.severity != DiagSeverity::Warning),
            "Window functions in CTE should not produce warnings, got: {:?}", diags);
        let labels = complete_labels(sql, schema);
        assert!(has_labels(&labels,
            &["id", "user_id", "total_amount", "rn", "amt_rank", "global_rank",
              "prev_order", "next_order", "rolling_3"]),
            "Window function CTE should project all named columns, got: {:?}", labels);
    }

    #[tokio::test]
    async fn pg_ext14_union_all_cte_no_false_positive() {
        let schema = pg_schema().await;
        let sql = r#"
WITH all_activity AS (
    SELECT user_id, created_at, 'order' AS activity_type
    FROM sales.orders
    UNION ALL
    SELECT user_id, created_at, event_type AS activity_type
    FROM events
    UNION ALL
    SELECT user_id, created_at, operation AS activity_type
    FROM audit_log
)
SELECT aa.user_id, aa.created_at, aa.activity_type
FROM all_activity aa
ORDER BY aa.created_at DESC"#;
        let diags = diagnostics(sql, schema);
        assert!(diags.iter().all(|d| d.severity != DiagSeverity::Warning),
            "UNION ALL CTE should not produce warnings, got: {:?}", diags);
    }

    #[tokio::test]
    async fn pg_ext15_cross_schema_multi_join_completion() {
        let schema = pg_schema().await;
        // Deep multi-schema query: cursor at stock alias after 4-table join
        let sql = r#"
SELECT s.|
FROM hr.employees e
JOIN users u ON e.user_id = u.id
JOIN sales.orders o ON o.user_id = u.id
JOIN sales.order_items oi ON oi.order_id = o.id
JOIN sales.products p ON oi.product_id = p.id
JOIN inventory.stock s ON s.product_id = p.id"#;
        let labels = complete_labels(sql, schema);
        assert!(has_labels(&labels, &["id", "product_id", "warehouse_id", "quantity", "reserved_quantity"]),
            "Cross-schema 6-table join: should see inventory.stock columns, got: {:?}", labels);
    }
}
```

- [ ] **Step 4: Run extension tests**

```bash
cd src-tauri && cargo test --lib "pg_extension_tests" -- --nocapture 2>&1 | tail -20
```
Expected:
```
test completion::pg_integration_tests::pg_extension_tests::pg_ext1_jsonb_arrow_operator_no_crash ... ok
...
test result: ok. 15 passed; 0 failed
```

- [ ] **Step 5: Run the full test suite to confirm nothing regressed**

```bash
cd src-tauri && cargo test --lib 2>&1 | grep "test result"
```
Expected:
```
test result: ok. 247 passed; 0 failed; 0 ignored
```
(200 existing + 4 helpers + 20 completion + 12 diagnostic + 15 extension = 251 total; exact count will vary slightly based on async test expansion.)

- [ ] **Step 6: Final commit**

```bash
git add src-tauri/src/completion/pg_integration_tests.rs
git commit -m "test(completion): add 47 extension/edge-case integration tests (JSONB, trgm, ltree, window fns, UNION ALL)"
```

---

## Self-Review

**1. Spec coverage:**
- ✅ Real PostgreSQL connection (Task 1)
- ✅ Multi-schema fixture with 12 tables (Task 1)
- ✅ Extensions: uuid-ossp, pg_trgm, ltree, citext installed (Task 1)
- ✅ All JOIN types: INNER, LEFT, RIGHT, FULL, CROSS, LATERAL (Tasks 2+3)
- ✅ N-level CTE nesting: up to 8-deep chain (Tasks 2+3)
- ✅ CTE shadowing real table (Tasks 2+3)
- ✅ Recursive CTE (Tasks 2+3)
- ✅ Window functions in CTEs (Task 4)
- ✅ JSONB operators in CTE body + WHERE (Task 4)
- ✅ pg_trgm `%` / `similarity()` (Task 4)
- ✅ LTREE `<@` / `~` (Task 4)
- ✅ Array operators `@>`, `&&`, `ANY()` (Task 4)
- ✅ DISTINCT ON, FILTER, WITHIN GROUP (Task 4)
- ✅ UNION ALL CTE (Task 4)
- ✅ Correlated subquery (Task 2 pg_c2)
- ✅ LATERAL subquery (Tasks 2+3)
- ✅ Zero false-positive diagnostic coverage (Task 3)
- ✅ Injected unknown-table fires correct warning (Task 3)
- ✅ Mega-query as single fixture for both completion + diagnostics (Task 3)

**2. Placeholder scan:** No TBD, TODO, or incomplete steps found.

**3. Type consistency:**
- `build_pg_test_schema()` → `SchemaGraph` ✅ used in all tasks
- `complete_labels(sql, schema)` → `Vec<String>` ✅ consistent
- `complete_items(sql, schema)` → `Vec<CompletionItem>` ✅ consistent
- `diagnostics(sql, schema)` → `Vec<ScopeDiagnostic>` ✅ consistent
- `has_labels`, `lacks_labels`, `has_warning`, `has_no_warning_containing` — all defined in same scope ✅
