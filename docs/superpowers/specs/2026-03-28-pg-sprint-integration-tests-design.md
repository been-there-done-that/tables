# PostgreSQL Sprint Integration Tests — Design Spec

**Date:** 2026-03-28
**Branch:** ai
**Scope:** Integration tests for the 11 new PostgreSQL introspection + DDL commands added in the feature sprint

---

## Goal

Verify that every SQL query inside the new PostgreSQL browse and DDL commands executes correctly against a real PostgreSQL database and returns correctly shaped data. Tests run against a local Postgres instance with a known test schema.

---

## Approach

Single test file: `src-tauri/src/commands/pg_sprint_tests.rs`

Follows the existing `postgres_diagnostic_test.rs` style:
- Raw `tokio_postgres::Client`, no ORM
- `DATABASE_URL` env var with default `postgres://postgres:postgres@localhost:5432/postgres`
- `tokio::spawn` for the background connection task
- Graceful skip (not panic) if Postgres is unreachable

One master test function `test_pg_sprint_introspection` that:
1. Connects to Postgres
2. Creates schema `tables_test` with all required objects (`DROP SCHEMA IF EXISTS … CASCADE` first for idempotency)
3. Runs each command's SQL query inline (exact same SQL as in the command)
4. Asserts key properties
5. Drops schema on completion

---

## Test Schema: `tables_test`

All objects created in the `tables_test` schema.

### Table: `users`

```sql
CREATE TABLE tables_test.users (
    id      BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    email   TEXT NOT NULL,
    age     INT,
    slug    TEXT GENERATED ALWAYS AS (lower(email)) STORED,
    CONSTRAINT users_email_unique UNIQUE (email),
    CONSTRAINT users_age_positive CHECK (age > 0)
);
```

Covers: identity column, stored generated column, UNIQUE constraint, CHECK constraint.

### Index: `idx_users_age_partial`

```sql
CREATE INDEX idx_users_age_partial ON tables_test.users(age) WHERE age > 18;
```

Covers: partial btree index with a predicate.

### View: `active_adults`

```sql
CREATE VIEW tables_test.active_adults AS
    SELECT id, email, age FROM tables_test.users WHERE age > 18;
```

### Materialized View: `user_count`

```sql
CREATE MATERIALIZED VIEW tables_test.user_count AS
    SELECT COUNT(*) AS total FROM tables_test.users
WITH DATA;
```

### Sequence: `custom_id_seq`

```sql
CREATE SEQUENCE tables_test.custom_id_seq
    AS bigint
    START WITH 100
    INCREMENT BY 5
    MINVALUE 1
    MAXVALUE 9999999
    NO CYCLE;
```

Covers: `data_type::text` cast correctness, quoted identifier DDL.

### Function: `get_user_email`

```sql
CREATE OR REPLACE FUNCTION tables_test.get_user_email(user_id bigint)
RETURNS text
LANGUAGE plpgsql AS $$
BEGIN
    RETURN (SELECT email FROM tables_test.users WHERE id = user_id);
END;
$$;
```

### Procedure: `mark_user_active`

```sql
CREATE OR REPLACE PROCEDURE tables_test.mark_user_active(user_id bigint)
LANGUAGE plpgsql AS $$
BEGIN
    UPDATE tables_test.users SET age = age WHERE id = user_id;
END;
$$;
```

### Trigger support function + trigger: `users_audit`

```sql
CREATE OR REPLACE FUNCTION tables_test.log_user_change()
RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    RETURN NEW;
END;
$$;

CREATE TRIGGER users_audit
AFTER INSERT OR UPDATE ON tables_test.users
FOR EACH ROW EXECUTE FUNCTION tables_test.log_user_change();
```

---

## Per-Command Assertions

| # | Command SQL source | What is asserted |
|---|---|---|
| 1 | `get_functions` (pg_proc) | Row with `proname = 'get_user_email'` and prokind `'f'`; row with `proname = 'mark_user_active'` and prokind `'p'` |
| 2 | `get_sequences` (pg_sequences) | `custom_id_seq` row has `data_type = 'bigint'` (proves `::text` cast is correct) |
| 3 | `get_constraints` (pg_constraint) | Two rows on `users`: one `contype = 'u'` (UNIQUE), one `contype = 'c'` (CHECK) |
| 4 | `get_index_details` (pg_index + pg_am) | `idx_users_age_partial` has `amname = 'btree'` and non-null `indpred` (partial) |
| 5 | `get_table_ddl` (assembled) | Output string contains `"email"`, `CHECK`, `GENERATED ALWAYS AS IDENTITY`, `GENERATED ALWAYS AS` |
| 6 | `get_view_definition` (pg_views) | Output contains `CREATE OR REPLACE VIEW` and `active_adults` |
| 7 | `get_matview_definition` (pg_matviews) | Output contains `CREATE MATERIALIZED VIEW` and `WITH DATA` |
| 8 | `get_function_ddl` (pg_get_functiondef) | Output contains `CREATE OR REPLACE FUNCTION` and `get_user_email` |
| 9 | `get_sequence_ddl` (assembled) | Output contains `"tables_test"."custom_id_seq"` (double-quoted identifiers) |
| 10 | `get_index_ddl` (pg_indexes.indexdef) | Output contains `CREATE INDEX` and `idx_users_age_partial` |
| 11 | `get_trigger_definition` (pg_get_triggerdef) | Output contains `TRIGGER users_audit` and `AFTER INSERT OR UPDATE` |

---

## Connection

```rust
let conn_str = std::env::var("DATABASE_URL")
    .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/postgres".to_string());
```

If connection fails → print message and return (no panic, test is skipped in CI without Postgres).

---

## Files Affected

- **Create:** `src-tauri/src/commands/pg_sprint_tests.rs`
- **Modify:** `src-tauri/src/commands/mod.rs` — add `mod pg_sprint_tests;`

---

## Run Command

```bash
cargo test test_pg_sprint_introspection -- --nocapture
```

Or with explicit URL:

```bash
DATABASE_URL=postgres://myuser:mypass@localhost:5432/mydb \
  cargo test test_pg_sprint_introspection -- --nocapture
```

---

## Out of Scope

- TLS connection testing
- Partitioned tables
- EXCLUSION constraints
- Overloaded function testing
- Testcontainers / Docker auto-spin-up
