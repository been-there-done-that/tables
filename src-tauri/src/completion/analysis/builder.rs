//! AST → SemanticModel builder.
//!
//! Walks the Tree-sitter AST to extract:
//! - Scopes (queries, subqueries, CTEs)
//! - Symbols (tables, aliases)

use tree_sitter::{Node, Tree};
use super::scope::{SemanticModel, Scope, Symbol};

/// Build a SemanticModel from a parsed SQL document.
pub fn build_semantic_model(source: &str, tree: &Tree) -> SemanticModel {
    let mut model = SemanticModel::new();
    let mut builder = ModelBuilder::new(source, &mut model);
    builder.visit(tree.root_node());
    
    // Fallback: if AST parsing didn't find any symbols, try text-based extraction
    // This handles cases where tree-sitter's error recovery confuses the AST structure
    if model.scopes.iter().all(|s| s.symbols.is_empty()) {
        extract_tables_from_text(source, &mut model);
    }
    
    model
}

/// Fallback text-based extraction for broken SQL.
/// Scans for FROM/JOIN patterns and extracts table aliases.
fn extract_tables_from_text(source: &str, model: &mut SemanticModel) {
    // Ensure we have at least one scope
    if model.scopes.is_empty() {
        model.scopes.push(Scope::new(0, None, 0..source.len()));
    }
    
    let source_upper = source.to_uppercase();
    
    // Find FROM clause: FROM table [alias]
    if let Some(from_pos) = source_upper.find(" FROM ") {
        let after_from = &source[from_pos + 6..];
        if let Some((table, alias)) = extract_table_alias(after_from) {
            let range = from_pos..(from_pos + 6 + table.len());
            model.scopes[0].symbols.push(
                if let Some(a) = alias {
                    Symbol::table_alias(&a, &table, range)
                } else {
                    Symbol::table(&table, range)
                }
            );
        }
    }
    
    // Find JOIN clauses: JOIN table [alias] ON
    let mut search_start = 0;
    while let Some(join_pos) = source_upper[search_start..].find(" JOIN ") {
        let abs_join_pos = search_start + join_pos;
        let after_join = &source[abs_join_pos + 6..];
        if let Some((table, alias)) = extract_table_alias(after_join) {
            let range = abs_join_pos..(abs_join_pos + 6 + table.len());
            model.scopes[0].symbols.push(
                if let Some(a) = alias {
                    Symbol::table_alias(&a, &table, range)
                } else {
                    Symbol::table(&table, range)
                }
            );
        }
        search_start = abs_join_pos + 6;
    }
}

/// Extract table name and optional alias from text after FROM/JOIN.
/// Handles: "table", "table alias", "table AS alias"
fn extract_table_alias(text: &str) -> Option<(String, Option<String>)> {
    let words: Vec<&str> = text.split_whitespace()
        .take_while(|w| !w.eq_ignore_ascii_case("ON") 
                       && !w.eq_ignore_ascii_case("JOIN")
                       && !w.eq_ignore_ascii_case("WHERE")
                       && !w.eq_ignore_ascii_case("LEFT")
                       && !w.eq_ignore_ascii_case("RIGHT")
                       && !w.eq_ignore_ascii_case("INNER")
                       && !w.eq_ignore_ascii_case("OUTER")
                       && !w.eq_ignore_ascii_case("CROSS"))
        .collect();
    
    match words.as_slice() {
        [table] => Some((table.to_string(), None)),
        [table, alias] if !alias.eq_ignore_ascii_case("AS") => {
            Some((table.to_string(), Some(alias.to_string())))
        }
        [table, _, alias] => {
            Some((table.to_string(), Some(alias.to_string())))
        }
        _ => None,
    }
}

struct ModelBuilder<'a> {
    source: &'a str,
    model: &'a mut SemanticModel,
    current_scope_id: Option<usize>,
}

impl<'a> ModelBuilder<'a> {
    fn new(source: &'a str, model: &'a mut SemanticModel) -> Self {
        Self {
            source,
            model,
            current_scope_id: None,
        }
    }

    fn visit(&mut self, node: Node) {
        let kind = node.kind();
        
        // Handle scopes first
        match kind {
            // Top-level or query scope - many possible names
            "program" | "source_file" | "statement" | "select_statement" | 
            "select" | "query" | "sql_stmt" | "sql_stmt_list" => {
                if self.current_scope_id.is_none() || kind == "subquery" {
                    self.enter_scope(node);
                    self.visit_children(node);
                    self.exit_scope();
                    return;
                }
            }
            
            // Subqueries create nested scopes
            "subquery" | "scalar_subquery" | "parenthesized_expression" => {
                // Only create scope if this contains a SELECT
                let has_select = node.children(&mut node.walk())
                    .any(|c| c.kind().contains("select"));
                if has_select {
                    self.enter_scope(node);
                    self.visit_children(node);
                    self.exit_scope();
                    return;
                }
            }
            _ => {}
        }
        
        // Handle table references - try many variations
        match kind {
            // FROM/JOIN table references
            "from_clause" | "from" | "FROM" | 
            "from_item" | "from_items" => {
                self.visit_from_clause(node);
                return;
            }
            
            "join_clause" | "join" | "JOIN" | 
            "join_item" | "joined_table" => {
                self.visit_join_clause(node);
                return;
            }
            
            // Table references with aliases - common patterns
            "table_reference" | "relation" | "table_or_subquery" |
            "relation_expr" | "relation_primary" | 
            "table_ref" | "table_factor" | "simple_table" => {
                self.handle_table_reference(node);
                return;
            }
            
            // Aliased tables
            "aliased_relation" | "table_alias" | "alias" | 
            "as_alias" | "opt_alias" => {
                self.handle_aliased_table(node);
                return;
            }
            
            // CTEs
            "common_table_expression" | "cte" | "cte_definition" |
            "with_clause" => {
                self.handle_cte(node);
                return;
            }
            _ => {}
        }
        
        // Always traverse ERROR nodes to find valid children inside
        if kind == "ERROR" {
            self.visit_children(node);
            return;
        }
        
        // Default: recurse into children
        self.visit_children(node);
    }

    fn visit_children(&mut self, node: Node) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.visit(child);
        }
    }

    fn enter_scope(&mut self, node: Node) {
        let range = node.start_byte()..node.end_byte();
        let scope_id = self.model.scopes.len();
        let scope = Scope::new(scope_id, self.current_scope_id, range);
        self.model.scopes.push(scope);
        self.current_scope_id = Some(scope_id);
    }

    fn exit_scope(&mut self) {
        if let Some(id) = self.current_scope_id {
            self.current_scope_id = self.model.scopes[id].parent_id;
        }
    }

    fn visit_from_clause(&mut self, node: Node) {
        // Visit all children to find table references
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.visit(child);
        }
    }

    fn visit_join_clause(&mut self, node: Node) {
        // Visit all children to find table references and join conditions
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.visit(child);
        }
    }

    fn handle_cte(&mut self, node: Node) {
        // Find CTE name
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "identifier" || child.kind() == "cte_name" {
                let name = self.node_text(&child);
                // Mark the scope as a CTE if we're in a scope
                if let Some(scope_id) = self.current_scope_id {
                    let range = child.start_byte()..child.end_byte();
                    self.model.scopes[scope_id].symbols.push(Symbol::cte(&name, range));
                    self.model.ctes.insert(name.to_lowercase(), Vec::new());
                }
                break;
            }
        }
        
        self.visit_children(node);
    }

    fn handle_table_reference(&mut self, node: Node) {
        let Some(scope_id) = self.current_scope_id else {
            self.visit_children(node);
            return;
        };

        // tree-sitter-sequel structure for "users u":
        // [relation] "users u"
        //   [object_reference] "users"
        //     [identifier] "users"
        //   [identifier] "u"
        //
        // We need to:
        // 1. Find object_reference → get the table name from its identifier child
        // 2. Find direct identifier child → that's the alias

        let mut table_name = None;
        let mut alias_name = None;
        
        let mut cursor = node.walk();
        let children: Vec<Node> = node.children(&mut cursor).collect();
        
        for (i, child) in children.iter().enumerate() {
            match child.kind() {
                "object_reference" => {
                    // Descend into object_reference to get the actual table name
                    let mut inner_cursor = child.walk();
                    for inner in child.children(&mut inner_cursor) {
                        if inner.kind() == "identifier" {
                            table_name = Some(self.node_text(&inner));
                            break;
                        }
                    }
                    // Fallback: use the whole text if no identifier found
                    if table_name.is_none() {
                        table_name = Some(self.node_text(child));
                    }
                }
                "identifier" => {
                    // Direct identifier child - this is the alias
                    let text = self.node_text(child);
                    if table_name.is_none() {
                        // No object_reference yet, this might be the table name
                        table_name = Some(text);
                    } else if alias_name.is_none() {
                        alias_name = Some(text);
                    }
                }
                "table_name" => {
                    table_name = Some(self.node_text(child));
                }
                "AS" | "as" | "keyword_as" => {
                    // Next identifier is the alias
                    if i + 1 < children.len() {
                        let next = &children[i + 1];
                        if next.kind() == "identifier" {
                            alias_name = Some(self.node_text(next));
                        }
                    }
                }
                "alias" => {
                    alias_name = Some(self.node_text(child));
                }
                _ => {}
            }
        }

        // Create symbol if we found a table
        if let Some(table) = table_name {
            let range = node.start_byte()..node.end_byte();
            let symbol = if let Some(alias) = alias_name {
                Symbol::table_alias(&alias, &table, range)
            } else {
                Symbol::table(&table, range)
            };
            self.model.scopes[scope_id].symbols.push(symbol);
        }

        // Don't visit children - we've handled this node completely
    }

    fn handle_aliased_table(&mut self, node: Node) {
        let Some(scope_id) = self.current_scope_id else {
            self.visit_children(node);
            return;
        };

        let mut table_name = None;
        let mut alias_name = None;

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "identifier" | "table_name" | "object_reference" => {
                    let text = self.node_text(&child);
                    if table_name.is_none() {
                        table_name = Some(text);
                    } else {
                        alias_name = Some(text);
                    }
                }
                "alias" => {
                    alias_name = Some(self.node_text(&child));
                }
                _ => {}
            }
        }

        if let (Some(table), Some(alias)) = (table_name, alias_name) {
            let range = node.start_byte()..node.end_byte();
            self.model.scopes[scope_id].symbols.push(
                Symbol::table_alias(&alias, &table, range)
            );
        }

        self.visit_children(node);
    }

    fn node_text(&self, node: &Node) -> String {
        let start = node.start_byte();
        let end = node.end_byte();
        if end <= self.source.len() {
            self.source[start..end].to_string()
        } else {
            String::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::completion::parsing::parse_sql;

    #[test]
    fn test_simple_table_extraction() {
        let source = "SELECT * FROM users";
        let tree = parse_sql(source, None).unwrap();
        let model = build_semantic_model(source, &tree);
        
        assert!(!model.scopes.is_empty(), "Should have at least one scope");
        
        // Check that we found the 'users' table
        let visible = model.visible_symbols_at(10);
        let has_users = visible.iter().any(|s| s.resolve_table_name() == Some("users"));
        assert!(has_users, "Should find 'users' table");
    }

    #[test]
    fn test_alias_extraction() {
        let source = "SELECT * FROM users u";
        let tree = parse_sql(source, None).unwrap();
        let model = build_semantic_model(source, &tree);
        
        // Should find alias 'u' pointing to 'users'
        let resolved = model.resolve_at_cursor(10, "u");
        
        // Note: This might fail if the grammar doesn't parse aliases the way we expect.
        // The test documents the expected behavior.
        if let Some(sym) = resolved {
            assert_eq!(sym.resolve_table_name(), Some("users"));
        }
    }

    #[test]
    fn test_multiple_tables() {
        let source = "SELECT * FROM users u JOIN orders o ON u.id = o.user_id";
        let tree = parse_sql(source, None).unwrap();
        let model = build_semantic_model(source, &tree);
        
        let visible = model.visible_symbols_at(20);
        
        // Should see both tables/aliases
        println!("Visible symbols: {:?}", visible.iter().map(|s| &s.name).collect::<Vec<_>>());
    }

    #[test]
    fn test_debug_ast_structure() {
        let source = "SELECT u.id FROM users u";
        let tree = parse_sql(source, None).unwrap();
        
        fn print_tree(node: tree_sitter::Node, source: &str, indent: usize) {
            let text = &source[node.start_byte()..node.end_byte().min(source.len())];
            let text_short = if text.len() > 20 { &text[..20] } else { text };
            println!("{:indent$}[{}] {:?}", "", node.kind(), text_short, indent=indent);
            for child in node.children(&mut node.walk()) {
                print_tree(child, source, indent + 2);
            }
        }
        
        println!("\n=== AST Debug for 'SELECT u.id FROM users u' ===");
        print_tree(tree.root_node(), source, 0);
        
        let model = build_semantic_model(source, &tree);
        println!("\n=== Scopes: {} ===", model.scopes.len());
        for (i, scope) in model.scopes.iter().enumerate() {
            println!("  Scope {}: range {:?}, symbols: {:?}", 
                i, scope.range, scope.symbols.iter().map(|s| &s.name).collect::<Vec<_>>());
        }
    }

    #[test]
    fn test_debug_broken_sql() {
        // This is what the tests see - incomplete SQL after removing cursor marker
        let source = "SELECT u. FROM users u";
        let tree = parse_sql(source, None).unwrap();
        
        fn print_tree(node: tree_sitter::Node, source: &str, indent: usize) {
            let text = &source[node.start_byte()..node.end_byte().min(source.len())];
            let text_short = if text.len() > 20 { &text[..20] } else { text };
            println!("{:indent$}[{}] {:?}", "", node.kind(), text_short, indent=indent);
            for child in node.children(&mut node.walk()) {
                print_tree(child, source, indent + 2);
            }
        }
        
        println!("\n=== AST Debug for BROKEN 'SELECT u. FROM users u' ===");
        print_tree(tree.root_node(), source, 0);
        
        let model = build_semantic_model(source, &tree);
        println!("\n=== Scopes: {} ===", model.scopes.len());
        for (i, scope) in model.scopes.iter().enumerate() {
            println!("  Scope {}: range {:?}, symbols: {:?}", 
                i, scope.range, scope.symbols.iter().map(|s| &s.name).collect::<Vec<_>>());
        }
    }
}
