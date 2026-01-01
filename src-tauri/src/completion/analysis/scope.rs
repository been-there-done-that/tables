//! Scope and symbol structures.
//!
//! The SemanticModel is the "compiled" representation of the SQL query,
//! containing all scopes and their symbols (aliases, tables, CTEs).

use std::collections::HashMap;
use std::ops::Range;

/// The compiled semantic model of a SQL query.
#[derive(Debug, Default)]
pub struct SemanticModel {
    /// All scopes in the query, indexed by scope ID.
    pub scopes: Vec<Scope>,
    /// CTE names visible in the query (name → columns).
    pub ctes: HashMap<String, Vec<String>>,
}

/// A scope represents a query or subquery context.
#[derive(Debug)]
pub struct Scope {
    /// Unique scope ID (index in SemanticModel.scopes)
    pub id: usize,
    /// Parent scope ID (None for top-level query)
    pub parent_id: Option<usize>,
    /// Byte range in source that this scope covers
    pub range: Range<usize>,
    /// Symbols defined in this scope (tables, aliases)
    pub symbols: Vec<Symbol>,
    /// Is this a CTE scope?
    pub is_cte: bool,
}

impl Scope {
    pub fn new(id: usize, parent_id: Option<usize>, range: Range<usize>) -> Self {
        Self {
            id,
            parent_id,
            range,
            symbols: Vec::new(),
            is_cte: false,
        }
    }

    /// Find a symbol by name in this scope.
    pub fn find_symbol(&self, name: &str) -> Option<&Symbol> {
        let name_lower = name.to_lowercase();
        self.symbols.iter().find(|s| s.name.to_lowercase() == name_lower)
    }
}

/// A symbol in a scope (table reference or alias).
#[derive(Debug, Clone)]
pub struct Symbol {
    /// The symbol name (e.g., "u" for alias, "users" for table)
    pub name: String,
    /// What kind of symbol this is
    pub kind: SymbolKind,
    /// Byte range where the symbol is defined
    pub def_range: Range<usize>,
}

/// The type of symbol.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SymbolKind {
    /// A direct table reference: `FROM users`
    Table { table_name: String },
    /// An aliased table: `FROM users u` or `FROM users AS u`
    TableAlias { alias: String, table_name: String },
    /// A column reference
    Column { column_name: String, table_name: Option<String> },
    /// A CTE reference: `WITH active_users AS (...)`
    CTE { cte_name: String },
}

impl Symbol {
    /// Create a table symbol.
    pub fn table(name: &str, range: Range<usize>) -> Self {
        Self {
            name: name.to_string(),
            kind: SymbolKind::Table { table_name: name.to_string() },
            def_range: range,
        }
    }

    /// Create a table alias symbol.
    pub fn table_alias(alias: &str, table_name: &str, range: Range<usize>) -> Self {
        Self {
            name: alias.to_string(),
            kind: SymbolKind::TableAlias {
                alias: alias.to_string(),
                table_name: table_name.to_string(),
            },
            def_range: range,
        }
    }

    /// Create a CTE symbol.
    pub fn cte(name: &str, range: Range<usize>) -> Self {
        Self {
            name: name.to_string(),
            kind: SymbolKind::CTE { cte_name: name.to_string() },
            def_range: range,
        }
    }

    /// Resolve the underlying table name.
    pub fn resolve_table_name(&self) -> Option<&str> {
        match &self.kind {
            SymbolKind::Table { table_name } => Some(table_name),
            SymbolKind::TableAlias { table_name, .. } => Some(table_name),
            SymbolKind::CTE { cte_name } => Some(cte_name),
            SymbolKind::Column { .. } => None,
        }
    }
}

impl SemanticModel {
    pub fn new() -> Self {
        Self::default()
    }

    /// Find the deepest scope containing the given byte offset.
    pub fn find_scope_at(&self, offset: usize) -> Option<&Scope> {
        // Find deepest (most nested) scope containing offset
        self.scopes
            .iter()
            .filter(|s| s.range.contains(&offset))
            .max_by_key(|s| s.range.start) // Deeper scopes start later
    }

    /// Resolve a table/alias name at a cursor position.
    /// Walks up the scope chain until a match is found.
    pub fn resolve_at_cursor(&self, cursor: usize, name: &str) -> Option<&Symbol> {
        let mut scope = self.find_scope_at(cursor)?;
        
        loop {
            if let Some(sym) = scope.find_symbol(name) {
                return Some(sym);
            }
            
            match scope.parent_id {
                Some(pid) => scope = &self.scopes[pid],
                None => break,
            }
        }
        
        // Check CTEs as fallback
        None
    }

    /// Get all visible symbols at a cursor position.
    pub fn visible_symbols_at(&self, cursor: usize) -> Vec<&Symbol> {
        let mut symbols = Vec::new();
        let mut seen_names = std::collections::HashSet::new();
        
        let Some(mut scope) = self.find_scope_at(cursor) else {
            return symbols;
        };
        
        loop {
            for sym in &scope.symbols {
                let name_lower = sym.name.to_lowercase();
                if !seen_names.contains(&name_lower) {
                    symbols.push(sym);
                    seen_names.insert(name_lower);
                }
            }
            
            match scope.parent_id {
                Some(pid) => scope = &self.scopes[pid],
                None => break,
            }
        }
        
        symbols
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_symbol_resolution() {
        let mut model = SemanticModel::new();
        
        // Create a scope with a table alias
        let mut scope = Scope::new(0, None, 0..50);
        scope.symbols.push(Symbol::table_alias("u", "users", 20..21));
        model.scopes.push(scope);
        
        // Resolve the alias
        let resolved = model.resolve_at_cursor(25, "u");
        assert!(resolved.is_some());
        
        let sym = resolved.unwrap();
        assert_eq!(sym.resolve_table_name(), Some("users"));
    }

    #[test]
    fn test_scope_nesting() {
        let mut model = SemanticModel::new();
        
        // Outer scope: 0..100
        let mut outer = Scope::new(0, None, 0..100);
        outer.symbols.push(Symbol::table_alias("u", "users", 10..11));
        model.scopes.push(outer);
        
        // Inner scope: 30..70 (subquery)
        let mut inner = Scope::new(1, Some(0), 30..70);
        inner.symbols.push(Symbol::table_alias("o", "orders", 35..36));
        model.scopes.push(inner);
        
        // Inside inner scope, can see both 'u' and 'o'
        let symbols = model.visible_symbols_at(50);
        assert_eq!(symbols.len(), 2);
        
        // 'o' should be resolved from inner scope
        let o_sym = model.resolve_at_cursor(50, "o");
        assert!(o_sym.is_some());
        assert_eq!(o_sym.unwrap().resolve_table_name(), Some("orders"));
        
        // 'u' should be resolved from outer scope
        let u_sym = model.resolve_at_cursor(50, "u");
        assert!(u_sym.is_some());
        assert_eq!(u_sym.unwrap().resolve_table_name(), Some("users"));
    }

    #[test]
    fn test_inner_shadows_outer() {
        let mut model = SemanticModel::new();
        
        // Outer scope with 'u' → users
        let mut outer = Scope::new(0, None, 0..100);
        outer.symbols.push(Symbol::table_alias("u", "users", 10..11));
        model.scopes.push(outer);
        
        // Inner scope also has 'u' → orders (shadows outer)
        let mut inner = Scope::new(1, Some(0), 30..70);
        inner.symbols.push(Symbol::table_alias("u", "orders", 35..36));
        model.scopes.push(inner);
        
        // Inside inner scope, 'u' should resolve to 'orders'
        let u_sym = model.resolve_at_cursor(50, "u");
        assert!(u_sym.is_some());
        assert_eq!(u_sym.unwrap().resolve_table_name(), Some("orders"));
    }
}
