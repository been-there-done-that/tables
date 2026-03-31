# Option C: Full SemanticModel Removal Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace `SemanticModel` (tree-sitter flat HashMap) with `ScopeTree` (sql-scope crate, ordered CTE visibility) as the sole driver of completions. Delete all legacy code — no backward compatibility shims.

**Architecture:** `completion_commands.rs` calls `sql_scope::resolve()` → `ScopeTree`. `ScopeTree` is passed down through `CompletionEngineVariant::complete()` → `CoreCompletionEngine` helpers, replacing every `semantic.*` call. The `analysis/` module (1,121 lines) and the duplicate `engine.rs` (736 lines) are both deleted.

**Tech Stack:** `sql-scope` crate (already a workspace dependency), `sql_scope::ScopeTree`, `sql_scope::Source`, `sql_scope::VisibleSymbols`

---

## API Translation Reference

| SemanticModel API | ScopeTree API |
|---|---|
| `semantic.visible_symbols_at(cursor)` | `scope_tree.visible_at(cursor).sources` → `Vec<(String, Source)>` |
| `sym.name` (alias) | `alias` — the `String` in the tuple |
| `sym.resolve_table_name()` | `resolve_table_name(&source)` helper (defined in Task 1) |
| `semantic.ctes.get(&name)` | `get_cte_columns(scope_tree, cursor, name)` helper (defined in Task 1) |
| `semantic.resolve_at_cursor(cursor, alias)` | `scope_tree.visible_at(cursor).get_source(alias)` |
| `SymbolKind::CTE { cte_name }` | `Source::Cte { name }` |
| `for scope in &semantic.scopes { scope.find_symbol(name) }` | `scope_tree.visible_at(usize::MAX).get_source(name)` |

**`resolve_table_name` helper** — add to `core.rs` in Task 1:

```rust
fn resolve_table_name(source: &sql_scope::Source) -> Option<&str> {
    match source {
        sql_scope::Source::Table { name, .. } => Some(name.as_str()),
        sql_scope::Source::Cte { name } => Some(name.as_str()),
        sql_scope::Source::Alias { target, .. } => resolve_table_name(target),
        sql_scope::Source::DerivedTable { .. } => None,
    }
}
```

**`get_cte_columns` helper** — add to `core.rs` in Task 1:

```rust
fn get_cte_columns<'a>(scope_tree: &'a sql_scope::ScopeTree, cursor: usize, cte_name: &str) -> &'a [String] {
    scope_tree
        .scope_at(cursor)
        .and_then(|s| s.cte_sources.get(cte_name))
        .map(|info| info.columns.as_slice())
        .unwrap_or(&[])
}
```

**`resolve_alias_to_table` helper** — replaces old `resolve_table_name_from_alias` free function:

```rust
fn resolve_alias_to_table(alias: &str, scope_tree: &sql_scope::ScopeTree) -> Option<String> {
    scope_tree.visible_at(usize::MAX).get_source(alias)
        .and_then(|s| resolve_table_name(s))
        .map(|s| s.to_string())
}
```

---

## File Map

**Modified:**
- `src/completion/engines/mod.rs` — change `CompletionEngineVariant::complete()` signature
- `src/completion/engines/core.rs` — replace all `SemanticModel` usage + update `filter_by_prefix` to use `match_score`
- `src/completion/engines/postgres.rs` — update signature, pass `scope_tree` to Core
- `src/completion/engines/sqlite.rs` — same as postgres
- `src/commands/completion_commands.rs` — replace `build_semantic_model` with `sql_scope::resolve`
- `src/completion/mod.rs` — remove `pub mod analysis` and `pub mod engine`
- `src/completion/tests.rs` — full rewrite to use `PostgresEngine` + `ScopeTree` (no SemanticModel)

**Deleted (clean slate — no shims):**
- `src/completion/analysis/scope.rs`
- `src/completion/analysis/builder.rs`
- `src/completion/analysis/ambiguity.rs`
- `src/completion/analysis/mod.rs`
- `src/completion/engine.rs` — duplicate of `core.rs` logic, only used by tests; deleted and tests rewritten

---

## Task 1: Add helpers and update `CompletionEngineVariant` trait

**Files:**
- Modify: `src/completion/engines/mod.rs`
- Modify: `src/completion/engines/core.rs` (helpers only, no logic change yet)

### Context

`CompletionEngineVariant` trait currently:

```rust
// engines/mod.rs line 41
fn complete(
    &self,
    semantic: &SemanticModel,
    context: &Context,
    ...
) -> Vec<CompletionItem>;
```

- [ ] **Step 1: Write the failing test**

Add to the bottom of `src/completion/engines/core.rs`:

```rust
#[cfg(test)]
mod helper_tests {
    use super::*;

    #[test]
    fn resolve_table_name_from_table_source() {
        let src = sql_scope::Source::Table { schema: None, name: "users".to_string() };
        assert_eq!(resolve_table_name(&src), Some("users"));
    }

    #[test]
    fn resolve_table_name_from_cte_source() {
        let src = sql_scope::Source::Cte { name: "my_cte".to_string() };
        assert_eq!(resolve_table_name(&src), Some("my_cte"));
    }

    #[test]
    fn resolve_table_name_unwraps_alias() {
        let src = sql_scope::Source::Alias {
            alias: "u".to_string(),
            target: Box::new(sql_scope::Source::Table { schema: None, name: "users".to_string() }),
        };
        assert_eq!(resolve_table_name(&src), Some("users"));
    }

    #[test]
    fn resolve_table_name_derived_is_none() {
        let src = sql_scope::Source::DerivedTable { scope_id: 0 };
        assert_eq!(resolve_table_name(&src), None);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd src-tauri && cargo test -p tables helper_tests 2>&1 | head -20
```

Expected: FAIL — `resolve_table_name` not defined.

- [ ] **Step 3: Add helpers to `core.rs`**

Add these three private functions at the bottom of `src/completion/engines/core.rs`, replacing the old `resolve_table_name_from_alias` free function (delete the old one):

```rust
/// Extract the table/CTE name from a Source, recursively unwrapping Alias.
fn resolve_table_name(source: &sql_scope::Source) -> Option<&str> {
    match source {
        sql_scope::Source::Table { name, .. } => Some(name.as_str()),
        sql_scope::Source::Cte { name } => Some(name.as_str()),
        sql_scope::Source::Alias { target, .. } => resolve_table_name(target),
        sql_scope::Source::DerivedTable { .. } => None,
    }
}

/// Get the column list for a CTE visible at the cursor position.
fn get_cte_columns<'a>(scope_tree: &'a sql_scope::ScopeTree, cursor: usize, cte_name: &str) -> &'a [String] {
    scope_tree
        .scope_at(cursor)
        .and_then(|s| s.cte_sources.get(cte_name))
        .map(|info| info.columns.as_slice())
        .unwrap_or(&[])
}

/// Resolve an alias to its actual table/CTE name using the full scope tree.
fn resolve_alias_to_table(alias: &str, scope_tree: &sql_scope::ScopeTree) -> Option<String> {
    scope_tree.visible_at(usize::MAX)
        .get_source(alias)
        .and_then(|s| resolve_table_name(s))
        .map(|s| s.to_string())
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cargo test -p tables helper_tests 2>&1 | tail -5
```

Expected: 4 tests pass.

- [ ] **Step 5: Update `CompletionEngineVariant` trait in `engines/mod.rs`**

Replace the import and the trait method:

```rust
// REMOVE:
use crate::completion::analysis::SemanticModel;

// In the trait — replace the complete() signature:
fn complete(
    &self,
    scope_tree: &sql_scope::ScopeTree,
    context: &Context,
    schema: &SchemaGraph,
    default_schema: Option<&str>,
    capabilities: Option<&DatabaseCapabilities>,
) -> Vec<CompletionItem>;
```

At this point `cargo build` will emit errors in `postgres.rs` and `sqlite.rs` — expected.

- [ ] **Step 6: Commit**

```bash
git add src/completion/engines/mod.rs src/completion/engines/core.rs
git commit -m "feat(completion): add resolve_table_name/get_cte_columns helpers, update CompletionEngineVariant trait to ScopeTree"
```

---

## Task 2: Rewrite `CoreCompletionEngine` to use `ScopeTree` + upgrade `filter_by_prefix`

**Files:**
- Modify: `src/completion/engines/core.rs`

### Context

`core.rs` has 7 methods using `SemanticModel` and one `filter_by_prefix` still using `starts_with` (not `match_score`). Both are fixed in this task.

Methods to update:
- `complete_after_dot` — `semantic.resolve_at_cursor()` + `semantic.ctes.get()`
- `complete_table_names` — `semantic.visible_symbols_at()` for CTEs
- `complete_join_condition` — `resolve_table_name_from_alias()` (old free fn, deleted in Task 1)
- `complete_where_clause` — `semantic.visible_symbols_at()`
- `complete_select_clause` — `semantic.visible_symbols_at()` + `semantic.ctes.get()`
- `complete_function_argument` — `semantic.visible_symbols_at()`
- `complete_generic` — `semantic.visible_symbols_at()`
- `filter_by_prefix` — upgrade from `starts_with` to `sql_scope::match_score()`

- [ ] **Step 1: Write the failing tests**

Add this block to `core.rs`:

```rust
#[cfg(test)]
mod scope_tree_method_tests {
    use super::*;
    use sql_scope::{ScopeTree, Source};
    use sql_scope::scope::tree::{Scope, ScopeType, CteInfo};
    use crate::completion::context::{Context, CursorContext};
    use crate::completion::schema::loader::MockSchemaLoader;

    fn tree_with_alias(alias: &str, table: &str) -> ScopeTree {
        let mut tree = ScopeTree::new();
        let mut scope = Scope::new(0, None, ScopeType::Root, 0..1000);
        scope.sources.insert(alias.to_string(), Source::Alias {
            alias: alias.to_string(),
            target: Box::new(Source::Table { schema: None, name: table.to_string() }),
        });
        tree.add_scope(scope);
        tree
    }

    fn tree_with_cte(cte_name: &str, cols: Vec<&str>) -> ScopeTree {
        let mut tree = ScopeTree::new();
        let mut scope = Scope::new(0, None, ScopeType::Root, 0..1000);
        scope.cte_sources.insert(cte_name.to_string(), CteInfo {
            scope_id: 0,
            columns: cols.into_iter().map(|s| s.to_string()).collect(),
            is_recursive: false,
        });
        tree.add_scope(scope);
        tree
    }

    fn ctx(cursor: usize, ctx_type: CursorContext) -> Context {
        Context { cursor_offset: cursor, context_type: ctx_type, prefix: String::new(), previous_word: String::new(), scope_depth: 0 }
    }

    #[test]
    fn after_dot_resolves_schema_table_alias() {
        let schema = MockSchemaLoader::create_test_schema();
        let tree = tree_with_alias("u", "users");
        let items = CoreCompletionEngine::complete_after_dot("u", &tree, &ctx(20, CursorContext::AfterDot { alias: "u".to_string() }), &schema);
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(labels.contains(&"id"));
        assert!(labels.contains(&"email"));
    }

    #[test]
    fn after_dot_resolves_cte_columns() {
        let schema = MockSchemaLoader::create_test_schema();
        let mut tree = ScopeTree::new();
        let mut scope = Scope::new(0, None, ScopeType::Root, 0..1000);
        scope.cte_sources.insert("orders_cte".to_string(), CteInfo {
            scope_id: 0,
            columns: vec!["order_id".to_string(), "total".to_string()],
            is_recursive: false,
        });
        // alias "oc" → CTE "orders_cte"
        scope.sources.insert("oc".to_string(), Source::Cte { name: "orders_cte".to_string() });
        tree.add_scope(scope);
        let items = CoreCompletionEngine::complete_after_dot("oc", &tree, &ctx(20, CursorContext::AfterDot { alias: "oc".to_string() }), &schema);
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(labels.contains(&"order_id"), "should return CTE columns");
        assert!(labels.contains(&"total"));
    }

    #[test]
    fn table_names_includes_cte() {
        let schema = MockSchemaLoader::create_test_schema();
        let tree = tree_with_cte("my_cte", vec!["id", "name"]);
        let items = CoreCompletionEngine::complete_table_names(&schema, &tree, &ctx(50, CursorContext::FromClause), None, &[]);
        assert!(items.iter().any(|i| i.label == "my_cte"), "CTE must appear in FROM completions");
    }

    #[test]
    fn filter_by_prefix_uses_match_score_acronym() {
        let mut items = vec![
            CompletionItem { label: "user_id".to_string(), kind: CompletionKind::Column, detail: None, insert_text: "user_id".to_string(), score: 50 },
            CompletionItem { label: "order_total".to_string(), kind: CompletionKind::Column, detail: None, insert_text: "order_total".to_string(), score: 50 },
        ];
        CoreCompletionEngine::filter_by_prefix(&mut items, "ui");
        // "ui" is an acronym for "user_id" — should match
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].label, "user_id");
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test -p tables scope_tree_method_tests 2>&1 | head -30
```

Expected: compile errors (methods still take `&SemanticModel`).

- [ ] **Step 3: Replace the import at top of `core.rs`**

```rust
// REMOVE:
use crate::completion::analysis::{SemanticModel, SymbolKind};
// ADD:
use sql_scope::{Source, ScopeTree};
```

- [ ] **Step 4: Rewrite `complete_after_dot`**

```rust
pub fn complete_after_dot(
    alias: &str,
    scope_tree: &ScopeTree,
    context: &Context,
    schema: &SchemaGraph,
) -> Vec<CompletionItem> {
    let mut items = Vec::new();

    log::debug!("[AfterDot] Looking up '{}' at cursor offset {}", alias, context.cursor_offset);

    let visible = scope_tree.visible_at(context.cursor_offset);
    if let Some(source) = visible.get_source(alias) {
        log::debug!("[AfterDot] Resolved source: {:?}", source);

        if let Source::Cte { name: cte_name } = source {
            // CTE alias — look up columns from scope tree
            let cte_cols = get_cte_columns(scope_tree, context.cursor_offset, cte_name);
            log::debug!("[AfterDot] Found {} CTE columns for '{}'", cte_cols.len(), cte_name);
            for col_name in cte_cols {
                items.push(CompletionItem {
                    label: col_name.clone(),
                    kind: CompletionKind::Column,
                    detail: Some(format!("CTE Column ({})", cte_name)),
                    insert_text: col_name.clone(),
                    score: 90,
                });
            }
        } else if let Some(table_name) = resolve_table_name(source) {
            log::debug!("[AfterDot] Table name: '{}'", table_name);
            let columns = schema.get_columns(table_name);
            log::debug!("[AfterDot] Found {} columns", columns.len());
            for col in columns {
                items.push(CompletionItem {
                    label: col.name.clone(),
                    kind: CompletionKind::Column,
                    detail: Some(format!("{} ({})", col.data_type, table_name)),
                    insert_text: col.name.clone(),
                    score: Self::column_score(col.is_primary_key, col.is_indexed),
                });
            }
        }
    }

    // If no alias match, check if this is a schema name (e.g., public.|)
    if items.is_empty() {
        log::debug!("[AfterDot] No alias match, checking if '{}' is a schema name", alias);
        let alias_lower = alias.to_lowercase();
        let schema_tables: Vec<_> = schema.tables.values()
            .filter(|t| t.schema.to_lowercase() == alias_lower)
            .collect();
        for table in schema_tables {
            items.push(CompletionItem {
                label: table.name.clone(),
                kind: CompletionKind::Table,
                detail: Some(table.schema.clone()),
                insert_text: table.name.clone(),
                score: SCORE_CURSOR_RELEVANCE + SCORE_UI_SCHEMA_HINT,
            });
        }
    }

    Self::filter_by_prefix(&mut items, &context.prefix);
    items.sort_by(|a, b| b.score.cmp(&a.score));
    items
}
```

- [ ] **Step 5: Rewrite `complete_table_names`**

```rust
pub fn complete_table_names(
    schema: &SchemaGraph,
    scope_tree: &ScopeTree,
    context: &Context,
    default_schema: Option<&str>,
    from_keywords: &[&str],
) -> Vec<CompletionItem> {
    let mut items = Vec::new();
    let mut seen_schemas = HashSet::new();

    let ui_schema = default_schema.unwrap_or("public");

    // 1. Schema suggestions
    for table_info in schema.tables.values() {
        if seen_schemas.insert(table_info.schema.clone()) {
            let score = Self::calculate_schema_score(&table_info.schema, ui_schema);
            items.push(CompletionItem {
                label: table_info.schema.clone(),
                kind: CompletionKind::Schema,
                detail: Some("schema".to_string()),
                insert_text: format!("{}.", table_info.schema),
                score,
            });
        }
    }

    // 2. Schema tables
    for table_info in schema.tables.values() {
        let is_ui_schema = table_info.schema == ui_schema;
        let is_public = table_info.schema == "public";
        let is_main = table_info.schema == "main";

        let mut score: i32 = SCORE_CURSOR_RELEVANCE as i32;
        if is_ui_schema {
            score += SCORE_UI_SCHEMA_HINT as i32;
        } else if is_public || is_main {
            score += SCORE_PUBLIC_SCHEMA as i32;
        } else {
            score += PENALTY_CROSS_SCHEMA;
        }

        let insert_text = Self::qualify_table_name(&table_info.schema, &table_info.name, ui_schema);
        let label = if is_ui_schema || is_public || is_main {
            table_info.name.clone()
        } else {
            format!("{} ({})", table_info.name, table_info.schema)
        };

        items.push(CompletionItem {
            label,
            kind: CompletionKind::Table,
            detail: Some(table_info.schema.clone()),
            insert_text,
            score: score.max(0) as u32,
        });
    }

    // 3. CTEs visible at cursor
    for (_alias, source) in &scope_tree.visible_at(context.cursor_offset).sources {
        if let Source::Cte { name: cte_name } = source {
            items.push(CompletionItem {
                label: cte_name.clone(),
                kind: CompletionKind::Table,
                detail: Some("CTE".to_string()),
                insert_text: cte_name.clone(),
                score: SCORE_CURSOR_RELEVANCE + SCORE_CTE,
            });
        }
    }

    // 4. FROM keywords
    for kw in from_keywords {
        items.push(CompletionItem {
            label: kw.to_string(),
            kind: CompletionKind::Keyword,
            detail: None,
            insert_text: kw.to_string(),
            score: 40,
        });
    }

    Self::filter_by_prefix(&mut items, &context.prefix);
    items.sort_by(|a, b| b.score.cmp(&a.score));
    items
}
```

- [ ] **Step 6: Rewrite `complete_join_condition`**

```rust
pub fn complete_join_condition(
    left_table: &Option<String>,
    right_table: &Option<String>,
    scope_tree: &ScopeTree,
    schema: &SchemaGraph,
) -> Vec<CompletionItem> {
    let mut items = Vec::new();

    if let (Some(left), Some(right)) = (left_table, right_table) {
        let left_resolved = resolve_alias_to_table(left, scope_tree).unwrap_or_else(|| left.clone());
        let right_resolved = resolve_alias_to_table(right, scope_tree).unwrap_or_else(|| right.clone());

        if let Some((condition, score)) = schema.infer_join_condition(
            &left_resolved, &right_resolved, Some(left), Some(right)
        ) {
            items.push(CompletionItem {
                label: condition.clone(),
                kind: CompletionKind::JoinCondition,
                detail: Some(format!("confidence: {}%", score)),
                insert_text: condition,
                score,
            });
        }
    }

    if let Some(left) = left_table {
        let left_resolved = resolve_alias_to_table(left, scope_tree).unwrap_or_else(|| left.clone());
        for col in schema.get_columns(&left_resolved) {
            items.push(CompletionItem {
                label: format!("{}.{}", left, col.name),
                kind: CompletionKind::Column,
                detail: Some(col.data_type.clone()),
                insert_text: format!("{}.{}", left, col.name),
                score: Self::column_score(col.is_primary_key, col.is_indexed) / 2,
            });
        }
    }

    items.sort_by(|a, b| b.score.cmp(&a.score));
    items
}
```

- [ ] **Step 7: Rewrite `complete_where_clause`**

```rust
pub fn complete_where_clause(
    scope_tree: &ScopeTree,
    context: &Context,
    schema: &SchemaGraph,
    where_keywords: &[&str],
    where_functions: &[&str],
    operators: &[(&str, &str, u32)],
) -> Vec<CompletionItem> {
    let mut items = Vec::new();

    let prev = context.previous_word.to_uppercase();
    let prev_is_keyword = ["WHERE", "AND", "OR", "NOT", "(", ","].contains(&prev.as_str());
    let prev_is_operator = operators.iter().any(|(op, _, _)| op.eq_ignore_ascii_case(&prev));

    if !prev_is_keyword && !prev_is_operator && !prev.is_empty() {
        for (op, detail, score) in operators {
            items.push(CompletionItem {
                label: op.to_string(),
                kind: CompletionKind::Operator,
                detail: Some(detail.to_string()),
                insert_text: op.to_string(),
                score: *score,
            });
        }
    }

    let show_columns = prev_is_keyword || prev_is_operator || prev.is_empty() || !context.prefix.is_empty();

    if show_columns {
        for (alias, source) in &scope_tree.visible_at(context.cursor_offset).sources {
            if let Some(table_name) = resolve_table_name(source) {
                for col in schema.get_columns(table_name) {
                    let qualified_name = format!("{}.{}", alias, col.name);
                    items.push(CompletionItem {
                        label: qualified_name.clone(),
                        kind: CompletionKind::Column,
                        detail: Some(col.data_type.clone()),
                        insert_text: qualified_name,
                        score: Self::column_score(col.is_primary_key, col.is_indexed),
                    });
                }
            }
        }
        for func in where_functions {
            items.push(CompletionItem {
                label: func.to_string(),
                kind: CompletionKind::Function,
                detail: Some("function".to_string()),
                insert_text: format!("{}()", func),
                score: 60,
            });
        }
        for kw in where_keywords {
            items.push(CompletionItem {
                label: kw.to_string(),
                kind: CompletionKind::Keyword,
                detail: None,
                insert_text: kw.to_string(),
                score: 50,
            });
        }
    }

    Self::filter_by_prefix(&mut items, &context.prefix);
    items.sort_by(|a, b| b.score.cmp(&a.score));
    items
}
```

- [ ] **Step 8: Rewrite `complete_select_clause`**

```rust
pub fn complete_select_clause(
    scope_tree: &ScopeTree,
    context: &Context,
    schema: &SchemaGraph,
    select_keywords: &[&str],
    select_functions: &[&str],
) -> Vec<CompletionItem> {
    let mut items = Vec::new();
    let mut seen_labels = HashSet::new();

    for (alias, source) in &scope_tree.visible_at(context.cursor_offset).sources {
        if seen_labels.insert(alias.clone()) {
            items.push(CompletionItem {
                label: alias.clone(),
                kind: CompletionKind::Alias,
                detail: resolve_table_name(source).map(|t| format!("alias for {}", t)),
                insert_text: format!("{}.", alias),
                score: 80,
            });
        }

        if let Source::Cte { name: cte_name } = source {
            for col_name in get_cte_columns(scope_tree, context.cursor_offset, cte_name) {
                if seen_labels.insert(col_name.clone()) {
                    items.push(CompletionItem {
                        label: col_name.clone(),
                        kind: CompletionKind::Column,
                        detail: Some(format!("CTE Column ({})", cte_name)),
                        insert_text: col_name.clone(),
                        score: 70,
                    });
                }
            }
        } else if let Some(table_name) = resolve_table_name(source) {
            for col in schema.get_columns(table_name) {
                if seen_labels.insert(col.name.clone()) {
                    items.push(CompletionItem {
                        label: col.name.clone(),
                        kind: CompletionKind::Column,
                        detail: Some(format!("{} ({})", col.data_type, alias)),
                        insert_text: col.name.clone(),
                        score: 70,
                    });
                }
            }
        }
    }

    items.push(CompletionItem {
        label: "*".to_string(),
        kind: CompletionKind::Keyword,
        detail: Some("All columns".to_string()),
        insert_text: "*".to_string(),
        score: 90,
    });

    for func in select_functions {
        if seen_labels.insert(func.to_string()) {
            items.push(CompletionItem {
                label: func.to_string(),
                kind: CompletionKind::Function,
                detail: Some("function".to_string()),
                insert_text: format!("{}()", func),
                score: 60,
            });
        }
    }

    for kw in select_keywords {
        if seen_labels.insert(kw.to_string()) {
            let mut score = 50;
            if matches!(context.context_type, CursorContext::AfterSelectList) {
                if *kw == "FROM" { score = 200; }
                else if *kw == "AS" { score = 150; }
            }
            items.push(CompletionItem {
                label: kw.to_string(),
                kind: CompletionKind::Keyword,
                detail: None,
                insert_text: kw.to_string(),
                score,
            });
        }
    }

    Self::filter_by_prefix(&mut items, &context.prefix);
    items.sort_by(|a, b| b.score.cmp(&a.score));
    items
}
```

- [ ] **Step 9: Rewrite `complete_function_argument`**

```rust
pub fn complete_function_argument(
    function_name: &str,
    scope_tree: &ScopeTree,
    context: &Context,
    schema: &SchemaGraph,
) -> Vec<CompletionItem> {
    let mut items = Vec::new();

    let numeric_only = matches!(
        function_name.to_uppercase().as_str(),
        "SUM" | "AVG"
    );

    for (alias, source) in &scope_tree.visible_at(context.cursor_offset).sources {
        if let Some(table_name) = resolve_table_name(source) {
            for col in schema.get_columns(table_name) {
                if numeric_only {
                    let type_lower = col.data_type.to_lowercase();
                    if !type_lower.contains("int")
                       && !type_lower.contains("decimal")
                       && !type_lower.contains("numeric")
                       && !type_lower.contains("float")
                       && !type_lower.contains("double")
                    {
                        continue;
                    }
                }
                let qualified_name = format!("{}.{}", alias, col.name);
                items.push(CompletionItem {
                    label: qualified_name.clone(),
                    kind: CompletionKind::Column,
                    detail: Some(col.data_type.clone()),
                    insert_text: qualified_name,
                    score: Self::column_score(col.is_primary_key, col.is_indexed),
                });
            }
        }
    }

    Self::filter_by_prefix(&mut items, &context.prefix);
    items.sort_by(|a, b| b.score.cmp(&a.score));
    items
}
```

- [ ] **Step 10: Rewrite `complete_generic`**

```rust
pub fn complete_generic(
    scope_tree: &ScopeTree,
    context: &Context,
    generic_keywords: &[&str],
) -> Vec<CompletionItem> {
    let mut items = Vec::new();

    for kw in generic_keywords {
        let score = match *kw {
            "SELECT" => 200,
            "INSERT" | "UPDATE" | "DELETE" => 90,
            "WHERE" | "FROM" | "JOIN" => 80,
            _ => 50,
        };
        items.push(CompletionItem {
            label: kw.to_string(),
            kind: CompletionKind::Keyword,
            detail: None,
            insert_text: kw.to_string(),
            score,
        });
    }

    for (alias, source) in &scope_tree.visible_at(context.cursor_offset).sources {
        items.push(CompletionItem {
            label: alias.clone(),
            kind: CompletionKind::Alias,
            detail: resolve_table_name(source).map(|t| t.to_string()),
            insert_text: alias.clone(),
            score: 70,
        });
    }

    Self::filter_by_prefix(&mut items, &context.prefix);
    items.sort_by(|a, b| b.score.cmp(&a.score));
    items
}
```

- [ ] **Step 11: Upgrade `filter_by_prefix` to use `match_score`**

Replace the existing `filter_by_prefix` method:

```rust
pub fn filter_by_prefix(items: &mut Vec<CompletionItem>, prefix: &str) {
    if prefix.is_empty() {
        return;
    }
    items.retain(|item| sql_scope::match_score(prefix, &item.label) > 0);
    items.sort_by(|a, b| {
        let sa = sql_scope::match_score(prefix, &a.label);
        let sb = sql_scope::match_score(prefix, &b.label);
        sb.cmp(&sa).then(b.score.cmp(&a.score))
    });
}
```

- [ ] **Step 12: Run all tests**

```bash
cargo test -p tables scope_tree_method_tests 2>&1 | tail -10
```

Expected: all 4 new tests pass. `postgres.rs`/`sqlite.rs` still have compile errors from the trait signature change — that's expected.

- [ ] **Step 13: Commit**

```bash
git add src/completion/engines/core.rs
git commit -m "feat(completion): rewrite CoreCompletionEngine to use ScopeTree; upgrade filter_by_prefix to match_score"
```

---

## Task 3: Update PostgresEngine and SqliteEngine

**Files:**
- Modify: `src/completion/engines/postgres.rs`
- Modify: `src/completion/engines/sqlite.rs`

- [ ] **Step 1: Write the failing test**

Add to `postgres.rs`:

```rust
#[cfg(test)]
mod postgres_scope_tree_tests {
    use super::*;
    use sql_scope::{ScopeTree, Source};
    use sql_scope::scope::tree::{Scope, ScopeType};
    use crate::completion::context::{Context, CursorContext};
    use crate::completion::schema::loader::MockSchemaLoader;

    fn tree_with_alias(alias: &str, table: &str) -> ScopeTree {
        let mut tree = ScopeTree::new();
        let mut scope = Scope::new(0, None, ScopeType::Root, 0..1000);
        scope.sources.insert(alias.to_string(), Source::Alias {
            alias: alias.to_string(),
            target: Box::new(Source::Table { schema: None, name: table.to_string() }),
        });
        tree.add_scope(scope);
        tree
    }

    #[test]
    fn postgres_complete_after_dot_returns_columns() {
        let engine = PostgresEngine::new();
        let schema = MockSchemaLoader::create_test_schema();
        let scope_tree = tree_with_alias("u", "users");
        let context = Context {
            cursor_offset: 20,
            context_type: CursorContext::AfterDot { alias: "u".to_string() },
            prefix: String::new(),
            previous_word: String::new(),
            scope_depth: 0,
        };
        let items = engine.complete(&scope_tree, &context, &schema, None, None);
        assert!(!items.is_empty());
        assert!(items.iter().any(|i| i.label == "id"));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cargo test -p tables postgres_scope_tree_tests 2>&1 | head -20
```

Expected: compile error — `complete` signature mismatch.

- [ ] **Step 3: Update `postgres.rs`**

Remove the `SemanticModel` import, update the `complete()` method signature and all `CoreCompletionEngine` calls:

```rust
// REMOVE:
use crate::completion::analysis::SemanticModel;

// complete() signature becomes:
fn complete(
    &self,
    scope_tree: &sql_scope::ScopeTree,
    context: &Context,
    schema: &SchemaGraph,
    default_schema: Option<&str>,
    capabilities: Option<&DatabaseCapabilities>,
) -> Vec<CompletionItem> {
    let effective_schema = default_schema
        .map(|s| s.to_string())
        .or_else(|| capabilities.and_then(|c| c.default_schema.clone()))
        .unwrap_or_else(|| self.default_schema().to_string());

    match &context.context_type {
        CursorContext::AfterDot { alias } =>
            CoreCompletionEngine::complete_after_dot(alias, scope_tree, context, schema),
        CursorContext::SelectClause | CursorContext::AfterSelectList =>
            CoreCompletionEngine::complete_select_clause(scope_tree, context, schema, SELECT_KEYWORDS, SELECT_FUNCTIONS),
        CursorContext::RootContext => {
            let mut items = CoreCompletionEngine::complete_root_context(context);
            self.add_postgres_root_items(&mut items, context);
            items
        }
        CursorContext::FromClause | CursorContext::JoinTable =>
            CoreCompletionEngine::complete_table_names(schema, scope_tree, context, Some(&effective_schema), FROM_KEYWORDS),
        CursorContext::JoinCondition { left_table, right_table } =>
            CoreCompletionEngine::complete_join_condition(left_table, right_table, scope_tree, schema),
        CursorContext::JoinConditionRhs { .. } => {
            let operators = self.operators();
            CoreCompletionEngine::complete_where_clause(scope_tree, context, schema, WHERE_KEYWORDS, WHERE_FUNCTIONS, &operators)
        }
        CursorContext::WhereClause => {
            let operators = self.operators();
            CoreCompletionEngine::complete_where_clause(scope_tree, context, schema, WHERE_KEYWORDS, WHERE_FUNCTIONS, &operators)
        }
        CursorContext::FunctionArgument { function_name } =>
            CoreCompletionEngine::complete_function_argument(function_name, scope_tree, context, schema),
        CursorContext::Unknown =>
            CoreCompletionEngine::complete_generic(scope_tree, context, GENERIC_KEYWORDS),
    }
}
```

- [ ] **Step 4: Update `sqlite.rs` identically**

Same structural changes as `postgres.rs`. `sqlite.rs` has its own keyword/function constants and an extra `add_sqlite_root_items` block — keep those intact, just swap the signature and `CoreCompletionEngine` argument positions.

- [ ] **Step 5: Run test and build check**

```bash
cargo test -p tables postgres_scope_tree_tests 2>&1 | tail -5
cargo build -p tables 2>&1 | grep "^error" | head -10
```

Expected: test passes. Remaining errors only in `completion_commands.rs` and `engine.rs`.

- [ ] **Step 6: Commit**

```bash
git add src/completion/engines/postgres.rs src/completion/engines/sqlite.rs
git commit -m "feat(completion): update PostgresEngine and SqliteEngine to pass ScopeTree"
```

---

## Task 4: Wire `sql_scope::resolve` in `completion_commands.rs` and delete `engine.rs`

**Files:**
- Modify: `src/commands/completion_commands.rs`
- Delete: `src/completion/engine.rs`
- Rewrite: `src/completion/tests.rs`
- Modify: `src/completion/mod.rs` — remove `pub mod engine`

### Context

`completion_commands.rs` currently (inside `spawn_blocking`):

```rust
let semantic = tree.as_ref()
    .map(|t| build_semantic_model(&text, t))
    .unwrap_or_default();
// ...
let engine = create_engine(dialect);
let items = engine.complete(&semantic, &context, &schema, default_schema.as_deref(), None);
```

`engine.rs` is a ~736-line duplicate of `core.rs` logic. It is only referenced by `src/completion/tests.rs` and its own `#[cfg(test)]`. It is deleted here; `tests.rs` is rewritten to use `PostgresEngine` directly.

- [ ] **Step 1: Write the failing test**

Add to `completion_commands.rs`:

```rust
#[cfg(test)]
mod wire_tests {
    use super::*;
    use crate::completion::document::Dialect;

    #[test]
    fn dialect_conversion_covers_all_variants() {
        assert!(matches!(dialect_to_sql_scope(Dialect::Postgres), sql_scope::Dialect::Postgres));
        assert!(matches!(dialect_to_sql_scope(Dialect::SQLite), sql_scope::Dialect::SQLite));
        assert!(matches!(dialect_to_sql_scope(Dialect::MySQL), sql_scope::Dialect::MySQL));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cargo test -p tables wire_tests 2>&1 | head -10
```

Expected: compile error — `dialect_to_sql_scope` not defined.

- [ ] **Step 3: Update `completion_commands.rs`**

1. Remove: `use crate::completion::analysis::build_semantic_model;`

2. Add private helper after imports:
```rust
fn dialect_to_sql_scope(dialect: Dialect) -> sql_scope::Dialect {
    match dialect {
        Dialect::Postgres => sql_scope::Dialect::Postgres,
        Dialect::SQLite => sql_scope::Dialect::SQLite,
        Dialect::MySQL => sql_scope::Dialect::MySQL,
    }
}
```

3. Inside the `spawn_blocking` closure in `request_completions`, replace:
```rust
// REMOVE:
let semantic = tree.as_ref()
    .map(|t| build_semantic_model(&text, t))
    .unwrap_or_default();
// ADD:
let scope_tree = sql_scope::resolve(&text, dialect_to_sql_scope(dialect), schema.as_ref())
    .unwrap_or_default();
```

4. Change the engine call:
```rust
// REMOVE:
let items = engine.complete(&semantic, &context, &schema, default_schema.as_deref(), None);
// ADD:
let items = engine.complete(&scope_tree, &context, &schema, default_schema.as_deref(), None);
```

Keep `parse_sql()` and `Context::analyze()` calls unchanged — they still use the tree-sitter tree.

- [ ] **Step 4: Delete `engine.rs`**

```bash
git rm src/completion/engine.rs
```

- [ ] **Step 5: Remove `pub mod engine` from `src/completion/mod.rs`**

```rust
// REMOVE from mod.rs:
pub mod engine;
```

- [ ] **Step 6: Rewrite `src/completion/tests.rs`**

The existing tests use `CompletionEngine::complete(&semantic, ...)` (from deleted `engine.rs`) and construct `SemanticModel` objects. Replace the entire file with tests that use `PostgresEngine::new().complete(&scope_tree, ...)`.

Helper to add at the top of the test file:

```rust
use sql_scope::{ScopeTree, Source};
use sql_scope::scope::tree::{Scope, ScopeType};
use crate::completion::engines::{PostgresEngine, CompletionEngineVariant};
use crate::completion::context::{Context, CursorContext};
use crate::completion::schema::loader::MockSchemaLoader;

fn run_complete(scope_tree: &ScopeTree, ctx_type: CursorContext, prefix: &str, default_schema: Option<&str>) -> Vec<String> {
    let engine = PostgresEngine::new();
    let schema = MockSchemaLoader::create_test_schema();
    let context = Context {
        cursor_offset: 50,
        context_type: ctx_type,
        prefix: prefix.to_string(),
        previous_word: String::new(),
        scope_depth: 0,
    };
    engine.complete(scope_tree, &context, &schema, default_schema, None)
        .into_iter().map(|i| i.label).collect()
}

fn tree_with_alias(alias: &str, table: &str) -> ScopeTree {
    let mut tree = ScopeTree::new();
    let mut scope = Scope::new(0, None, ScopeType::Root, 0..1000);
    scope.sources.insert(alias.to_string(), Source::Alias {
        alias: alias.to_string(),
        target: Box::new(Source::Table { schema: None, name: table.to_string() }),
    });
    tree.add_scope(scope);
    tree
}

fn empty_tree() -> ScopeTree { ScopeTree::new() }
```

Keep all the same test scenarios from the old file — just translated to the new API. Any test that was asserting something about `SemanticModel` internals can be deleted; keep only the behavioral assertions (what completions appear).

- [ ] **Step 7: Run all tests**

```bash
cargo test -p tables 2>&1 | tail -20
```

Expected: all tests pass, no references to `SemanticModel` or `analysis` in errors.

- [ ] **Step 8: Commit**

```bash
git add src/commands/completion_commands.rs src/completion/mod.rs src/completion/tests.rs
git commit -m "feat(completion): wire sql_scope::resolve at call site, delete engine.rs, rewrite tests.rs"
```

---

## Task 5: Delete `analysis/` module

**Files:**
- Delete: `src/completion/analysis/scope.rs`
- Delete: `src/completion/analysis/builder.rs`
- Delete: `src/completion/analysis/ambiguity.rs`
- Delete: `src/completion/analysis/mod.rs`
- Modify: `src/completion/mod.rs` — remove `pub mod analysis`

- [ ] **Step 1: Verify zero remaining references**

```bash
grep -r "analysis\|SemanticModel\|build_semantic_model\|SymbolKind" \
  src/completion/ src/commands/ 2>&1 | grep -v "^Binary\|//\|\.git"
```

Expected: no output. If any remain, fix them before proceeding.

- [ ] **Step 2: Remove `pub mod analysis` from `mod.rs`**

```rust
// REMOVE from src/completion/mod.rs:
pub mod analysis;
```

- [ ] **Step 3: Delete the files**

```bash
git rm src/completion/analysis/scope.rs \
       src/completion/analysis/builder.rs \
       src/completion/analysis/ambiguity.rs \
       src/completion/analysis/mod.rs
```

- [ ] **Step 4: Full build and test run**

```bash
cargo build -p tables 2>&1 | grep "^error"
cargo test -p tables 2>&1 | tail -20
cargo test -p sql-scope 2>&1 | tail -10
```

Expected: clean build, all tests pass across both crates.

- [ ] **Step 5: Final commit**

```bash
git add src/completion/mod.rs
git commit -m "feat(completion): delete analysis/ module — SemanticModel fully replaced by ScopeTree (Option C complete)"
```

---

## What This Plan Covers

| Feature | Status |
|---|---|
| AfterDot → schema table columns | ✅ Task 2 |
| AfterDot → CTE columns | ✅ Task 2 (explicit CTE branch) |
| SELECT clause → aliases + columns | ✅ Task 2 |
| FROM clause → tables + schemas + CTEs | ✅ Task 2 |
| JOIN condition inference (FK + heuristics) | ✅ Task 2 |
| WHERE clause → qualified columns + operators | ✅ Task 2 |
| Function argument type filtering | ✅ Task 2 |
| Generic fallback keywords | ✅ Task 2 |
| Root context keywords | ✅ Task 2 (no SemanticModel dependency, unchanged) |
| Fuzzy / acronym prefix matching | ✅ Task 2 (`filter_by_prefix` upgraded to `match_score`) |
| CTE forward-isolation (valid SQL semantics) | ✅ inherent in ScopeTree — correctly tightened |
| Diagnostics | ✅ already wired to sql-scope in prior session |
| Multi-statement SQL isolation | ✅ inherent in `sql_scope::resolve()` |
| PostgreSQL dialect keywords/functions | ✅ Task 3 (postgres.rs constants unchanged) |
| SQLite dialect keywords/functions | ✅ Task 3 (sqlite.rs constants unchanged) |
| Cancellation token in completion_commands | ✅ Task 4 — cancellation checks remain around `resolve()` |
| All existing tests pass | ✅ Task 4 (`tests.rs` rewritten with same scenarios) |
