use std::collections::HashMap;
use std::ops::Range;
use indexmap::IndexMap;
use super::symbol::{ColumnRef, ScopeId, Source, VisibleSymbols};

#[derive(Debug, Clone, PartialEq)]
pub enum ScopeType {
    Root,
    Cte { name: String },
    Subquery,
    DerivedTable { alias: String },
    Union,
}

/// CTE info stored in a scope's cte_sources registry.
#[derive(Debug, Clone)]
pub struct CteInfo {
    pub scope_id: ScopeId,
    /// Resolved projected column names (wildcard-expanded). Empty = not yet resolved.
    pub columns: Vec<String>,
    pub is_recursive: bool,
}

/// A single scope in the scope tree.
#[derive(Debug)]
pub struct Scope {
    pub id: ScopeId,
    pub parent: Option<ScopeId>,
    pub scope_type: ScopeType,
    pub byte_range: Range<usize>,
    /// Local sources: alias/name → Source (tables and subqueries in FROM)
    pub sources: HashMap<String, Source>,
    /// CTEs visible in this scope (inherited from parent + locally defined, in declaration order)
    pub cte_sources: IndexMap<String, CteInfo>,
    /// Columns this scope projects outward (used by parent for wildcard expansion)
    pub columns: Vec<ColumnRef>,
}

impl Scope {
    pub fn new(id: ScopeId, parent: Option<ScopeId>, scope_type: ScopeType, byte_range: Range<usize>) -> Self {
        Self {
            id,
            parent,
            scope_type,
            byte_range,
            sources: HashMap::new(),
            cte_sources: IndexMap::new(),
            columns: Vec::new(),
        }
    }

    /// Whether this scope's byte range contains the given cursor position.
    pub fn contains(&self, cursor_byte: usize) -> bool {
        self.byte_range.contains(&cursor_byte)
    }
}

/// Diagnostics produced during scope resolution.
#[derive(Debug, Clone)]
pub struct ScopeDiagnostic {
    pub message: String,
    pub severity: DiagSeverity,
    pub byte_range: Range<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagSeverity {
    Error,
    Warning,
    Info,
}

/// The fully resolved scope tree for a single SQL statement.
pub struct ScopeTree {
    scopes: Vec<Scope>,
    diagnostics: Vec<ScopeDiagnostic>,
}

impl ScopeTree {
    pub fn new() -> Self {
        Self { scopes: Vec::new(), diagnostics: Vec::new() }
    }

    /// Add a scope and return its assigned ScopeId.
    pub fn add_scope(&mut self, mut scope: Scope) -> ScopeId {
        let id = self.scopes.len();
        scope.id = id;
        self.scopes.push(scope);
        id
    }

    /// Returns the scope with the given `id`.
    ///
    /// # Panics
    /// Panics if `id` was not returned by [`ScopeTree::add_scope`].
    pub fn scope(&self, id: ScopeId) -> &Scope {
        &self.scopes[id]
    }

    /// Returns a mutable reference to the scope with the given `id`.
    ///
    /// # Panics
    /// Panics if `id` was not returned by [`ScopeTree::add_scope`].
    pub fn scope_mut(&mut self, id: ScopeId) -> &mut Scope {
        &mut self.scopes[id]
    }

    /// All scopes (for diagnostic passes).
    pub fn all_scopes(&self) -> &[Scope] {
        &self.scopes
    }

    /// Returns the innermost scope containing `cursor_byte`.
    ///
    /// Requires that scopes were added in pre-order (parent before child) —
    /// a parent's `ScopeId` is always less than any of its children's ids.
    /// The resolver must uphold this invariant.
    pub fn scope_at(&self, cursor_byte: usize) -> Option<&Scope> {
        self.scopes.iter().rev().find(|s| s.contains(cursor_byte))
    }

    /// Walk up the scope chain from the innermost scope at `cursor_byte`,
    /// collecting all sources and CTE names visible at that position.
    ///
    /// All IR byte ranges are `0..sql.len()` (exclusive upper bound), so a cursor
    /// sitting exactly at the end of the text (`cursor_byte == sql.len()`) would
    /// fall outside every scope.  We step back one byte in that case so the root
    /// scope still covers the position.
    pub fn visible_at(&self, cursor_byte: usize) -> VisibleSymbols {
        // Try the given position first; if not inside any scope, try one byte back.
        let effective = if self.scope_at(cursor_byte).is_none() && cursor_byte > 0 {
            cursor_byte - 1
        } else {
            cursor_byte
        };
        let mut vis = VisibleSymbols::default();
        let Some(start_scope) = self.scope_at(effective) else {
            return vis;
        };

        let mut scope_id = start_scope.id;
        loop {
            let scope = self.scope(scope_id);
            // Add local sources (no shadowing — first definition wins)
            for (alias, source) in &scope.sources {
                if !vis.has_source(alias) {
                    vis.sources.push((alias.clone(), source.clone()));
                }
            }
            // Add CTE sources
            for (name, _info) in &scope.cte_sources {
                let src = Source::Cte { name: name.clone() };
                if !vis.has_source(name) {
                    vis.sources.push((name.clone(), src));
                }
            }
            match scope.parent {
                Some(pid) => scope_id = pid,
                None => break,
            }
        }
        vis
    }

    /// Add a diagnostic.
    pub fn add_diagnostic(&mut self, diag: ScopeDiagnostic) {
        self.diagnostics.push(diag);
    }

    /// Return a clone of all diagnostics collected during scope resolution.
    pub fn diagnostics(&self) -> Vec<ScopeDiagnostic> {
        self.diagnostics.clone()
    }
}

impl Default for ScopeTree {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::symbol::{ColumnRef, Source};

    fn make_scope(id: ScopeId, parent: Option<ScopeId>, scope_type: ScopeType, range: Range<usize>) -> Scope {
        Scope::new(id, parent, scope_type, range)
    }

    // -------------------------------------------------------------------------
    // ScopeTree::new() is empty
    // -------------------------------------------------------------------------

    #[test]
    fn scope_tree_new_is_empty() {
        let tree = ScopeTree::new();
        assert!(tree.all_scopes().is_empty());
        assert!(tree.diagnostics().is_empty());
    }

    // -------------------------------------------------------------------------
    // add_scope assigns sequential IDs starting at 0
    // -------------------------------------------------------------------------

    #[test]
    fn add_scope_assigns_sequential_ids() {
        let mut tree = ScopeTree::new();
        let s0 = make_scope(0, None, ScopeType::Root, 0..100);
        let s1 = make_scope(99, None, ScopeType::Subquery, 10..50); // id 99 will be overwritten

        let id0 = tree.add_scope(s0);
        let id1 = tree.add_scope(s1);

        assert_eq!(id0, 0);
        assert_eq!(id1, 1);
        assert_eq!(tree.scope(0).id, 0);
        assert_eq!(tree.scope(1).id, 1);
    }

    // -------------------------------------------------------------------------
    // scope_at on empty tree returns None
    // -------------------------------------------------------------------------

    #[test]
    fn scope_at_empty_tree_returns_none() {
        let tree = ScopeTree::new();
        assert!(tree.scope_at(0).is_none());
        assert!(tree.scope_at(50).is_none());
    }

    // -------------------------------------------------------------------------
    // scope_at finds innermost scope (deeper scope wins)
    // -------------------------------------------------------------------------

    #[test]
    fn scope_at_finds_innermost_scope() {
        let mut tree = ScopeTree::new();
        // Root: 0..100
        let root = make_scope(0, None, ScopeType::Root, 0..100);
        let root_id = tree.add_scope(root);
        // Inner subquery: 20..60
        let inner = make_scope(1, Some(root_id), ScopeType::Subquery, 20..60);
        let inner_id = tree.add_scope(inner);

        // cursor at 30 is in both root (0..100) and inner (20..60)
        // should return inner (higher id = deeper)
        let found = tree.scope_at(30).unwrap();
        assert_eq!(found.id, inner_id);
    }

    // -------------------------------------------------------------------------
    // scope_at falls back to root when cursor is in root range but not child
    // -------------------------------------------------------------------------

    #[test]
    fn scope_at_falls_back_to_root() {
        let mut tree = ScopeTree::new();
        let root = make_scope(0, None, ScopeType::Root, 0..100);
        let root_id = tree.add_scope(root);
        let inner = make_scope(1, Some(root_id), ScopeType::Subquery, 20..60);
        tree.add_scope(inner);

        // cursor at 5: only root contains it
        let found = tree.scope_at(5).unwrap();
        assert_eq!(found.id, root_id);
    }

    // -------------------------------------------------------------------------
    // scope_at returns None when cursor is outside all scopes
    // -------------------------------------------------------------------------

    #[test]
    fn scope_at_returns_none_outside_all_scopes() {
        let mut tree = ScopeTree::new();
        let root = make_scope(0, None, ScopeType::Root, 0..100);
        tree.add_scope(root);

        // cursor at 200 is outside root (0..100)
        assert!(tree.scope_at(200).is_none());
    }

    // -------------------------------------------------------------------------
    // visible_at on empty tree returns empty VisibleSymbols
    // -------------------------------------------------------------------------

    #[test]
    fn visible_at_empty_tree_returns_empty() {
        let tree = ScopeTree::new();
        let vis = tree.visible_at(50);
        assert!(vis.sources.is_empty());
        assert!(vis.columns.is_empty());
    }

    // -------------------------------------------------------------------------
    // visible_at includes sources from innermost scope
    // -------------------------------------------------------------------------

    #[test]
    fn visible_at_includes_sources_from_innermost() {
        let mut tree = ScopeTree::new();
        let mut root = make_scope(0, None, ScopeType::Root, 0..100);
        root.sources.insert("users".to_string(), Source::Table { schema: None, name: "users".to_string() });
        tree.add_scope(root);

        let vis = tree.visible_at(50);
        assert!(vis.has_source("users"));
    }

    // -------------------------------------------------------------------------
    // visible_at walks up to parent scope
    // -------------------------------------------------------------------------

    #[test]
    fn visible_at_walks_up_to_parent() {
        let mut tree = ScopeTree::new();
        let mut root = make_scope(0, None, ScopeType::Root, 0..100);
        root.sources.insert("users".to_string(), Source::Table { schema: None, name: "users".to_string() });
        let root_id = tree.add_scope(root);

        let mut inner = make_scope(1, Some(root_id), ScopeType::Subquery, 20..60);
        inner.sources.insert("orders".to_string(), Source::Table { schema: None, name: "orders".to_string() });
        tree.add_scope(inner);

        // cursor at 30 — in inner scope; should see both inner and parent sources
        let vis = tree.visible_at(30);
        assert!(vis.has_source("orders"));   // inner
        assert!(vis.has_source("users"));    // parent
    }

    // -------------------------------------------------------------------------
    // visible_at inner sources shadow outer sources with same name
    // -------------------------------------------------------------------------

    #[test]
    fn visible_at_inner_shadows_outer() {
        let mut tree = ScopeTree::new();
        let mut root = make_scope(0, None, ScopeType::Root, 0..100);
        // Root has "t" → orders
        root.sources.insert("t".to_string(), Source::Table { schema: None, name: "orders".to_string() });
        let root_id = tree.add_scope(root);

        let mut inner = make_scope(1, Some(root_id), ScopeType::Subquery, 20..60);
        // Inner also has "t" → users (shadows root's "t")
        inner.sources.insert("t".to_string(), Source::Table { schema: None, name: "users".to_string() });
        tree.add_scope(inner);

        let vis = tree.visible_at(30);

        // "t" should appear exactly once
        let count = vis.sources.iter().filter(|(a, _)| a == "t").count();
        assert_eq!(count, 1);

        // The visible "t" should be the inner one (users), not the outer one (orders)
        let src = vis.get_source("t").unwrap();
        assert_eq!(src.canonical_name(), "users");
    }

    // -------------------------------------------------------------------------
    // visible_at includes CTE sources from cte_sources
    // -------------------------------------------------------------------------

    #[test]
    fn visible_at_includes_cte_sources() {
        let mut tree = ScopeTree::new();
        let mut root = make_scope(0, None, ScopeType::Root, 0..100);
        root.cte_sources.insert("my_cte".to_string(), CteInfo {
            scope_id: 0,
            columns: vec![],
            is_recursive: false,
        });
        tree.add_scope(root);

        let vis = tree.visible_at(50);
        assert!(vis.has_source("my_cte"));
    }

    // -------------------------------------------------------------------------
    // visible_at CTE sources from parent scope are visible in child
    // -------------------------------------------------------------------------

    #[test]
    fn visible_at_parent_cte_visible_in_child() {
        let mut tree = ScopeTree::new();
        let mut root = make_scope(0, None, ScopeType::Root, 0..100);
        root.cte_sources.insert("parent_cte".to_string(), CteInfo {
            scope_id: 0,
            columns: vec!["id".to_string()],
            is_recursive: false,
        });
        let root_id = tree.add_scope(root);

        let inner = make_scope(1, Some(root_id), ScopeType::Subquery, 20..60);
        tree.add_scope(inner);

        // cursor at 30 in inner scope; parent's CTE should be visible
        let vis = tree.visible_at(30);
        assert!(vis.has_source("parent_cte"));
    }

    // -------------------------------------------------------------------------
    // Scope::contains
    // -------------------------------------------------------------------------

    #[test]
    fn scope_contains_true_inside_range() {
        let scope = make_scope(0, None, ScopeType::Root, 10..50);
        assert!(scope.contains(10));
        assert!(scope.contains(30));
        assert!(scope.contains(49));
    }

    #[test]
    fn scope_contains_false_outside_range() {
        let scope = make_scope(0, None, ScopeType::Root, 10..50);
        assert!(!scope.contains(9));
        assert!(!scope.contains(50));
        assert!(!scope.contains(100));
    }

    // -------------------------------------------------------------------------
    // ScopeDiagnostic — construction and field access
    // -------------------------------------------------------------------------

    #[test]
    fn scope_diagnostic_construction() {
        let diag = ScopeDiagnostic {
            message: "unknown table".to_string(),
            severity: DiagSeverity::Error,
            byte_range: 10..20,
        };
        assert_eq!(diag.message, "unknown table");
        assert_eq!(diag.severity, DiagSeverity::Error);
        assert_eq!(diag.byte_range, 10..20);
    }

    #[test]
    fn scope_diagnostic_clone() {
        let diag = ScopeDiagnostic {
            message: "warning msg".to_string(),
            severity: DiagSeverity::Warning,
            byte_range: 5..15,
        };
        let diag2 = diag.clone();
        assert_eq!(diag2.message, "warning msg");
        assert_eq!(diag2.severity, DiagSeverity::Warning);
    }

    // -------------------------------------------------------------------------
    // DiagSeverity — PartialEq
    // -------------------------------------------------------------------------

    #[test]
    fn diag_severity_partial_eq() {
        assert_eq!(DiagSeverity::Error, DiagSeverity::Error);
        assert_eq!(DiagSeverity::Warning, DiagSeverity::Warning);
        assert_eq!(DiagSeverity::Info, DiagSeverity::Info);
        assert_ne!(DiagSeverity::Error, DiagSeverity::Warning);
        assert_ne!(DiagSeverity::Warning, DiagSeverity::Info);
        assert_ne!(DiagSeverity::Error, DiagSeverity::Info);
    }

    // -------------------------------------------------------------------------
    // ScopeTree::add_diagnostic stores diagnostics
    // -------------------------------------------------------------------------

    #[test]
    fn add_diagnostic_stores_diagnostics() {
        let mut tree = ScopeTree::new();
        tree.add_diagnostic(ScopeDiagnostic {
            message: "error one".to_string(),
            severity: DiagSeverity::Error,
            byte_range: 0..5,
        });
        tree.add_diagnostic(ScopeDiagnostic {
            message: "info note".to_string(),
            severity: DiagSeverity::Info,
            byte_range: 10..20,
        });
        let diags = tree.diagnostics();
        assert_eq!(diags.len(), 2);
        assert_eq!(diags[0].message, "error one");
        assert_eq!(diags[1].severity, DiagSeverity::Info);
    }

    // -------------------------------------------------------------------------
    // all_scopes returns all added scopes
    // -------------------------------------------------------------------------

    #[test]
    fn all_scopes_returns_all() {
        let mut tree = ScopeTree::new();
        tree.add_scope(make_scope(0, None, ScopeType::Root, 0..100));
        tree.add_scope(make_scope(1, Some(0), ScopeType::Subquery, 10..50));
        tree.add_scope(make_scope(2, Some(0), ScopeType::Subquery, 60..90));
        assert_eq!(tree.all_scopes().len(), 3);
    }

    // -------------------------------------------------------------------------
    // Multiple scopes at different depth levels — scope_at finds correct one
    // -------------------------------------------------------------------------

    #[test]
    fn scope_at_multiple_depths_finds_correct() {
        let mut tree = ScopeTree::new();
        // Root: 0..200
        let root_id = tree.add_scope(make_scope(0, None, ScopeType::Root, 0..200));
        // Mid: 50..150
        let mid_id = tree.add_scope(make_scope(1, Some(root_id), ScopeType::Subquery, 50..150));
        // Deep: 80..120
        let deep_id = tree.add_scope(make_scope(2, Some(mid_id), ScopeType::DerivedTable { alias: "dt".to_string() }, 80..120));

        // cursor at 10: only root
        assert_eq!(tree.scope_at(10).unwrap().id, root_id);
        // cursor at 60: root and mid; innermost = mid
        assert_eq!(tree.scope_at(60).unwrap().id, mid_id);
        // cursor at 90: root, mid, deep; innermost = deep
        assert_eq!(tree.scope_at(90).unwrap().id, deep_id);
    }

    // -------------------------------------------------------------------------
    // Scope chain walking — grandparent → parent → child, all visible at child
    // -------------------------------------------------------------------------

    #[test]
    fn visible_at_grandparent_chain() {
        let mut tree = ScopeTree::new();

        let mut grandparent = make_scope(0, None, ScopeType::Root, 0..200);
        grandparent.sources.insert("gp_table".to_string(), Source::Table { schema: None, name: "gp_table".to_string() });
        let gp_id = tree.add_scope(grandparent);

        let mut parent = make_scope(1, Some(gp_id), ScopeType::Subquery, 10..150);
        parent.sources.insert("p_table".to_string(), Source::Table { schema: None, name: "p_table".to_string() });
        let p_id = tree.add_scope(parent);

        let mut child = make_scope(2, Some(p_id), ScopeType::Subquery, 20..100);
        child.sources.insert("c_table".to_string(), Source::Table { schema: None, name: "c_table".to_string() });
        tree.add_scope(child);

        let vis = tree.visible_at(50);
        assert!(vis.has_source("c_table"));   // child
        assert!(vis.has_source("p_table"));   // parent
        assert!(vis.has_source("gp_table")); // grandparent
    }

    // -------------------------------------------------------------------------
    // ScopeType variants are all constructable
    // -------------------------------------------------------------------------

    #[test]
    fn scope_type_variants_constructable() {
        let _root = ScopeType::Root;
        let _cte = ScopeType::Cte { name: "my_cte".to_string() };
        let _subquery = ScopeType::Subquery;
        let _derived = ScopeType::DerivedTable { alias: "dt".to_string() };
        let _union = ScopeType::Union;

        // Clone them
        let types = vec![
            ScopeType::Root,
            ScopeType::Cte { name: "x".to_string() },
            ScopeType::Subquery,
            ScopeType::DerivedTable { alias: "y".to_string() },
            ScopeType::Union,
        ];
        assert_eq!(types.len(), 5);
    }

    // -------------------------------------------------------------------------
    // CteInfo — construction, clone, is_recursive flag
    // -------------------------------------------------------------------------

    #[test]
    fn cte_info_construction() {
        let info = CteInfo {
            scope_id: 3,
            columns: vec!["id".to_string(), "name".to_string()],
            is_recursive: true,
        };
        assert_eq!(info.scope_id, 3);
        assert_eq!(info.columns.len(), 2);
        assert!(info.is_recursive);
    }

    #[test]
    fn cte_info_clone() {
        let info = CteInfo {
            scope_id: 7,
            columns: vec!["col1".to_string()],
            is_recursive: false,
        };
        let info2 = info.clone();
        assert_eq!(info2.scope_id, 7);
        assert_eq!(info2.columns, vec!["col1"]);
        assert!(!info2.is_recursive);
    }

    // -------------------------------------------------------------------------
    // scope_mut — mutable access to a scope
    // -------------------------------------------------------------------------

    #[test]
    fn scope_mut_modifies_scope() {
        let mut tree = ScopeTree::new();
        tree.add_scope(make_scope(0, None, ScopeType::Root, 0..100));

        let scope = tree.scope_mut(0);
        scope.columns.push(ColumnRef { name: "result".into(), source_table: None, source_alias: None });

        assert_eq!(tree.scope(0).columns, vec![ColumnRef { name: "result".into(), source_table: None, source_alias: None }]);
    }
}
