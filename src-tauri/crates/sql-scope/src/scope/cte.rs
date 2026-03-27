use crate::ir::{CteIr, SelectBodyIr, SelectItemIr, TableRefIr};
use crate::schema::SchemaSnapshot;
use super::symbol::Source;
use super::symbol::ScopeId;
use super::tree::ScopeTree;

const MAX_WILDCARD_DEPTH: u8 = 5;

/// Resolve the projected columns of a CTE.
/// If explicit_columns is non-empty, use those directly.
/// Otherwise, project from the body SELECT list with wildcard expansion.
pub fn resolve_cte_columns(
    cte: &CteIr,
    scope_id: ScopeId,
    tree: &ScopeTree,
    schema: &dyn SchemaSnapshot,
) -> Vec<String> {
    if !cte.explicit_columns.is_empty() {
        return cte.explicit_columns.clone();
    }
    project_from_body(&cte.body, scope_id, tree, schema, 0)
}

fn project_from_body(
    body: &SelectBodyIr,
    scope_id: ScopeId,
    tree: &ScopeTree,
    schema: &dyn SchemaSnapshot,
    depth: u8,
) -> Vec<String> {
    if depth > MAX_WILDCARD_DEPTH {
        return vec![];
    }

    let mut cols = Vec::new();
    for item in &body.select_list {
        match item {
            SelectItemIr::Wildcard => {
                // Expand * from all sources in this scope
                for tref in &body.from {
                    let expanded = expand_table_ref_columns(tref, scope_id, tree, schema, depth);
                    cols.extend(expanded);
                }
            }
            SelectItemIr::TableWildcard(tname) => {
                let expanded = expand_source_columns(tname, scope_id, tree, schema, depth);
                cols.extend(expanded);
            }
            SelectItemIr::Expr { alias: Some(alias), .. } => {
                cols.push(alias.clone());
            }
            SelectItemIr::Expr { alias: None, .. } => {
                // unnamed expr — skip
            }
        }
    }
    cols
}

fn expand_table_ref_columns(
    tref: &TableRefIr,
    scope_id: ScopeId,
    tree: &ScopeTree,
    schema: &dyn SchemaSnapshot,
    depth: u8,
) -> Vec<String> {
    match tref {
        TableRefIr::Table { name, alias, .. } => {
            let key = alias.as_deref().unwrap_or(name);
            expand_source_columns(key, scope_id, tree, schema, depth)
        }
        TableRefIr::Subquery { alias, .. } => {
            expand_source_columns(alias, scope_id, tree, schema, depth)
        }
        TableRefIr::Join { left, right } => {
            let mut cols = expand_table_ref_columns(left, scope_id, tree, schema, depth);
            cols.extend(expand_table_ref_columns(right, scope_id, tree, schema, depth));
            cols
        }
    }
}

fn expand_source_columns(
    alias: &str,
    scope_id: ScopeId,
    tree: &ScopeTree,
    schema: &dyn SchemaSnapshot,
    depth: u8,
) -> Vec<String> {
    if depth > MAX_WILDCARD_DEPTH {
        return vec![];
    }
    let scope = tree.scope(scope_id);

    // Check local sources
    if let Some(source) = scope.sources.get(alias) {
        return match source {
            Source::Table { schema: tschema, name } => {
                schema.table_columns(tschema.as_deref(), name).unwrap_or_default()
            }
            Source::Alias { target, .. } => {
                match target.as_ref() {
                    Source::Table { schema: tschema, name } => {
                        schema.table_columns(tschema.as_deref(), name).unwrap_or_default()
                    }
                    Source::DerivedTable { scope_id: child_id } => {
                        tree.scope(*child_id).columns.iter().map(|c| c.name.clone()).collect()
                    }
                    Source::Cte { name } => {
                        let cte_name = name.clone();
                        if let Some(info) = scope.cte_sources.get(&cte_name) {
                            info.columns.clone()
                        } else {
                            vec![]
                        }
                    }
                    _ => vec![],
                }
            }
            Source::DerivedTable { scope_id: child_id } => {
                tree.scope(*child_id).columns.iter().map(|c| c.name.clone()).collect()
            }
            Source::Cte { name } => {
                let cte_name = name.clone();
                if let Some(cte_info) = scope.cte_sources.get(&cte_name) {
                    if cte_info.columns.is_empty() {
                        let cte_scope_id = cte_info.scope_id;
                        expand_from_cte_scope(cte_scope_id, tree, schema, depth + 1)
                    } else {
                        cte_info.columns.clone()
                    }
                } else {
                    vec![]
                }
            }
        };
    }

    // Check CTE sources directly
    if let Some(cte_info) = scope.cte_sources.get(alias) {
        if cte_info.columns.is_empty() {
            let cte_scope_id = cte_info.scope_id;
            return expand_from_cte_scope(cte_scope_id, tree, schema, depth + 1);
        }
        return cte_info.columns.clone();
    }

    // Walk up to parent scope
    if let Some(pid) = scope.parent {
        return expand_source_columns(alias, pid, tree, schema, depth);
    }

    vec![]
}

fn expand_from_cte_scope(
    scope_id: ScopeId,
    tree: &ScopeTree,
    schema: &dyn SchemaSnapshot,
    depth: u8,
) -> Vec<String> {
    if depth > MAX_WILDCARD_DEPTH {
        return vec![];
    }
    let _ = schema; // may be used for future expansion
    tree.scope(scope_id).columns.iter().map(|c| c.name.clone()).collect()
}
