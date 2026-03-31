# pg_query-Based Diagnostic Pipeline Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the tree-sitter-first diagnostic pipeline with pg_query as the primary validator, giving accurate error positions and true semantic checks for all PostgreSQL statement types including ANALYZE, VACUUM, INSERT, etc.

**Architecture:** Split SQL input into statements using `split_statements()`, parse each with `pg_query` via `sql_scope`, emit pg_query's precise error message (with byte position) on parse failure, run semantic checks on success. Tree-sitter is removed from the diagnostic path entirely for complete statements.

**Tech Stack:** `sql_scope` crate (already exists at `src-tauri/crates/sql-scope/`), `pg_query = "6.1"` (already a dependency), Rust, tree-sitter (kept only for completions, not diagnostics)

---

## File Map

| File | Change |
|---|---|
| `src-tauri/crates/sql-scope/src/ir.rs` | `ParsedStatement::Other` unit → struct with `table_refs` |
| `src-tauri/crates/sql-scope/src/parser/postgres.rs` | Extract `TableRefIr` from VacuumStmt, InsertStmt, UpdateStmt relation, TruncateStmt, LockStmt, CreateStmt, AlterTableStmt |
| `src-tauri/crates/sql-scope/src/parser/sqlite.rs` | Minimal: `Other` unit → `Other { table_refs: vec![] }` |
| `src-tauri/crates/sql-scope/src/parser/mysql.rs` | Same as sqlite |
| `src-tauri/crates/sql-scope/src/scope/resolver.rs` | Handle new `Other { table_refs }` variant (currently only handles `Select`) |
| `src-tauri/crates/sql-scope/src/diagnostics.rs` | Add validation loop for `Other { table_refs }` |
| `src-tauri/src/completion/diagnostics.rs` | Flip pipeline: pg_query primary, tree-sitter removed from diagnostic path |

---

### Task 1: Change `ParsedStatement::Other` to carry table refs

**Files:**
- Modify: `src-tauri/crates/sql-scope/src/ir.rs`

- [ ] **Step 1: Write the failing test**

Add to the bottom of the existing `#[cfg(test)]` block in `ir.rs`:

```rust
#[test]
fn test_parsed_statement_other_with_table_refs() {
    let tref = TableRefIr::Table {
        schema: Some("production".to_string()),
        name: "tasks".to_string(),
        alias: None,
        byte_range: 0..20,
    };
    let stmt = ParsedStatement::Other { table_refs: vec![tref] };
    if let ParsedStatement::Other { table_refs } = stmt {
        assert_eq!(table_refs.len(), 1);
        assert!(matches!(&table_refs[0], TableRefIr::Table { name, .. } if name == "tasks"));
    } else {
        panic!("Expected Other");
    }
}

#[test]
fn test_parsed_statement_other_empty_refs() {
    let stmt = ParsedStatement::Other { table_refs: vec![] };
    if let ParsedStatement::Other { table_refs } = stmt {
        assert!(table_refs.is_empty());
    } else {
        panic!("Expected Other");
    }
}

#[test]
fn test_clone_parsed_statement_other_with_refs() {
    let stmt = ParsedStatement::Other {
        table_refs: vec![TableRefIr::Table {
            schema: None,
            name: "logs".to_string(),
            alias: None,
            byte_range: 0..4,
        }],
    };
    let cloned = stmt.clone();
    if let ParsedStatement::Other { table_refs } = cloned {
        assert_eq!(table_refs.len(), 1);
    } else {
        panic!("Expected Other");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd src-tauri && cargo test -p sql-scope ir::tests::test_parsed_statement_other_with_table_refs 2>&1 | tail -20
```
Expected: FAIL — `Other` doesn't accept struct syntax yet.

- [ ] **Step 3: Update the `ParsedStatement` enum**

In `src-tauri/crates/sql-scope/src/ir.rs`, change line 10:

```rust
// BEFORE
/// INSERT, CREATE, ALTER, etc. — kept for future expansion
Other,

// AFTER
/// INSERT, CREATE, ALTER, ANALYZE, etc. — carries table references for semantic checks
Other { table_refs: Vec<TableRefIr> },
```

- [ ] **Step 4: Fix the existing `test_parsed_statement_other` test**

In the same file, find and update the existing unit test (around line 241):

```rust
#[test]
fn test_parsed_statement_other() {
    let stmt = ParsedStatement::Other { table_refs: vec![] };
    assert!(matches!(stmt, ParsedStatement::Other { .. }));
}
```

Also update the pattern matching test around line 499:
```rust
#[test]
fn test_pattern_match_parsed_statement_select() {
    let body = make_select_body(vec![SelectItemIr::Wildcard], vec![]);
    let stmt = ParsedStatement::Select(SelectIr {
        with: None,
        body,
        byte_range: 0..10,
    });
    assert!(matches!(stmt, ParsedStatement::Select(_)));
    assert!(!matches!(stmt, ParsedStatement::Other { .. }));
}
```

- [ ] **Step 5: Run tests to verify they pass**

```bash
cd src-tauri && cargo test -p sql-scope ir:: 2>&1 | tail -30
```
Expected: All `ir::tests::*` pass. Compiler will now emit errors in `parser/postgres.rs`, `parser/sqlite.rs`, `parser/mysql.rs`, and `scope/resolver.rs` because of exhaustive match. That's expected — fix those in the next tasks.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/crates/sql-scope/src/ir.rs
git commit -m "feat(sql-scope): ParsedStatement::Other now carries Vec<TableRefIr>"
```

---

### Task 2: Extract table refs from utility statements in postgres parser

**Files:**
- Modify: `src-tauri/crates/sql-scope/src/parser/postgres.rs`

- [ ] **Step 1: Write the failing tests**

Add to the existing `#[cfg(test)]` block in `postgres.rs`:

```rust
#[test]
fn test_insert_carries_target_table() {
    let sql = "INSERT INTO production.users (id, name) VALUES (1, 'Alice')";
    let result = parse_postgres(sql).expect("should parse");
    if let ParsedStatement::Other { table_refs } = result {
        assert_eq!(table_refs.len(), 1, "INSERT should carry the target table");
        if let TableRefIr::Table { schema, name, .. } = &table_refs[0] {
            assert_eq!(schema.as_deref(), Some("production"));
            assert_eq!(name, "users");
        } else {
            panic!("Expected Table ref");
        }
    } else {
        panic!("Expected Other");
    }
}

#[test]
fn test_analyze_carries_relation() {
    let sql = "ANALYZE production.tasks";
    let result = parse_postgres(sql).expect("should parse");
    if let ParsedStatement::Other { table_refs } = result {
        assert_eq!(table_refs.len(), 1, "ANALYZE should carry the target relation");
        if let TableRefIr::Table { schema, name, .. } = &table_refs[0] {
            assert_eq!(schema.as_deref(), Some("production"));
            assert_eq!(name, "tasks");
        } else {
            panic!("Expected Table ref");
        }
    } else {
        panic!("Expected Other");
    }
}

#[test]
fn test_analyze_multiple_tables() {
    let sql = "ANALYZE production.tasks, production.users";
    let result = parse_postgres(sql).expect("should parse");
    if let ParsedStatement::Other { table_refs } = result {
        assert_eq!(table_refs.len(), 2);
    } else {
        panic!("Expected Other");
    }
}

#[test]
fn test_vacuum_carries_relation() {
    let sql = "VACUUM production.logs";
    let result = parse_postgres(sql).expect("should parse");
    if let ParsedStatement::Other { table_refs } = result {
        assert_eq!(table_refs.len(), 1);
        if let TableRefIr::Table { name, .. } = &table_refs[0] {
            assert_eq!(name, "logs");
        } else {
            panic!("Expected Table");
        }
    } else {
        panic!("Expected Other");
    }
}

#[test]
fn test_lock_table_carries_relation() {
    let sql = "LOCK TABLE production.orders IN EXCLUSIVE MODE";
    let result = parse_postgres(sql).expect("should parse");
    if let ParsedStatement::Other { table_refs } = result {
        assert_eq!(table_refs.len(), 1);
        if let TableRefIr::Table { name, .. } = &table_refs[0] {
            assert_eq!(name, "orders");
        } else {
            panic!("Expected Table");
        }
    } else {
        panic!("Expected Other");
    }
}

#[test]
fn test_create_table_empty_refs() {
    // CREATE TABLE itself is a table definition, not a reference
    let sql = "CREATE TABLE users (id INT, name TEXT)";
    let result = parse_postgres(sql).expect("should parse");
    if let ParsedStatement::Other { table_refs } = result {
        // No table refs for CREATE TABLE (the table doesn't exist yet)
        assert!(table_refs.is_empty(), "CREATE TABLE should have no table refs");
    } else {
        panic!("Expected Other");
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cd src-tauri && cargo test -p sql-scope parser::postgres::tests::test_insert_carries 2>&1 | tail -20
```
Expected: Compile error because `Other` variants in the `match` arm still use unit syntax, or test fails because `table_refs` is empty.

- [ ] **Step 3: Update the match in `parse_postgres`**

In `src-tauri/crates/sql-scope/src/parser/postgres.rs`, replace the `_ => Some(ParsedStatement::Other)` catch-all and add specific arms:

```rust
pub fn parse_postgres(sql: &str) -> Option<ParsedStatement> {
    if sql.trim().is_empty() {
        return None;
    }

    let result = pg_query::parse(sql).ok()?;
    let raw_stmt = result.protobuf.stmts.into_iter().next()?;
    let node_enum = raw_stmt.stmt?.node?;

    match node_enum {
        NodeEnum::SelectStmt(sel) => {
            let select_ir = convert_select_stmt(&sel, sql);
            Some(ParsedStatement::Select(select_ir))
        }
        NodeEnum::DeleteStmt(del) => Some(ParsedStatement::Dangerous {
            kind: DangerousKind::DeleteWithoutWhere,
            has_where: del.where_clause.is_some(),
        }),
        NodeEnum::UpdateStmt(upd) => Some(ParsedStatement::Dangerous {
            kind: DangerousKind::UpdateWithoutWhere,
            has_where: upd.where_clause.is_some(),
        }),
        NodeEnum::TruncateStmt(_) => Some(ParsedStatement::Dangerous {
            kind: DangerousKind::Truncate,
            has_where: false,
        }),
        NodeEnum::DropStmt(_) => Some(ParsedStatement::Dangerous {
            kind: DangerousKind::Drop,
            has_where: false,
        }),
        // ANALYZE / VACUUM — VacuumStmt covers both
        NodeEnum::VacuumStmt(vac) => {
            let table_refs = vac.rels.iter()
                .filter_map(|node| node.node.as_ref())
                .filter_map(|n| {
                    if let NodeEnum::VacuumRelation(vr) = n {
                        let name = vr.relation.as_ref()?.relname.to_lowercase();
                        let schema = vr.relation.as_ref()
                            .filter(|rv| !rv.schemaname.is_empty())
                            .map(|rv| rv.schemaname.to_lowercase());
                        Some(TableRefIr::Table { schema, name, alias: None, byte_range: 0..sql.len() })
                    } else {
                        None
                    }
                })
                .collect();
            Some(ParsedStatement::Other { table_refs })
        }
        // INSERT — carries the target relation
        NodeEnum::InsertStmt(ins) => {
            let table_refs = extract_range_var_ref(ins.relation.as_ref(), sql);
            Some(ParsedStatement::Other { table_refs })
        }
        // LOCK TABLE
        NodeEnum::LockStmt(lock) => {
            let table_refs = lock.relations.iter()
                .filter_map(|node| node.node.as_ref())
                .filter_map(|n| {
                    if let NodeEnum::RangeVar(rv) = n {
                        let name = rv.relname.to_lowercase();
                        let schema = if rv.schemaname.is_empty() { None } else { Some(rv.schemaname.to_lowercase()) };
                        Some(TableRefIr::Table { schema, name, alias: None, byte_range: 0..sql.len() })
                    } else {
                        None
                    }
                })
                .collect();
            Some(ParsedStatement::Other { table_refs })
        }
        // CREATE TABLE, ALTER TABLE, etc. — no pre-existing table refs to validate
        _ => Some(ParsedStatement::Other { table_refs: vec![] }),
    }
}

/// Extract a single TableRefIr from an optional RangeVar (used by INSERT)
fn extract_range_var_ref(rv: Option<&pg_query::protobuf::RangeVar>, sql: &str) -> Vec<TableRefIr> {
    let Some(rv) = rv else { return vec![] };
    let name = rv.relname.to_lowercase();
    if name.is_empty() {
        return vec![];
    }
    let schema = if rv.schemaname.is_empty() { None } else { Some(rv.schemaname.to_lowercase()) };
    vec![TableRefIr::Table { schema, name, alias: None, byte_range: 0..sql.len() }]
}
```

- [ ] **Step 4: Fix the `test_insert_returns_other` and `test_create_table_returns_other` tests**

These existing tests used `matches!(result, ParsedStatement::Other)` — update to `ParsedStatement::Other { .. }`:

```rust
#[test]
fn test_insert_returns_other() {
    let sql = "INSERT INTO users (id, name) VALUES (1, 'Alice')";
    let result = parse_postgres(sql).expect("should parse");
    assert!(matches!(result, ParsedStatement::Other { .. }));
}

#[test]
fn test_create_table_returns_other() {
    let sql = "CREATE TABLE users (id INT, name TEXT)";
    let result = parse_postgres(sql).expect("should parse");
    assert!(matches!(result, ParsedStatement::Other { .. }));
}

#[test]
fn test_alter_table_returns_other() {
    let sql = "ALTER TABLE users ADD COLUMN email TEXT";
    let result = parse_postgres(sql).expect("should parse");
    assert!(matches!(result, ParsedStatement::Other { .. }));
}
```

- [ ] **Step 5: Run tests to verify they pass**

```bash
cd src-tauri && cargo test -p sql-scope parser::postgres:: 2>&1 | tail -40
```
Expected: All `parser::postgres::tests::*` pass.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/crates/sql-scope/src/parser/postgres.rs
git commit -m "feat(sql-scope): extract table refs from ANALYZE/VACUUM/INSERT/LOCK in postgres parser"
```

---

### Task 3: Update SQLite and MySQL parsers for the new Other variant

**Files:**
- Modify: `src-tauri/crates/sql-scope/src/parser/sqlite.rs`
- Modify: `src-tauri/crates/sql-scope/src/parser/mysql.rs`

Note: SQLite and MySQL don't have ANALYZE/VACUUM. The only change is updating `_ => Some(ParsedStatement::Other)` to `_ => Some(ParsedStatement::Other { table_refs: vec![] })` and fixing all tests that pattern-match on `ParsedStatement::Other`.

- [ ] **Step 1: Update `sqlite.rs`**

In `src-tauri/crates/sql-scope/src/parser/sqlite.rs`, in `convert_statement`:

```rust
fn convert_statement(stmt: Statement, sql: &str) -> Option<ParsedStatement> {
    match stmt {
        Statement::Query(q) => Some(ParsedStatement::Select(convert_query(*q, sql))),
        Statement::Delete(del) => Some(ParsedStatement::Dangerous {
            kind: DangerousKind::DeleteWithoutWhere,
            has_where: del.selection.is_some(),
        }),
        Statement::Update(update) => Some(ParsedStatement::Dangerous {
            kind: DangerousKind::UpdateWithoutWhere,
            has_where: update.selection.is_some(),
        }),
        Statement::Truncate { .. } => Some(ParsedStatement::Dangerous {
            kind: DangerousKind::Truncate,
            has_where: false,
        }),
        Statement::Drop { .. } => Some(ParsedStatement::Dangerous {
            kind: DangerousKind::Drop,
            has_where: false,
        }),
        _ => Some(ParsedStatement::Other { table_refs: vec![] }),
    }
}
```

- [ ] **Step 2: Fix any SQLite tests that match `ParsedStatement::Other`**

Search the sqlite tests for `ParsedStatement::Other` and update `matches!(result, ParsedStatement::Other)` to `matches!(result, ParsedStatement::Other { .. })`.

- [ ] **Step 3: Update `mysql.rs`**

Do the same change in `src-tauri/crates/sql-scope/src/parser/mysql.rs` — the `convert_statement` function has the same pattern. Change the catch-all arm to `_ => Some(ParsedStatement::Other { table_refs: vec![] })`.

Fix any MySQL tests the same way.

- [ ] **Step 4: Run all sql-scope parser tests**

```bash
cd src-tauri && cargo test -p sql-scope parser:: 2>&1 | tail -40
```
Expected: All `parser::*` tests pass.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/crates/sql-scope/src/parser/sqlite.rs src-tauri/crates/sql-scope/src/parser/mysql.rs
git commit -m "fix(sql-scope): update SQLite/MySQL parsers for Other { table_refs } variant"
```

---

### Task 4: Update scope resolver to handle `Other { table_refs }`

**Files:**
- Modify: `src-tauri/crates/sql-scope/src/scope/resolver.rs`

Currently `traverse_scope` only processes `ParsedStatement::Select`. The new `Other { table_refs }` variant can now be used to build a minimal scope so diagnostics can validate its table refs.

- [ ] **Step 1: Write the failing test**

Add to the `#[cfg(test)]` block in `resolver.rs`:

```rust
#[test]
fn analyze_stmt_registers_table_in_scope() {
    use crate::parser::postgres::parse_postgres;
    let sql = "ANALYZE production.tasks";
    let stmt = parse_postgres(sql).unwrap();
    let schema = mock(&[("tasks", &[])]);
    let tree = traverse_scope(&stmt, &schema);
    // The scope tree should have at least one scope with "tasks" registered
    let all = tree.all_scopes();
    assert!(!all.is_empty(), "ANALYZE should produce a scope");
    let has_tasks = all.iter().any(|s| s.sources.contains_key("tasks"));
    assert!(has_tasks, "tasks should be registered as a source");
}

#[test]
fn other_with_empty_refs_produces_empty_tree() {
    use crate::parser::postgres::parse_postgres;
    let sql = "CREATE TABLE newt (id INT)";
    let stmt = parse_postgres(sql).unwrap();
    let schema = mock(&[]);
    let tree = traverse_scope(&stmt, &schema);
    // CREATE TABLE produces Other { table_refs: [] } — no scopes needed
    assert!(tree.scope_at(5).is_none(), "CREATE TABLE should produce no scopes");
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cd src-tauri && cargo test -p sql-scope scope::resolver::tests::analyze_stmt_registers 2>&1 | tail -20
```
Expected: FAIL — `traverse_scope` returns empty tree for `Other { .. }`.

- [ ] **Step 3: Update `traverse_scope` to handle `Other { table_refs }`**

In `src-tauri/crates/sql-scope/src/scope/resolver.rs`:

```rust
/// Build a `ScopeTree` for the given parsed statement.
///
/// - `Select`: full scope tree with CTEs and derived tables.
/// - `Other { table_refs }`: a single flat scope with the table refs registered.
///   This enables `run_diagnostics` to validate table names in INSERT/ANALYZE/LOCK/etc.
/// - `Dangerous` / `Other { table_refs: [] }`: empty tree (no validation needed).
pub fn traverse_scope(stmt: &ParsedStatement, schema: &dyn SchemaSnapshot) -> ScopeTree {
    let mut tree = ScopeTree::new();
    match stmt {
        ParsedStatement::Select(sel) => {
            build_select_scope(sel, None, &mut tree, schema);
        }
        ParsedStatement::Other { table_refs } if !table_refs.is_empty() => {
            // Build a minimal flat scope so diagnostics can validate table refs
            let scope_id = tree.add_scope(Scope {
                id: 0,
                parent: None,
                scope_type: ScopeType::Root,
                byte_range: 0..1,
                sources: std::collections::HashMap::new(),
                cte_sources: indexmap::IndexMap::new(),
                columns: Vec::new(),
            });
            for tref in table_refs {
                register_table_ref(tref, scope_id, &mut tree, schema);
            }
        }
        _ => {} // Dangerous, Other { table_refs: [] } — nothing to scope
    }
    tree
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cd src-tauri && cargo test -p sql-scope scope::resolver:: 2>&1 | tail -40
```
Expected: All `scope::resolver::tests::*` pass including the two new ones.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/crates/sql-scope/src/scope/resolver.rs
git commit -m "feat(sql-scope): traverse_scope handles Other { table_refs } with a flat scope"
```

---

### Task 5: Add diagnostic validation for `Other { table_refs }` in sql-scope

**Files:**
- Modify: `src-tauri/crates/sql-scope/src/diagnostics.rs`

Currently `run_diagnostics` only sees sources registered in the scope tree. Because Task 4 now registers `Other { table_refs }` sources into the tree, `run_diagnostics` will automatically pick them up — but we need a test to verify this end-to-end.

- [ ] **Step 1: Write the failing tests**

Add to the `#[cfg(test)]` block in `diagnostics.rs`:

```rust
#[test]
fn analyze_unknown_table_warns() {
    let sql = "ANALYZE production.tasksj";
    let stmt = parse_postgres(sql).unwrap();
    let schema = schema_with(&["tasks"]); // "tasksj" not in schema
    let tree = traverse_scope(&stmt, &schema);
    let diags = run_diagnostics(&tree, &schema, sql);
    assert!(
        diags.iter().any(|d| d.message.contains("tasksj") && d.severity == DiagSeverity::Warning),
        "expected warning for unknown table 'tasksj', got {:?}", diags
    );
}

#[test]
fn analyze_known_table_no_warning() {
    let sql = "ANALYZE production.tasks";
    let stmt = parse_postgres(sql).unwrap();
    let schema = schema_with(&["tasks"]);
    let tree = traverse_scope(&stmt, &schema);
    let diags = run_diagnostics(&tree, &schema, sql);
    assert!(
        !diags.iter().any(|d| d.message.contains("tasks")),
        "known table should not warn, got {:?}", diags
    );
}

#[test]
fn insert_unknown_table_warns() {
    let sql = "INSERT INTO ghost_table (id) VALUES (1)";
    let stmt = parse_postgres(sql).unwrap();
    let schema = schema_with(&["users"]);
    let tree = traverse_scope(&stmt, &schema);
    let diags = run_diagnostics(&tree, &schema, sql);
    assert!(
        diags.iter().any(|d| d.message.contains("ghost_table")),
        "expected warning for unknown insert target, got {:?}", diags
    );
}

#[test]
fn create_table_no_warning() {
    // CREATE TABLE produces Other { table_refs: [] }, no scope, no diagnostics
    let sql = "CREATE TABLE new_table (id INT)";
    let stmt = parse_postgres(sql).unwrap();
    let schema = schema_with(&[]); // empty schema
    let tree = traverse_scope(&stmt, &schema);
    let diags = run_diagnostics(&tree, &schema, sql);
    assert!(diags.is_empty(), "CREATE TABLE should produce no diagnostics, got {:?}", diags);
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cd src-tauri && cargo test -p sql-scope diagnostics::tests::analyze_unknown 2>&1 | tail -20
```
Expected: FAIL — `tasksj` not reported because scope tree was empty before Task 4. After Task 4 is done, this should now pass automatically.

- [ ] **Step 3: Run all diagnostic tests**

```bash
cd src-tauri && cargo test -p sql-scope diagnostics:: 2>&1 | tail -40
```
Expected: All `diagnostics::tests::*` pass — the resolver changes from Task 4 make this work without any code changes to `diagnostics.rs` itself.

- [ ] **Step 4: Run the full sql-scope test suite**

```bash
cd src-tauri && cargo test -p sql-scope 2>&1 | tail -50
```
Expected: All tests pass.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/crates/sql-scope/src/diagnostics.rs
git commit -m "test(sql-scope): add diagnostic tests for ANALYZE/INSERT/CREATE TABLE via Other { table_refs }"
```

---

### Task 6: Flip the diagnostic pipeline in `completion/diagnostics.rs`

**Files:**
- Modify: `src-tauri/src/completion/diagnostics.rs`

This is the main payoff: replace the tree-sitter-first 5-pass pipeline with pg_query as the primary validator.

**New flow:**
1. Split input into statements using `sql_scope::split_statements()`
2. For each statement, attempt `sql_scope::resolve()` (which calls `pg_query::parse` internally)
3. **If parse succeeds**: run `sql_scope::run_diagnostics()` for semantic checks (unknown tables, dangerous warnings)
4. **If parse fails**: `pg_query` gives a precise error with byte position — emit that directly
5. Tree-sitter passes are removed entirely from the diagnostic path

- [ ] **Step 1: Write failing tests**

Add to the `#[cfg(test)]` block in `src-tauri/src/completion/diagnostics.rs`:

```rust
#[test]
fn test_analyze_known_table_no_diagnostic() {
    let sql = "ANALYZE production.tasks;";
    let tree = parse_sql(sql, None).unwrap();
    let mut schema = SchemaGraph::new();
    schema.add_table(TableInfo::new("tasks", "production", vec![]));
    let diagnostics = DiagnosticEngine::check(&tree, sql, &schema);
    let errors_or_warnings: Vec<_> = diagnostics.iter()
        .filter(|d| d.severity <= 2)
        .collect();
    assert!(
        errors_or_warnings.is_empty(),
        "ANALYZE with known table should produce no diagnostics, got {:?}", errors_or_warnings
    );
}

#[test]
fn test_analyze_unknown_table_warns() {
    let sql = "ANALYZE production.tasksj;";
    let tree = parse_sql(sql, None).unwrap();
    let mut schema = SchemaGraph::new();
    schema.add_table(TableInfo::new("tasks", "production", vec![]));
    let diagnostics = DiagnosticEngine::check(&tree, sql, &schema);
    assert!(
        diagnostics.iter().any(|d| d.message.contains("tasksj")),
        "should warn about unknown table 'tasksj', got {:?}", diagnostics
    );
}

#[test]
fn test_pg_query_error_has_position() {
    let sql = "SELECT * FROM ;"; // actual parse error
    let tree = parse_sql(sql, None).unwrap();
    let schema = SchemaGraph::new();
    let diagnostics = DiagnosticEngine::check(&tree, sql, &schema);
    assert!(
        diagnostics.iter().any(|d| d.severity == 1),
        "invalid SQL should produce a severity=1 error, got {:?}", diagnostics
    );
}

#[test]
fn test_multi_statement_mixed() {
    let sql = "ANALYZE production.tasks; SELECT * FROM production.tasks;";
    let tree = parse_sql(sql, None).unwrap();
    let mut schema = SchemaGraph::new();
    schema.add_table(TableInfo::new("tasks", "production", vec![]));
    let diagnostics = DiagnosticEngine::check(&tree, sql, &schema);
    let errors: Vec<_> = diagnostics.iter().filter(|d| d.severity == 1).collect();
    assert!(errors.is_empty(), "valid multi-statement SQL should have no errors, got {:?}", errors);
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cd src-tauri && cargo test -p tables-lib completion::diagnostics::tests::test_analyze_unknown_table_warns 2>&1 | tail -20
```
or
```bash
cd src-tauri && cargo test completion::diagnostics 2>&1 | tail -20
```
Expected: `test_analyze_unknown_table_warns` FAIL (currently returns no diagnostic for unknown table in ANALYZE).

- [ ] **Step 3: Rewrite `DiagnosticEngine::check`**

Replace the entire body of `DiagnosticEngine::check` in `src-tauri/src/completion/diagnostics.rs`:

```rust
pub fn check(tree: &Tree, source: &str, schema: &SchemaGraph) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // Only run semantic diagnostics if schema is populated
    // (avoids squiggling everything during initialization / while disconnected)
    let schema_populated = !schema.tables.is_empty();

    for (offset, stmt_str) in sql_scope::split_statements(source) {
        if stmt_str.trim().is_empty() {
            continue;
        }

        // Try pg_query parse via sql_scope
        match sql_scope::resolve(stmt_str, sql_scope::Dialect::Postgres, schema) {
            Ok(scope_tree) => {
                // Parse succeeded — run semantic checks
                if schema_populated {
                    let scope_diags = sql_scope::run_diagnostics(&scope_tree, schema, stmt_str);
                    for sd in scope_diags {
                        diagnostics.push(Diagnostic {
                            message: sd.message,
                            start: offset + sd.byte_range.start,
                            end: offset + sd.byte_range.end,
                            severity: match sd.severity {
                                sql_scope::DiagSeverity::Error => 1,
                                sql_scope::DiagSeverity::Warning => 2,
                                sql_scope::DiagSeverity::Info => 3,
                            },
                        });
                    }
                }

                // Dangerous statement warnings (DROP/TRUNCATE/DELETE without WHERE)
                // sql_scope already classifies these — emit them here
                Self::check_dangerous_scope(&scope_tree, stmt_str, offset, &mut diagnostics);
            }
            Err(sql_scope::ScopeError::Parse(msg)) => {
                // pg_query parse error — extract position if available
                // pg_query errors look like: "syntax error at or near \"foo\" (pos 42)"
                // or the error message contains "position:" from libpg_query
                let (start, end) = Self::extract_error_position(&msg, stmt_str, offset);
                diagnostics.push(Diagnostic {
                    message: format!("Syntax error: {}", Self::clean_pg_error(&msg)),
                    start,
                    end,
                    severity: 1,
                });
            }
            Err(_) => {
                // Other errors (shouldn't happen) — skip
            }
        }
    }

    diagnostics
}
```

- [ ] **Step 4: Remove the old tree-sitter traverse/check methods and add new helpers**

Remove these methods entirely from `DiagnosticEngine`:
- `fn traverse(...)` (the old Pass 0-3 implementation)
- `fn check_dangerous_stmts(...)` (old text-based Pass 4)

Keep `find_first_child_of_kind` and `is_in_table_context` only if still needed — they are not needed after this change, so remove them too.

Add the new helper methods:

```rust
/// Check if the scope tree contains a dangerous statement and emit warnings.
/// sql_scope classifies DELETE/UPDATE/TRUNCATE/DROP into ParsedStatement::Dangerous.
/// We reconstruct those from the scope tree by checking if resolve() produced
/// a Dangerous variant. Since traverse_scope ignores Dangerous (empty tree),
/// we need to call parse_postgres directly.
fn check_dangerous_scope(
    _scope_tree: &sql_scope::ScopeTree,
    stmt_str: &str,
    offset: usize,
    diagnostics: &mut Vec<Diagnostic>,
) {
    // Re-parse to get the IR variant — parse_postgres is fast (pure C parser)
    use sql_scope::ParsedStatement;
    if let Some(stmt) = sql_scope::parser::postgres::parse_postgres(stmt_str) {
        if let ParsedStatement::Dangerous { kind, has_where } = stmt {
            let message = match kind {
                sql_scope::ir::DangerousKind::Drop =>
                    "Destructive: DROP cannot be undone".to_string(),
                sql_scope::ir::DangerousKind::Truncate =>
                    "Destructive: TRUNCATE will delete all rows".to_string(),
                sql_scope::ir::DangerousKind::DeleteWithoutWhere if !has_where =>
                    "DELETE without WHERE will erase every row".to_string(),
                sql_scope::ir::DangerousKind::UpdateWithoutWhere if !has_where =>
                    "UPDATE without WHERE will modify every row".to_string(),
                _ => return, // has WHERE — not dangerous
            };
            diagnostics.push(Diagnostic {
                message,
                start: offset,
                end: offset + stmt_str.len(),
                severity: 2,
            });
        }
    }
}

/// Extract byte position from pg_query error message.
/// pg_query error strings contain "position: N" (1-based character offset).
/// Returns (start, end) in the full source string.
fn extract_error_position(msg: &str, stmt_str: &str, offset: usize) -> (usize, usize) {
    // pg_query embeds cursor position as "position: N" (1-based)
    if let Some(pos_str) = msg.split("position: ").nth(1) {
        if let Ok(char_pos) = pos_str.trim().trim_end_matches(')').parse::<usize>() {
            // char_pos is 1-based; convert to 0-based byte offset
            let byte_pos = char_pos.saturating_sub(1).min(stmt_str.len());
            let start = offset + byte_pos;
            let end = (start + 1).min(offset + stmt_str.len());
            return (start, end);
        }
    }
    // Fallback: highlight the whole statement
    (offset, offset + stmt_str.len())
}

/// Clean up pg_query error message for display.
/// pg_query errors contain internal context; extract the human-readable part.
fn clean_pg_error(msg: &str) -> &str {
    // pg_query messages often start with "syntax error at or near..."
    // Strip trailing position info if present
    msg.split(" (position:").next().unwrap_or(msg).trim()
}
```

- [ ] **Step 5: Fix the public API access for `parse_postgres` and `ir`**

The `check_dangerous_scope` method calls `sql_scope::parser::postgres::parse_postgres` and `sql_scope::ir::DangerousKind`. These are internal modules. Expose them from `sql_scope/src/lib.rs`:

In `src-tauri/crates/sql-scope/src/lib.rs`, add:
```rust
pub use ir::DangerousKind;
pub use parser::postgres::parse_postgres as parse_postgres_stmt;
```

Then in `completion/diagnostics.rs`, update the call:
```rust
if let Some(stmt) = sql_scope::parse_postgres_stmt(stmt_str) {
    if let ParsedStatement::Dangerous { kind, has_where } = stmt {
        let message = match kind {
            sql_scope::DangerousKind::Drop =>
                "Destructive: DROP cannot be undone".to_string(),
            sql_scope::DangerousKind::Truncate =>
                "Destructive: TRUNCATE will delete all rows".to_string(),
            sql_scope::DangerousKind::DeleteWithoutWhere if !has_where =>
                "DELETE without WHERE will erase every row".to_string(),
            sql_scope::DangerousKind::UpdateWithoutWhere if !has_where =>
                "UPDATE without WHERE will modify every row".to_string(),
            _ => return,
        };
        ...
    }
}
```

- [ ] **Step 6: Update the `SchemaGraph` to implement `SchemaSnapshot`**

The `DiagnosticEngine::check` passes `schema: &SchemaGraph` to `sql_scope::resolve()`. Verify that `SchemaGraph` already implements `sql_scope::SchemaSnapshot` (it should from the existing Pass 5 code). If not, confirm the existing trait impl and that the right trait is used.

Check by searching:
```bash
cd src-tauri && grep -r "SchemaSnapshot" src/completion/ --include="*.rs"
```

If `SchemaGraph` doesn't implement it, check `schema/graph.rs` to see how the existing `sql_scope::resolve(source, ..., schema)` call passes it, and confirm the impl exists there.

- [ ] **Step 7: Run the new tests**

```bash
cd src-tauri && cargo test completion::diagnostics 2>&1 | tail -50
```
Expected: All `diagnostics::tests::*` pass including the new `test_analyze_*` and `test_pg_query_error_has_position` tests.

- [ ] **Step 8: Run all Rust tests**

```bash
cd src-tauri && cargo test 2>&1 | tail -60
```
Expected: All tests pass. Zero compile errors.

- [ ] **Step 9: Verify clippy is clean**

```bash
cd src-tauri && cargo clippy 2>&1 | grep -E "^error" | head -20
```
Expected: No errors (warnings about unused imports from removed tree-sitter code are OK to fix).

- [ ] **Step 10: Commit**

```bash
git add src-tauri/src/completion/diagnostics.rs src-tauri/crates/sql-scope/src/lib.rs
git commit -m "feat(diagnostics): pg_query as primary validator, tree-sitter removed from diagnostic path

- ANALYZE/VACUUM/INSERT/LOCK table refs now validated against schema
- Parse errors show pg_query's precise message with byte position
- Dangerous statement detection (DROP/TRUNCATE/DELETE/UPDATE) preserved via sql_scope IR
- False positive blanket suppression for utility statements removed"
```

---

## Verification

After all tasks are complete:

1. Open the SQL editor, type `ANALYZE production.tasksj;` → should show yellow squiggle on "tasksj" (unknown table warning)
2. Type `ANALYZE production.tasks;` with a connected schema that has `tasks` → no squiggles
3. Type `SELECT * FROM` (incomplete) → should show a syntax error with precise position
4. Type `DELETE FROM orders;` → should show "DELETE without WHERE" warning
5. Type `DROP TABLE foo;` → should show "DROP cannot be undone" warning
6. Run `cd src-tauri && cargo test` → all tests pass
