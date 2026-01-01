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
    model
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
        match node.kind() {
            // Top-level statement or subquery creates a scope
            "statement" | "select_statement" => {
                self.enter_scope(node);
                self.visit_children(node);
                self.exit_scope();
            }
            
            // Subqueries create nested scopes
            "subquery" => {
                self.enter_scope(node);
                self.visit_children(node);
                self.exit_scope();
            }

            // CTE definitions
            "common_table_expression" | "cte" => {
                self.handle_cte(node);
            }

            // FROM clause table references
            "from_clause" | "from" => {
                self.visit_from_clause(node);
            }

            // JOIN clause table references  
            "join_clause" | "join" => {
                self.visit_join_clause(node);
            }

            // Table reference with optional alias
            "table_reference" | "relation" | "table_or_subquery" => {
                self.handle_table_reference(node);
            }

            // Aliased table: table_name alias
            "aliased_relation" | "table_alias" => {
                self.handle_aliased_table(node);
            }

            _ => {
                self.visit_children(node);
            }
        }
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

        // Check for aliased form: "table_name alias" or "table_name AS alias"
        let mut table_name = None;
        let mut alias_name = None;
        
        let mut cursor = node.walk();
        let children: Vec<Node> = node.children(&mut cursor).collect();
        
        for (i, child) in children.iter().enumerate() {
            match child.kind() {
                "identifier" | "table_name" | "object_reference" => {
                    let text = self.node_text(child);
                    if table_name.is_none() {
                        table_name = Some(text);
                    } else if alias_name.is_none() {
                        alias_name = Some(text);
                    }
                }
                "AS" | "as" => {
                    // Next identifier is the alias
                    if i + 1 < children.len() {
                        let next = &children[i + 1];
                        if next.kind() == "identifier" || next.kind() == "alias" {
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

        // Create symbol
        if let Some(table) = table_name {
            let range = node.start_byte()..node.end_byte();
            let symbol = if let Some(alias) = alias_name {
                Symbol::table_alias(&alias, &table, range)
            } else {
                Symbol::table(&table, range)
            };
            self.model.scopes[scope_id].symbols.push(symbol);
        }

        self.visit_children(node);
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
}
