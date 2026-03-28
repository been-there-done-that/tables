# sql-scope Crate Design

**Date:** 2026-03-28
**Status:** Approved
**Scope:** New workspace crate `src-tauri/crates/sql-scope/` — foundation layer for DataGrip-level SQL scope resolution, CTE tracking, and multi-dialect diagnostics

---

## Problem Statement

The existing completion engine in `src-tauri/src/completion/` has four structural problems:

1. **Flaky query recognition when pasting** — context detection mixes AST traversal with raw string scanning; multi-statement pastes have no splitter so tree-sitter sees one giant partial query
2. **Sub-CTEs not in autocomplete** — `SemanticModel.ctes` is a flat global `HashMap`; CTE bodies are not entered as sub-scopes; `SELECT *` wildcards are never expanded
3. **False positive diagnostics** — tree-sitter ERROR nodes on valid PostgreSQL syntax (RETURNING, ::text casts, CONSTRAINT) reported as errors; special-case suppression list does not scale
4. **Ambiguity detection exists but is never called** — `check_column_ambiguity()` in `ambiguity.rs` is fully implemented but never wired into the completion or diagnostic pass

---

## Decisions

| Question | Decision |
|---|---|
| Dialect support | PostgreSQL + SQLite + MySQL. PostgreSQL first, SQLite + MySQL follow-up |
| Crate location | New workspace crate: `src-tauri/crates/sql-scope/`. Publish-ready structure; extract to separate repo when mature |
| Wildcard expansion depth | Best-effort recursive, max 5 hops, cycle-safe |
| Parser backends | `pg_query.rs` (FFI) for PostgreSQL complete-statement parsing + diagnostics; `sqlparser-rs` for SQLite/MySQL; tree-sitter (existing) for mid-type incomplete input across all dialects |
| DB calls | None. Crate is pure offline — takes SQL text + `SchemaSnapshot` as input |
| Integration | Foundation layer (B): existing `engine.rs` and `diagnostics.rs` call into `sql-scope` for symbol resolution. Full replacement (A) is a future step |

---

## Crate Structure

```
src-tauri/crates/sql-scope/
  Cargo.toml
  src/
    lib.rs              # Public API: resolve(), ScopeTree, VisibleSymbols
    dialect.rs          # Dialect enum (Postgres | SQLite | MySQL)
    error.rs            # ScopeError enum
    schema.rs           # SchemaSnapshot trait
    parser/
      mod.rs            # DialectParser trait
      postgres.rs       # pg_query.rs backend (complete) + tree-sitter handoff (incomplete)
      sqlite.rs         # sqlparser-rs SQLite dialect
      mysql.rs          # sqlparser-rs MySQL dialect
    scope/
      mod.rs
      tree.rs           # ScopeTree, Scope, ScopeId
      resolver.rs       # traverse_scope() — main algorithm
      cte.rs            # CTE registry, ordered processing, wildcard expansion
      symbol.rs         # Symbol, SymbolKind, ColumnRef, Source
    match.rs            # match_score() — fuzzy/acronym/prefix scoring
    join.rs             # JOIN condition inference (FK → heuristics → shared columns)
    types.rs            # SqlType enum, basic column type resolution
  tests/
    postgres_ctes.rs
    mysql_ctes.rs
    sqlite_ctes.rs
    wildcard_expansion.rs
    false_positive_diagnostics.rs
    fuzzy_match.rs
    join_inference.rs
    type_resolution.rs
```

---

## Core Types

### Scope

```rust
pub struct Scope {
    pub id: ScopeId,
    pub parent: Option<ScopeId>,
    pub scope_type: ScopeType,
    /// Local table refs and subquery aliases visible in this scope
    pub sources: HashMap<String, Source>,
    /// CTEs visible in this scope: inherited from parent + locally defined (in order)
    pub cte_sources: IndexMap<String, CteInfo>,  // IndexMap preserves insertion order
    /// Columns projected by this scope (for derived table / CTE consumers)
    pub columns: Vec<ColumnRef>,
}

pub enum ScopeType {
    Root,
    Cte { name: String },
    Subquery,
    DerivedTable { alias: String },
    Union,
}

pub enum Source {
    Table { schema: Option<String>, name: String },
    Cte { name: String },
    DerivedTable { scope_id: ScopeId },
    Alias { alias: String, target: Box<Source> },
}

pub struct CteInfo {
    pub scope_id: ScopeId,
    pub columns: Vec<String>,    // resolved, wildcard-expanded
    pub is_recursive: bool,
}
```

### SchemaSnapshot (trait)

```rust
/// Thin interface — the existing SchemaGraph implements this.
pub trait SchemaSnapshot {
    fn table_columns(&self, schema: Option<&str>, table: &str) -> Option<Vec<String>>;
    fn table_exists(&self, schema: Option<&str>, table: &str) -> bool;
    fn default_schema(&self) -> Option<&str>;
    fn foreign_keys(&self, schema: Option<&str>, table: &str) -> Vec<ForeignKey>;
    fn column_type(&self, schema: Option<&str>, table: &str, column: &str) -> Option<SqlType>;
}
```

### Public API

```rust
/// Split a multi-statement SQL string into individual statement strings.
/// Splits on `;` boundaries and double newlines. Used before calling resolve().
pub fn split_statements(sql: &str) -> Vec<&str>;

/// Resolve scope tree for a single SQL statement (complete or partial/mid-type).
/// Pass one element from split_statements() at a time.
/// Returns Err only on unrecoverable parse failure; partial results are always returned.
pub fn resolve(
    sql: &str,
    dialect: Dialect,
    schema: &dyn SchemaSnapshot,
) -> Result<ScopeTree, ScopeError>;

/// Query result from ScopeTree
impl ScopeTree {
    /// All symbols (tables, CTEs, columns, aliases) visible at a byte offset
    pub fn visible_at(&self, cursor_byte: usize) -> VisibleSymbols;

    /// Diagnostics: unknown tables, ambiguous columns, false-positive-suppressed syntax errors
    pub fn diagnostics(&self) -> Vec<ScopeDiagnostic>;

    /// The innermost scope containing the cursor
    pub fn scope_at(&self, cursor_byte: usize) -> Option<&Scope>;
}
```

---

## `traverse_scope()` Algorithm

Direct port of [sqlglot's `traverse_scope`](https://sqlglot.com/sqlglot/optimizer/scope.html) to Rust. Operates on the parsed AST from the dialect-specific backend.

```
fn traverse_scope(stmt, schema) -> ScopeTree:
  1. Create root scope (inherits no CTEs)

  2. If stmt has WITH clause:
     For each CTE in declaration order (index 0..N):
       a. Create child scope for CTE body
       b. Child.cte_sources = parent.cte_sources
                            + CTEs[0..current_index]  (CTEs defined before this one)
          (NOT including itself, unless RECURSIVE)
       c. If RECURSIVE: add self-reference as initial source in child scope
       d. Recursively traverse CTE body (handles nested WITH inside CTE)
       e. Resolve CTE columns:
            - If explicit column list: use as-is
            - Else: project from body SELECT list
                - If SELECT *: expand wildcards (see Wildcard Expansion)
                - Else: collect aliases and expressions
       f. Register CteInfo in parent.cte_sources (now visible to subsequent CTEs)

  3. Process main SELECT body in root scope:
     a. Register FROM sources (tables, joined tables, subqueries)
     b. For each subquery / derived table: recurse with current scope as parent
     c. Collect projected columns into scope.columns

  4. Return ScopeTree (post-order DFS — children before parents)
```

### Wildcard Expansion

```
fn expand_wildcard(source, schema, scope_tree, depth) -> Vec<String>:
  if depth > 5: return []           // cycle / depth guard
  if source is SchemaTable:
    return schema.table_columns(source)
  if source is CTE:
    cte_info = scope.cte_sources[name]
    if cte_info.columns non-empty: return cte_info.columns
    // CTE itself had SELECT * — recurse into CTE scope
    return expand_wildcard(cte_scope.sources, schema, scope_tree, depth + 1)
  if source is DerivedTable:
    child_scope = scope_tree.scope(scope_id)
    return child_scope.columns  // already resolved during child traversal
  return []
```

---

## Parser Backends

### PostgreSQL (`parser/postgres.rs`)

Two-phase approach:

**Phase 1 — Statement splitting:**
Split input on `;` boundaries and double newlines before any parsing. Each statement is analysed independently. A syntax error in statement N does not corrupt scope detection in statement N+1.

**Phase 2 — Per-statement parsing:**

| Input state | Parser used | Output |
|---|---|---|
| Complete statement (no tree-sitter errors) | `pg_query::parse()` | Protobuf AST → `traverse_scope()` |
| Incomplete / mid-type (tree-sitter ERROR nodes) | tree-sitter (existing, unchanged) | Existing completion engine handles |
| Complete but pg_query fails | `pg_query` error message | Real diagnostic reported |
| Complete, tree-sitter ERROR, pg_query succeeds | pg_query AST | ERROR node suppressed (false positive) |

CTE column extraction uses `CommonTableExpr.ctecolnames` from the pg_query protobuf tree — no heuristic SELECT-list scanning.

### SQLite (`parser/sqlite.rs`) and MySQL (`parser/mysql.rs`)

Use `sqlparser-rs` with the appropriate dialect flag. `sqlparser-rs` is strict (fails on invalid SQL) — used only for complete statements, same gating logic as PostgreSQL Phase 2 (without pg_query).

For mid-type input: fall through to existing tree-sitter path unchanged.

---

## Diagnostic Pass

After `traverse_scope()` completes, the diagnostic pass runs over the `ScopeTree`:

1. **Unknown table reference** — source in `scope.sources` not found in `schema.table_exists()` and not in `scope.cte_sources` → Warning
2. **Unknown CTE reference** — CTE name used but not in `scope.cte_sources` at that point in order → Error
3. **Ambiguous column reference** — column name exists in 2+ visible sources → Warning (uses existing `check_column_ambiguity()` — finally wired in)
4. **False positive suppression (PostgreSQL)** — tree-sitter ERROR node present + `pg_query::parse()` succeeds → suppress, do not report
5. **Real syntax error** — `pg_query::parse()` fails → report pg_query's error message (more precise than tree-sitter's)
6. **Dangerous statement warning** — DROP/TRUNCATE/DELETE without WHERE — kept from existing diagnostics.rs but moved into scope-aware context (no string scanning)

---

## Integration with Existing Engine

No breaking changes to the existing `src-tauri/src/completion/` code in the initial integration. The new crate is added as a dependency and called at two points:

### In `engine.rs` (completions)
```rust
// Replace direct SemanticModel usage with ScopeTree
let scope_tree = sql_scope::resolve(sql, dialect, &schema_graph)?;
let visible = scope_tree.visible_at(cursor_byte);

// visible.tables   → feed into FromClause completions
// visible.ctes     → feed into FromClause + AfterDot completions
// visible.columns  → feed into SelectClause + WhereClause completions
```

### In `diagnostics.rs`
```rust
// Replace existing diagnostic checks with scope-aware pass
let scope_tree = sql_scope::resolve(sql, dialect, &schema_graph)?;
let diagnostics = scope_tree.diagnostics();
// Map ScopeDiagnostic → existing Diagnostic DTO for IPC
```

The existing `SemanticModel`, `schema_graph.rs`, `context.rs` remain in place during the transition. They can be incrementally removed as the new crate proves stable.

---

## Workspace Changes

```toml
# src-tauri/Cargo.toml — add member
[workspace]
members = [".", "crates/sql-scope"]

# src-tauri/Cargo.toml — add dependency
[dependencies]
sql-scope = { path = "crates/sql-scope" }
```

```toml
# src-tauri/crates/sql-scope/Cargo.toml
[package]
name = "sql-scope"
version = "0.1.0"
edition = "2021"
description = "Multi-dialect SQL scope resolver and diagnostic engine"
license = "MIT"

[dependencies]
pg_query = "6.1"           # PostgreSQL FFI backend
sqlparser = "0.59"         # SQLite + MySQL backend
indexmap = "2"             # Ordered CTE registry
tree-sitter = "0.22"       # Shared with parent crate (incomplete input)
tree-sitter-sequel = "0.3" # SQL grammar

[dev-dependencies]
pretty_assertions = "1"
```

---

## Testing Strategy

Each dialect gets its own test file. Key scenarios to cover:

- Simple CTE referenced in main query
- CTE referencing a prior CTE in the same WITH clause
- CTE referencing a subsequent CTE (should NOT resolve — error)
- Nested WITH inside a CTE body (sub-CTE)
- Recursive CTE (`WITH RECURSIVE`)
- `SELECT *` wildcard expansion — schema table source
- `SELECT *` wildcard expansion — CTE source (recursive resolution)
- `SELECT *` wildcard expansion — 5-hop depth limit
- Derived table alias (`FROM (SELECT ...) AS t`)
- Ambiguous column reference across two joined tables
- False positive suppression: valid PostgreSQL syntax that tree-sitter marks as ERROR
- Multi-statement SQL: scope isolation between statements
- Incomplete SQL (mid-type): graceful partial result, no panic

---

## Additional Features (In Scope)

### Fuzzy / Acronym Completion Matching

A pure scoring utility moved from `engine.rs` into `sql-scope`:

```rust
/// Score how well `input` matches `candidate`. Higher is better.
/// Handles prefix match, acronym match (e.g. "ui" → "user_id"), substring match.
pub fn match_score(input: &str, candidate: &str) -> u32;
```

Lives in `src/match.rs`. No dependencies. Replaces the current `starts_with`-only filter in `engine.rs`.

### JOIN Condition Inference

Moved from existing FK heuristic code into `sql-scope`. Requires extending `SchemaSnapshot`:

```rust
pub trait SchemaSnapshot {
    fn table_columns(&self, schema: Option<&str>, table: &str) -> Option<Vec<String>>;
    fn table_exists(&self, schema: Option<&str>, table: &str) -> bool;
    fn default_schema(&self) -> Option<&str>;
    // New:
    fn foreign_keys(&self, schema: Option<&str>, table: &str) -> Vec<ForeignKey>;
}

pub struct ForeignKey {
    pub from_column: String,
    pub to_table: String,
    pub to_column: String,
}
```

3-tier inference stays the same (FK graph → naming heuristics → shared column names), just lives in `src/join.rs` instead of `engine.rs`.

### Basic Type Resolution (v1)

Resolve column types from the schema snapshot. Enables type-mismatch diagnostics (`WHERE age = 'foo'`).

```rust
pub enum SqlType {
    Integer, BigInt, Float, Text, Boolean, Date, Timestamp, Uuid, Json, Unknown,
}

impl ScopeTree {
    /// Resolve the type of a column reference at the given cursor position.
    pub fn column_type(&self, col: &ColumnRef) -> SqlType;
}
```

Requires extending `SchemaSnapshot`:
```rust
fn column_type(&self, schema: Option<&str>, table: &str, column: &str) -> Option<SqlType>;
```

**v1 scope:** column type lookup from schema only — no expression type propagation, no function return types.
**v2 (future):** full expression type inference, PostgreSQL coercion rules, function signatures.

---

## Out of Scope (This Crate)

- Full expression type inference / PostgreSQL coercion rules (v2 follow-up)
- Full replacement of `engine.rs` / `diagnostics.rs` (follow-up, Option A)
- MongoDB / Redis (not SQL dialects)
