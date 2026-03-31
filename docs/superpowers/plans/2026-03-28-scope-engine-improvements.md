# Scope Engine Improvements Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix four gaps found during the PostgreSQL integration test suite: recursive CTE UNION ALL column projection (highest impact), a production FK query typo that silently drops FK columns, EXISTS subquery outer alias propagation, and hardening test assertions that were weakened while waiting for these fixes.

**Architecture:** The fixes are independent and ordered by priority. Task 1 (Postgres parser) fixes the root cause for recursive CTEs. Task 2 (FK typo) is a one-line production bug fix. Task 3 strengthens tests that were weakened pending Tasks 1 and 2. Task 4 (EXISTS) requires teaching the scope resolver to emit child scopes for WHERE-clause subqueries.

**Tech Stack:** Rust, `pg_query` protobuf (Postgres parser), `sqlparser` (SQLite parser), `sql-scope` crate, `tokio-postgres` (FK query), existing test infrastructure in `src-tauri/src/completion/tests.rs` and `src-tauri/src/completion/pg_integration_tests.rs`.

---

## Background: Key Files

| File | Role |
|------|------|
| `src-tauri/crates/sql-scope/src/parser/postgres.rs` | `convert_cte()` — where UNION ALL body is incorrectly extracted |
| `src-tauri/crates/sql-scope/src/parser/sqlite.rs` | `convert_set_expr()` — already correctly handles UNION ALL (no change needed) |
| `src-tauri/crates/sql-scope/src/scope/cte.rs` | `resolve_cte_columns()` / `project_from_body()` — works correctly once body is right |
| `src-tauri/crates/sql-scope/src/scope/resolver.rs` | `process_ctes()` — registers CTE sources; needs to handle EXISTS subqueries |
| `src-tauri/crates/sql-scope/src/ir.rs` | `CteIr`, `SelectBodyIr`, `TableRefIr` — IR structs |
| `src-tauri/src/adapters/postgres.rs` | `list_foreign_keys_schema()` — typo: `cols.src_attnum` |
| `src-tauri/src/completion/tests.rs` | Unit tests including T25 (weakened recursive CTE assertion) |
| `src-tauri/src/completion/pg_integration_tests.rs` | `pg_b5` and `pg_d12` (weakened assertions) |

---

## Task 1: Fix Postgres parser — recursive CTE anchor extraction

**The bug:** `convert_cte()` in `postgres.rs` calls `convert_select_body(inner_sel, sql)` directly on the CTE's inner `SelectStmt`. For `WITH RECURSIVE … UNION ALL`, pg_query represents the body as a `SelectStmt` with `op ≠ 0` (SetOperation::Union) whose `from_clause` and `target_list` are empty — the actual anchor SELECT lives in `larg`. So `SelectBodyIr` ends up with empty `from` and `select_list`, making `project_from_body` return `[]` and `CteInfo.columns` stay empty.

**The fix:** In `convert_cte`, detect `inner_sel.op != 0` and descend into `inner_sel.larg` to get the anchor branch.

**Note:** The SQLite parser already handles this correctly via `convert_set_expr → SetExpr::SetOperation { left, .. } => convert_set_expr(*left)`. No change needed there.

**Files:**
- Modify: `src-tauri/crates/sql-scope/src/parser/postgres.rs` (function `convert_cte`, lines 74–121)
- Test via: `src-tauri/crates/sql-scope/` — run the sql-scope crate tests

- [ ] **Step 1: Write the failing test in sql-scope**

Add to `src-tauri/crates/sql-scope/src/parser/postgres.rs` inside `#[cfg(test)] mod tests`:

```rust
#[test]
fn test_recursive_cte_union_all_anchor_columns() {
    use crate::ir::{ParsedStatement, SelectItemIr};
    let sql = r#"WITH RECURSIVE dept_tree AS (
    SELECT id, name, parent_id, 0 AS depth
    FROM departments
    WHERE parent_id IS NULL
    UNION ALL
    SELECT d.id, d.name, d.parent_id, dt.depth + 1
    FROM departments d
    INNER JOIN dept_tree dt ON d.parent_id = dt.id
)
SELECT * FROM dept_tree"#;
    let parsed = super::parse_postgres(sql).expect("parse failed");
    let ParsedStatement::Select(sel) = parsed else { panic!("expected Select") };
    let with = sel.with.expect("expected WITH clause");
    assert_eq!(with.ctes.len(), 1);
    let cte = &with.ctes[0];
    assert_eq!(cte.name, "dept_tree");
    // The anchor SELECT projects: id, name, parent_id, depth (alias on 0)
    let aliases: Vec<Option<&str>> = cte.body.select_list.iter().map(|item| {
        if let SelectItemIr::Expr { alias, .. } = item { alias.as_deref() } else { None }
    }).collect();
    assert!(aliases.contains(&Some("id")),     "anchor should project 'id', got {:?}", aliases);
    assert!(aliases.contains(&Some("name")),   "anchor should project 'name', got {:?}", aliases);
    assert!(aliases.contains(&Some("depth")),  "anchor should project 'depth', got {:?}", aliases);
    // FROM clause should see departments
    assert!(!cte.body.from.is_empty(), "anchor FROM should not be empty");
}
```

- [ ] **Step 2: Run to verify it fails**

```bash
cd src-tauri/crates/sql-scope && cargo test "test_recursive_cte_union_all_anchor_columns" -- --nocapture 2>&1 | tail -10
```

Expected: FAIL — `aliases` is empty because `cte.body.select_list` is empty.

- [ ] **Step 3: Fix `convert_cte` in `postgres.rs`**

Replace the `body` extraction block (lines 94–112) with:

```rust
    // Extract the inner SelectStmt body.
    // For UNION ALL recursive CTEs, pg_query represents the body as a SelectStmt
    // with op != 0 (SetOperation::Union/Intersect/Except) — the anchor SELECT
    // is in `larg`. We use the anchor branch so CTE column projection works.
    let body = cte
        .ctequery
        .as_ref()
        .and_then(|q| q.node.as_ref())
        .and_then(|n| {
            if let NodeEnum::SelectStmt(inner_sel) = n {
                // op == 0 means plain SELECT; op != 0 means set operation — use larg (anchor)
                let body_sel: &pg_query::protobuf::SelectStmt = if inner_sel.op != 0 {
                    inner_sel
                        .larg
                        .as_ref()
                        .and_then(|node| node.node.as_ref())
                        .and_then(|n| {
                            if let NodeEnum::SelectStmt(s) = n { Some(s) } else { None }
                        })
                        .unwrap_or(inner_sel)
                } else {
                    inner_sel
                };
                Some(Box::new(convert_select_body(body_sel, sql)))
            } else {
                None
            }
        })
        .unwrap_or_else(|| {
            Box::new(SelectBodyIr {
                from: vec![],
                select_list: vec![],
                byte_range: 0..sql.len(),
            })
        });
```

- [ ] **Step 4: Run the new test to verify it passes**

```bash
cd src-tauri/crates/sql-scope && cargo test "test_recursive_cte_union_all_anchor_columns" -- --nocapture 2>&1 | tail -10
```

Expected: PASS.

- [ ] **Step 5: Run the full sql-scope test suite to confirm no regressions**

```bash
cd src-tauri/crates/sql-scope && cargo test 2>&1 | grep "test result"
```

Expected: `test result: ok. N passed; 0 failed` (N ≥ 380, exact number depends on current count).

- [ ] **Step 6: Run the full lib tests to confirm completion engine still passes**

```bash
cd src-tauri && cargo test --lib 2>&1 | grep "test result"
```

Expected: `test result: ok. 251 passed; 0 failed`.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/crates/sql-scope/src/parser/postgres.rs
git commit -m "fix(parser): extract UNION ALL anchor branch for recursive CTE body in Postgres parser"
```

---

## Task 2: Fix `list_foreign_keys_schema` column alias typo

**The bug:** `src-tauri/src/adapters/postgres.rs`, function `list_foreign_keys_schema` (around line 891). The LATERAL unnest aliases are declared as `cols(attnum, ref_attnum, ordinality)` on line 911, but line 912 joins with `cols.src_attnum` — a name that does not exist in the alias list. This query would fail at runtime for any schema-wide FK introspection.

**Files:**
- Modify: `src-tauri/src/adapters/postgres.rs` (line 912)

- [ ] **Step 1: Locate the bug**

Read `src-tauri/src/adapters/postgres.rs` lines 905–920. Verify:
- Line 911: `CROSS JOIN LATERAL unnest(con.conkey, con.confkey) WITH ORDINALITY AS cols(attnum, ref_attnum, ordinality)`
- Line 912: `JOIN pg_attribute src_att ON src_att.attrelid = tab.oid AND src_att.attnum = cols.src_attnum`

The alias is `attnum` but the join references `cols.src_attnum`.

- [ ] **Step 2: Fix the typo**

Change line 912 from:
```sql
            JOIN pg_attribute src_att ON src_att.attrelid = tab.oid AND src_att.attnum = cols.src_attnum
```
to:
```sql
            JOIN pg_attribute src_att ON src_att.attrelid = tab.oid AND src_att.attnum = cols.attnum
```

- [ ] **Step 3: Compile to verify no type errors**

```bash
cd src-tauri && cargo build 2>&1 | grep -E "^error" | head -10
```

Expected: no errors (this is a string constant change; compile confirms syntax).

- [ ] **Step 4: Run lib tests to confirm no regressions**

```bash
cd src-tauri && cargo test --lib 2>&1 | grep "test result"
```

Expected: `test result: ok. 251 passed; 0 failed`.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/adapters/postgres.rs
git commit -m "fix(postgres): fix cols.src_attnum → cols.attnum in list_foreign_keys_schema LATERAL unnest"
```

---

## Task 3: Strengthen weakened test assertions

Three tests were deliberately weakened because they depended on the recursive CTE parser bug fixed in Task 1. With that fix in place, each should now have a full assertion.

**Files:**
- Modify: `src-tauri/src/completion/tests.rs` (T25)
- Modify: `src-tauri/src/completion/pg_integration_tests.rs` (pg_b5, pg_d12)

- [ ] **Step 1: Verify T25 now returns anchor columns**

Run T25 with output to see what the engine actually returns after the parser fix:

```bash
cd src-tauri && cargo test --lib "t25_recursive_cte_anchor_select" -- --nocapture 2>&1 | tail -10
```

Expected output will show `T25 actual completions (UNION-ALL CTE limitation): N items` where N > 0 and includes `FirstName`, `LastName`, `EmployeeId`, `ReportsTo`, `Level`.

- [ ] **Step 2: Replace T25 assertion in `tests.rs`**

Find the test `t25_recursive_cte_anchor_select` in `src-tauri/src/completion/tests.rs`. Replace the body with a strong assertion:

```rust
#[test]
fn t25_recursive_cte_anchor_select() {
    let query = "
WITH RECURSIVE employee_hierarchy AS (
   SELECT EmployeeId, FirstName, LastName, ReportsTo, 0 AS Level |
   FROM employees
   WHERE ReportsTo IS NULL
   UNION ALL
   SELECT e.EmployeeId, e.FirstName, e.LastName, e.ReportsTo, eh.Level + 1
   FROM employees e
   JOIN employee_hierarchy eh ON e.ReportsTo = eh.EmployeeId
)
SELECT * FROM employee_hierarchy
    ";

    let items = complete(query);
    // Anchor SELECT projects: EmployeeId, FirstName, LastName, ReportsTo, Level
    // After the parser fix these must now be visible at the cursor.
    let lower: Vec<String> = items.iter().map(|s| s.to_lowercase()).collect();
    assert!(lower.contains(&"employeeid".to_string()), "Expected EmployeeId in suggestions, got {:?}", items);
    assert!(lower.contains(&"firstname".to_string()),  "Expected FirstName in suggestions, got {:?}", items);
    assert!(lower.contains(&"level".to_string()),      "Expected Level in suggestions, got {:?}", items);
    // No duplicates
    let count_firstname = lower.iter().filter(|s| s.as_str() == "firstname").count();
    assert_eq!(count_firstname, 1, "FirstName should appear exactly once, got {:?}", items);
}
```

- [ ] **Step 3: Run T25 to verify it passes**

```bash
cd src-tauri && cargo test --lib "t25_recursive_cte_anchor_select" -- --nocapture 2>&1 | tail -5
```

Expected: PASS. If it fails (columns still empty), the resolver may need to be told the CTE is recursive — see the note at the bottom of this task.

- [ ] **Step 4: Verify pg_b5 now returns anchor columns**

Run pg_b5 with output to see what the engine returns:

```bash
cd src-tauri && cargo test --lib "pg_completion_tests::pg_b5" -- --nocapture 2>&1 | tail -10
```

Expected: labels include `id`, `name`, `parent_id`, `depth`.

- [ ] **Step 5: Replace pg_b5 assertion in `pg_integration_tests.rs`**

Find `pg_b5_recursive_cte_columns` in `src-tauri/src/completion/pg_integration_tests.rs`. Replace the body:

```rust
#[tokio::test]
async fn pg_b5_recursive_cte_columns() {
    let schema = pg_schema().await;
    let sql = r#"WITH RECURSIVE dept_tree AS (
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
    assert!(has_labels(&labels, &["id", "name", "parent_id", "depth"]),
        "Recursive CTE should expose anchor projected columns, got: {:?}", labels);
}
```

- [ ] **Step 6: Replace pg_d12 assertion in `pg_integration_tests.rs`**

Find `pg_d12_mega_query_with_injected_unknown_table_fires_warning`. Replace the body with a hard assertion (the `match Ok/Err` pattern is no longer needed):

```rust
#[tokio::test]
async fn pg_d12_mega_query_with_injected_unknown_table_fires_warning() {
    let schema = pg_schema().await;
    let broken = MEGA_QUERY.replace(
        "FROM hr.departments d\n    WHERE d.parent_id IS NULL",
        "FROM definitely_not_a_real_table d\n    WHERE d.parent_id IS NULL",
    );
    let diags = diagnostics(&broken, schema);
    assert!(
        diags.iter().any(|d| d.message.contains("definitely_not_a_real_table") && d.severity == DiagSeverity::Warning),
        "Injected unknown table must produce a warning, got: {:?}", diags
    );
}
```

- [ ] **Step 7: Run all three strengthened tests**

```bash
cd src-tauri && cargo test --lib "t25_recursive_cte\|pg_b5\|pg_d12" -- --nocapture 2>&1 | tail -10
```

Expected: all 3 pass.

- [ ] **Step 8: Run the full lib test suite**

```bash
cd src-tauri && cargo test --lib 2>&1 | grep "test result"
```

Expected: `test result: ok. 251 passed; 0 failed` (count stays same — these are replacements, not additions).

**Note if T25 or pg_b5 still fails:** The resolver `process_ctes` may only register the self-reference source (for recursion avoidance) when `cte.recursive == true`. Since `CommonTableExpr.cterecursive` is always `false` from the raw parser (it's set by pg_query's analyzer pass, not the parser), and `WithIr.recursive` is `true` from `WithClause.recursive`, the resolver checks `with_ir.recursive` to decide. Verify that `src-tauri/crates/sql-scope/src/scope/resolver.rs` checks `with_ir.recursive` (not `cte_ir.recursive`) when determining whether to register the self-reference. If it checks `cte_ir.recursive`, change it to also check `with_ir.recursive`.

- [ ] **Step 9: Commit**

```bash
git add src-tauri/src/completion/tests.rs src-tauri/src/completion/pg_integration_tests.rs
git commit -m "test: strengthen T25, pg_b5, pg_d12 assertions — recursive CTE parser fix makes these real"
```

---

## Task 4: EXISTS subquery outer alias propagation

**The problem:** When the cursor is inside `EXISTS (SELECT 1 FROM orders o WHERE o.user_id = u.|)`, the `u` alias (from the outer query) is invisible because EXISTS subqueries are WHERE-clause expressions, not FROM items. The scope resolver only builds child scopes for `TableRefIr::Subquery` (subqueries in the FROM clause). EXISTS/correlated subqueries are in the expression tree which the IR discards.

**The fix:** In `resolver.rs`, teach `process_select_body` to walk the `select_list` of each `SelectBodyIr` looking for `TableRefIr::Subquery` items that appear as expression arguments — but since those aren't in the IR, the real fix is in the **parser**: emit subqueries appearing in WHERE-clause scalar positions as special `TableRefIr` entries so the resolver can build their child scopes.

**Pragmatic approach:** Rather than a full IR overhaul, we add a `WhereSubquery` variant to `TableRefIr` that the resolver processes into a child scope with the outer scope's aliases inherited. The parser detects scalar subqueries in WHERE-clause positions for both pg_query (Postgres) and sqlparser (SQLite).

**Files:**
- Modify: `src-tauri/crates/sql-scope/src/ir.rs` (add `WhereSubquery` variant to `TableRefIr`)
- Modify: `src-tauri/crates/sql-scope/src/parser/postgres.rs` (emit `WhereSubquery` from WHERE-clause `SubLink` nodes)
- Modify: `src-tauri/crates/sql-scope/src/parser/sqlite.rs` (emit `WhereSubquery` from WHERE-clause `Subquery` expr nodes)
- Modify: `src-tauri/crates/sql-scope/src/scope/resolver.rs` (`build_select_scope` handles `WhereSubquery`)
- Modify: `src-tauri/crates/sql-scope/src/scope/cte.rs` (`expand_table_ref_columns` handles `WhereSubquery`)
- Test: `src-tauri/src/completion/tests.rs` (strengthen T7)
- Test: `src-tauri/src/completion/pg_integration_tests.rs` (add `pg_c2_strong` or upgrade `pg_c2`)

- [ ] **Step 1: Write a failing unit test for EXISTS propagation in `tests.rs`**

Find T7 in `src-tauri/src/completion/tests.rs`. It currently has a weak assertion. Read the test to understand the SQL. Then add a NEW test immediately after T7:

```rust
/// T7b. EXISTS subquery sees outer alias
#[test]
fn t7b_exists_subquery_outer_alias() {
    // cursor at u. inside EXISTS — should see users columns from outer query
    let query = "SELECT u.id FROM users u WHERE EXISTS (SELECT 1 FROM orders o WHERE o.user_id = u.|)";
    let items = complete(query);
    assert!(items.iter().any(|s| s.to_lowercase() == "id"),
        "EXISTS subquery should see outer alias u (users.id), got: {:?}", items);
    assert!(items.iter().any(|s| s.to_lowercase() == "email"),
        "EXISTS subquery should see outer alias u (users.email), got: {:?}", items);
}
```

- [ ] **Step 2: Run to verify it fails**

```bash
cd src-tauri && cargo test --lib "t7b_exists_subquery" -- --nocapture 2>&1 | tail -10
```

Expected: FAIL — `items` is empty because the outer alias `u` isn't propagated into the EXISTS subquery scope.

- [ ] **Step 3: Add `WhereSubquery` variant to `TableRefIr` in `ir.rs`**

In `src-tauri/crates/sql-scope/src/ir.rs`, find the `TableRefIr` enum and add a new variant after `Join`:

```rust
pub enum TableRefIr {
    Table {
        schema: Option<String>,
        name: String,
        alias: Option<String>,
        byte_range: Range<usize>,
    },
    Subquery {
        alias: String,
        body: Box<SelectBodyIr>,
        byte_range: Range<usize>,
    },
    Join {
        left: Box<TableRefIr>,
        right: Box<TableRefIr>,
    },
    /// A scalar or EXISTS subquery appearing in a WHERE/HAVING clause.
    /// Not in the FROM list, so has no alias. Registered to build a child scope
    /// that inherits the enclosing query's sources (enabling outer alias propagation).
    WhereSubquery {
        body: Box<SelectBodyIr>,
    },
}
```

- [ ] **Step 4: Run `cargo build` to find all match arms that need updating**

```bash
cd src-tauri/crates/sql-scope && cargo build 2>&1 | grep "non-exhaustive patterns\|WhereSubquery" | head -20
```

For each match on `TableRefIr` that becomes non-exhaustive, add a `TableRefIr::WhereSubquery { .. } => { /* no-op or recurse */ }` arm. Key locations: `cte.rs` (`expand_table_ref_columns`), `resolver.rs` (`register_table_ref` and `build_select_scope`).

In `cte.rs`, `expand_table_ref_columns`, add:

```rust
TableRefIr::WhereSubquery { .. } => vec![],  // WHERE subqueries don't contribute to SELECT *
```

- [ ] **Step 5: Parse WHERE-clause subqueries in the Postgres parser**

In `src-tauri/crates/sql-scope/src/parser/postgres.rs`, find `convert_select_body`. It currently only reads `sel.from_clause`. Add WHERE-clause subquery extraction after the FROM:

```rust
fn convert_select_body(sel: &pg_query::protobuf::SelectStmt, sql: &str) -> SelectBodyIr {
    let mut from: Vec<TableRefIr> = sel
        .from_clause
        .iter()
        .filter_map(|node| node.node.as_ref().and_then(|n| parse_table_ref(n, sql)))
        .collect();

    // Extract scalar/EXISTS subqueries from the WHERE clause so the resolver
    // can build child scopes that inherit the outer query's aliases.
    if let Some(where_node) = &sel.where_clause {
        collect_where_subqueries(where_node, sql, &mut from);
    }

    let select_list = sel
        .target_list
        .iter()
        .filter_map(|node| {
            if let Some(NodeEnum::ResTarget(rt)) = node.node.as_ref() {
                Some(parse_select_item(rt))
            } else {
                None
            }
        })
        .collect();

    SelectBodyIr { from, select_list, byte_range: 0..sql.len() }
}

/// Walk a WHERE-clause expression tree and emit WhereSubquery entries for
/// any scalar subquery or EXISTS(subquery) nodes found.
fn collect_where_subqueries(
    node: &pg_query::protobuf::Node,
    sql: &str,
    out: &mut Vec<TableRefIr>,
) {
    use pg_query::protobuf::SubLinkType;
    let Some(n) = node.node.as_ref() else { return };
    match n {
        NodeEnum::SubLink(sub) => {
            // EXISTS, ANY, ALL, EXPR sublinks — all have a subquery body
            if let Some(subsel_node) = &sub.subselect {
                if let Some(NodeEnum::SelectStmt(subsel)) = subsel_node.node.as_ref() {
                    let body = convert_select_body(subsel, sql);
                    out.push(TableRefIr::WhereSubquery { body: Box::new(body) });
                }
            }
        }
        NodeEnum::BoolExpr(b) => {
            for arg in &b.args {
                collect_where_subqueries(arg, sql, out);
            }
        }
        NodeEnum::AExpr(a) => {
            if let Some(l) = &a.lexpr { collect_where_subqueries(l, sql, out); }
            if let Some(r) = &a.rexpr { collect_where_subqueries(r, sql, out); }
        }
        _ => {}
    }
}
```

- [ ] **Step 6: Parse WHERE-clause subqueries in the SQLite parser**

In `src-tauri/crates/sql-scope/src/parser/sqlite.rs`, find `convert_select`. After the FROM extraction, add WHERE-clause subquery extraction:

```rust
fn convert_select(sel: Select, sql: &str) -> SelectBodyIr {
    let mut from: Vec<TableRefIr> = sel
        .from
        .into_iter()
        .flat_map(|twj| convert_table_with_joins(twj, sql))
        .collect();

    // Collect scalar/EXISTS subqueries from the WHERE clause
    if let Some(where_expr) = sel.selection {
        collect_where_subqueries_expr(&where_expr, sql, &mut from);
    }

    let select_list: Vec<SelectItemIr> = sel
        .projection
        .into_iter()
        .map(convert_select_item)
        .collect();

    SelectBodyIr { from, select_list, byte_range: 0..sql.len() }
}

fn collect_where_subqueries_expr(expr: &Expr, sql: &str, out: &mut Vec<TableRefIr>) {
    match expr {
        Expr::Exists { subquery, .. } | Expr::Subquery(subquery) => {
            let body = convert_query(*subquery.clone(), sql).body;
            out.push(TableRefIr::WhereSubquery { body: Box::new(body) });
        }
        Expr::BinaryOp { left, right, .. } => {
            collect_where_subqueries_expr(left, sql, out);
            collect_where_subqueries_expr(right, sql, out);
        }
        Expr::UnaryOp { expr, .. } => collect_where_subqueries_expr(expr, sql, out),
        Expr::IsNull(e) | Expr::IsNotNull(e) | Expr::IsTrue(e)
        | Expr::IsFalse(e) | Expr::IsNotTrue(e) | Expr::IsNotFalse(e) => {
            collect_where_subqueries_expr(e, sql, out);
        }
        _ => {}
    }
}
```

**Note:** The `Expr` variants available depend on `sqlparser` version. Read `src-tauri/crates/sql-scope/src/parser/sqlite.rs` imports at the top to see which `Expr` variants are already imported, and add any new ones needed.

- [ ] **Step 7: Teach the resolver to build a child scope for `WhereSubquery`**

In `src-tauri/crates/sql-scope/src/scope/resolver.rs`, find `register_table_ref` (the function that handles each `TableRefIr` variant). Add a match arm for `WhereSubquery`:

```rust
TableRefIr::WhereSubquery { body } => {
    // Build a child scope for the WHERE subquery with the current scope as parent.
    // This makes outer aliases (e.g. `u` for users) visible inside the subquery.
    let child_id = build_select_scope(body, tree, schema, Some(scope_id));
    // No alias to register — WHERE subqueries are not accessible by name from outside.
    let _ = child_id;
}
```

`build_select_scope` must accept `parent_id: Option<ScopeId>`. Verify it already does — if the signature is `build_select_scope(body, tree, schema)` (no parent), add the parameter and thread it through. Check the existing `Subquery` arm to see how it's called.

- [ ] **Step 8: Run T7b to verify it passes**

```bash
cd src-tauri && cargo test --lib "t7b_exists_subquery" -- --nocapture 2>&1 | tail -10
```

Expected: PASS — `items` now contains `id`, `email`.

- [ ] **Step 9: Strengthen T7 in `tests.rs`**

Find the original T7 test. It has a `println!` and a weak assertion. Replace its assertion with:

```rust
assert!(items.iter().any(|s| s.to_lowercase() == "id"),
    "T7: EXISTS subquery should see outer users alias, got: {:?}", items);
```

- [ ] **Step 10: Run the full test suite**

```bash
cd src-tauri && cargo test --lib 2>&1 | grep "test result"
```

Expected: `test result: ok. 252 passed; 0 failed` (251 + 1 new T7b).

Also run:
```bash
cd src-tauri/crates/sql-scope && cargo test 2>&1 | grep "test result"
```

Expected: all sql-scope tests pass.

- [ ] **Step 11: Commit**

```bash
git add src-tauri/crates/sql-scope/src/ir.rs \
        src-tauri/crates/sql-scope/src/parser/postgres.rs \
        src-tauri/crates/sql-scope/src/parser/sqlite.rs \
        src-tauri/crates/sql-scope/src/scope/resolver.rs \
        src-tauri/crates/sql-scope/src/scope/cte.rs \
        src-tauri/src/completion/tests.rs
git commit -m "feat(scope): propagate outer aliases into EXISTS/correlated subquery scopes via WhereSubquery IR node"
```

---

## Self-Review

**1. Spec coverage:**
- ✅ Recursive CTE UNION ALL anchor extraction (Task 1)
- ✅ `list_foreign_keys_schema` typo fix (Task 2)
- ✅ Test assertions strengthened (Task 3 — depends on Task 1)
- ✅ EXISTS subquery outer alias propagation (Task 4)
- ℹ️ Column type propagation through CTEs — intentionally excluded (requires full IR type system, YAGNI for now)

**2. Placeholder scan:** All steps contain exact code. No "TBD" or "handle appropriately" phrases.

**3. Type consistency:**
- `TableRefIr::WhereSubquery { body: Box<SelectBodyIr> }` — used consistently in ir.rs, postgres.rs, sqlite.rs, resolver.rs, cte.rs
- `build_select_scope` — Task 4 Step 7 notes to verify the existing signature accepts `parent_id`; if not, thread it through. The existing `Subquery` arm already uses a parent — follow that exact pattern.
- `convert_select_body` — used in both postgres.rs Task 1 and Task 4; signature unchanged

**Task ordering note:** Task 3 depends on Task 1 (recursive CTE parser fix). Tasks 2 and 4 are independent of each other and of Tasks 1/3.
