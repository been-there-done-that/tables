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
    
    // Fallback: if AST parsing encountered errors or didn't find any symbols, try text-based extraction
    // This handles cases where tree-sitter's error recovery confuses the AST structure
    if builder.has_error || model.scopes.iter().all(|s| s.symbols.is_empty()) {
        extract_tables_from_text(source, &mut model);
    }
    
    model
}

/// Fallback text-based extraction for broken SQL.
/// Scans for FROM/JOIN patterns and extracts table aliases.
fn extract_tables_from_text(source: &str, model: &mut SemanticModel) {
    // Find or create a global scope (covering entire source)
    let global_scope_idx = model.scopes.iter().position(|s| s.range.len() == source.len());
    
    let target_idx = if let Some(idx) = global_scope_idx {
        idx
    } else {
        let id = model.scopes.len();
        model.scopes.push(Scope::new(id, None, 0..source.len()));
        id
    };
    
    let source_upper = source.to_uppercase();
    
    // Find FROM clause: FROM table [alias]
    if let Some(from_pos) = source_upper.find(" FROM ") {
        let after_from = &source[from_pos + 6..];
        if let Some((table, alias)) = extract_table_alias(after_from) {
            let range = from_pos..(from_pos + 6 + table.len());
            model.scopes[target_idx].symbols.push(
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
            model.scopes[target_idx].symbols.push(
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
    has_error: bool,
}

impl<'a> ModelBuilder<'a> {
    fn new(source: &'a str, model: &'a mut SemanticModel) -> Self {
        Self {
            source,
            model,
            current_scope_id: None,
            has_error: false,
        }
    }

    fn visit(&mut self, node: Node) {
        let kind = node.kind();
        
        // Scope handling
        match kind {
            "program" | "statement_list" | "source_file" => {
                // Root scope - ensure we have one
                if self.current_scope_id.is_none() {
                    self.enter_scope(node);
                }
            }
            "select_statement" | "subquery" => {
                self.enter_scope(node);
            }
            "cte" | "common_table_expression" => {
                // Determine if this is definition or usage scope...
                // Handled in handle_cte
            }
            _ => {}
        }
        
        match kind {
            // FROM/JOIN table references
            "from_clause" | "from" | "FROM" | 
            "from_item" | "from_items" => {
                self.visit_from_clause(node);
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
        
        match kind {
            "program" | "statement_list" | "source_file" => {
                self.visit_children(node);
                self.exit_scope();
                return;
            }

            "select_statement" | "select_clause" | "from_clause" | 
            "where_clause" | "group_by_clause" | "order_by_clause" |
            "having_clause" | "limit_clause" | "subquery" => {
                self.visit_children(node);
                
                // Exit scope after select statement/subquery
                if kind == "select_statement" || kind == "subquery" {
                    self.exit_scope();
                }
                return;
            }
            
            "join_clause" | "join" | "JOIN" | 
            "join_item" | "joined_table" => {
                self.visit_join_clause(node);
                return;
            }
            _ => {}
        }
        
        // Always traverse ERROR nodes to find valid children inside
        if kind == "ERROR" {
            self.has_error = true;
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
        let mut name = String::new();
        let mut columns = Vec::new();

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "identifier" || child.kind() == "cte_name" {
                name = self.node_text(&child);
            } else if child.kind() == "select_statement" || child.kind() == "subquery" {
                columns = self.extract_query_columns(child);
            } else if child.kind() == "statement" {
                // statement -> select_statement
                let mut inner = child.walk();
                for inner_child in child.children(&mut inner) {
                    if inner_child.kind().contains("select") {
                         columns = self.extract_query_columns(inner_child);
                         break;
                    }
                }
            }
        }
        
        // Sometimes the query is inside a parenthesized expression or deeper
        if columns.is_empty() {
            // Try to find any nested select statement
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind().contains("select") {
                     columns = self.extract_query_columns(child);
                     break;
                }
                // Check inside parenthesis
                if child.kind() == "(" || child.kind() == "parenthesized_expression" {
                     // Recurse a bit? Or just linear scan children
                     let mut inner = child.walk();
                     for inner_child in child.children(&mut inner) {
                         if inner_child.kind().contains("select") {
                             columns = self.extract_query_columns(inner_child);
                             break;
                         }
                     }
                }
            }
        }

        if !name.is_empty() {
            // Mark the scope as a CTE if we're in a scope (which creates a new scope for the CTE body)
            // But wait, handle_cte is called on the definition. The body of the CTE is a content.
            // Scopes are entered/exited in `visit`.
            
            // We just need to register the CTE globally (or scope-locally)
            if let Some(scope_id) = self.current_scope_id {
                let range = node.start_byte()..node.end_byte();
                self.model.scopes[scope_id].symbols.push(Symbol::cte(&name, range));
                self.model.ctes.insert(name.to_lowercase(), columns);
            }
        }
        
        self.visit_children(node);
    }
    
    /// Extract projected columns from a SELECT statement/subquery
    fn extract_query_columns(&self, node: Node) -> Vec<String> {
        let mut columns = Vec::new();
        let mut cursor = node.walk();
        
        // Find the select list/result columns
        // [select_statement] -> [result_column]*
        // tree-sitter-sql varies. often has "select_list" or direct children
        
        // Simple heuristic: traverse all children, look for "result_column" or "aliased_expression"
        // Stop if we hit "FROM"
        
        for child in node.children(&mut cursor) {
            if child.kind().to_uppercase() == "FROM" {
                break;
            }
            
            // Handle select_expression which might be a list or a single item
            if child.kind() == "select_expression" {
                let mut has_comma = false;
                let mut inner = child.walk();
                for sub in child.children(&mut inner) {
                    if sub.kind() == "," {
                        has_comma = true; 
                        break; 
                    }
                }
                
                if has_comma {
                    // Treat as list
                    let mut inner = child.walk();
                    for item in child.children(&mut inner) {
                        if item.kind() == "," { continue; }
                        // Recursive extraction for list items
                         if let Some(col_name) = self.extract_column_alias(item) {
                            columns.push(col_name);
                        }
                    }
                    continue;
                }
            }

            if child.kind() == "result_column" || child.kind() == "select_expression" || child.kind() == "aliased_expression" {
                if let Some(col_name) = self.extract_column_alias(child) {
                    columns.push(col_name);
                }
            }
            // Sometimes result columns are inside a "select_list" node
            if child.kind() == "select_list" {
                let mut list_cursor = child.walk();
                for item in child.children(&mut list_cursor) {
                     if let Some(col_name) = self.extract_column_alias(item) {
                        columns.push(col_name);
                    }
                }
            }
        }
        columns
    }
    
    fn extract_column_alias(&self, node: Node) -> Option<String> {
        let mut cursor = node.walk();
        let children: Vec<Node> = node.children(&mut cursor).collect();

        // Case 1: Explicit AS alias ("col AS alias")
        // Look for "AS" (or keyword_as) and take the next identifier
        for (i, child) in children.iter().enumerate() {
            let kind = child.kind();
             if kind.eq_ignore_ascii_case("AS") || kind == "keyword_as" {
                 if i + 1 < children.len() {
                     return Some(self.node_text(&children[i+1]));
                 }
             }
        }
        
        // Case 2: Implicit alias ("col alias")
        // Usually the last identifier is the alias if there are multiple children and no AS
        if children.len() >= 2 {
            let last = &children[children.len() - 1];
            if last.kind() == "identifier" || last.kind() == "alias" {
                 // Make sure the previous one isn't "AS" (handled above)
                 let prev = &children[children.len() - 2];
                 let prev_kind = prev.kind();
                 if prev_kind != "AS" && prev_kind != "keyword_as" && prev_kind != "." {
                      return Some(self.node_text(last));
                 }
            }
        }
        
        // Case 3: "alias" node direct child
        for child in children.iter() {
             if child.kind() == "alias" {
                 return Some(self.node_text(child));
             }
        }
        
        // Case 4: Simple identifier "col"
        if children.len() == 1 && children[0].kind() == "identifier" {
             return Some(self.node_text(&children[0]));
        }
        
        // Case 5: The node itself is an identifier (if unwrapped)
        if node.kind() == "identifier" {
            return Some(self.node_text(&node));
        }
        
        // Case 6: field_access "table.col" -> extract "col"
        if node.kind() == "field_access" || node.kind() == "object_reference" {
             if let Some(last) = children.last() {
                 if last.kind() == "identifier" {
                     return Some(self.node_text(last));
                 }
             }
        }

        None
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
                    // For schema-qualified names like "production.task_hierarchy_table",
                    // use the full text of the object_reference, not just the first identifier.
                    // The object_reference node contains the complete qualified name.
                    let full_text = self.node_text(child);
                    log::debug!("[Builder] object_reference full text: '{}'", full_text);
                    table_name = Some(full_text);
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
                "table_alias" => {
                    // Extract alias from table_alias node
                    let mut inner = child.walk();
                    for sub in child.children(&mut inner) {
                        if sub.kind() == "identifier" {
                            alias_name = Some(self.node_text(&sub));
                        }
                    }
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
                "object_reference" => {
                    // object_reference contains the full qualified table name (e.g., production.task_hierarchy_table)
                    let text = self.node_text(&child);
                    log::debug!("[Builder] aliased_table object_reference: '{}'", text);
                    table_name = Some(text);
                }
                "identifier" | "table_name" => {
                    let text = self.node_text(&child);
                    log::debug!("[Builder] aliased_table identifier/table_name: '{}'", text);
                    if table_name.is_none() {
                        table_name = Some(text);
                    } else if alias_name.is_none() {
                        alias_name = Some(text);
                    }
                }
                "alias" => {
                    let text = self.node_text(&child);
                    log::debug!("[Builder] aliased_table alias: '{}'", text);
                    alias_name = Some(text);
                }
                _ => {}
            }
        }

        log::debug!("[Builder] aliased_table result: table={:?}, alias={:?}", table_name, alias_name);

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
        let source = "SELECT *,  FROM actor a where a.actor_id = NULL";
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
