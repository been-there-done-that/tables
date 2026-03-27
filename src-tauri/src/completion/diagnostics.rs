use tree_sitter::{Node, Tree};
use crate::completion::schema::graph::SchemaGraph;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Diagnostic {
    pub message: String,
    pub start: usize, // byte offset
    pub end: usize,
    pub severity: u8, // 1=Error, 2=Warning
}

pub struct DiagnosticEngine;

impl DiagnosticEngine {
    pub fn check(tree: &Tree, source: &str, schema: &SchemaGraph) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let mut cte_scopes = std::collections::HashSet::new();
        
        // Traverse the tree once for efficiency
        Self::traverse(tree.root_node(), source, schema, &mut diagnostics, &mut cte_scopes);

        // Pass 4: Dangerous statement detection (text-based — grammar lacks DML node types)
        Self::check_dangerous_stmts(tree.root_node(), source, &mut diagnostics);

        diagnostics
    }

    fn traverse(
        node: Node,
        source: &str,
        schema: &SchemaGraph,
        diagnostics: &mut Vec<Diagnostic>,
        cte_scopes: &mut std::collections::HashSet<String>
    ) {
        let kind = node.kind();

        // Pass 0: Track CTEs in scope
        // If we see a `cte` node, extract the alias identifier and add it to scopes
        let mut added_cte = None;
        if kind == "cte" {
            // A CTE normally has an `identifier` before the `AS` keyword.
            // Example: `apples AS (select ...)`
            // We just need the first identifier child.
            if let Some(alias_node) = Self::find_first_child_of_kind(node, "identifier") {
                if let Ok(alias) = alias_node.utf8_text(source.as_bytes()) {
                    let alias_lower = alias.to_lowercase();
                    cte_scopes.insert(alias_lower.clone());
                    added_cte = Some(alias_lower);
                }
            }
        }

        // Pass 1: Syntax Errors (from Tree-sitter)
        if node.is_error() || node.is_missing() {
            let desc = if node.is_missing() { "missing" } else { "unexpected" };
            let mut text = node.utf8_text(source.as_bytes()).unwrap_or("").trim();
            
            // False positive fix: Many SQL grammars flag a trailing semicolon as an ERROR
            // even if it's perfectly valid between statements or at the end.
            if kind == "ERROR" && text == ";" {
                // Skip reporting this as an error
            } else {
                // False positive fix: tree-sitter-sql incorrectly treats table constraints as syntax errors.
                // We will silence errors that appear to be table constraints.
                if kind == "ERROR" {
                    let cleaned_text = text.trim_start_matches(|c: char| c == ',' || c.is_whitespace());
                    if cleaned_text.to_uppercase().starts_with("CONSTRAINT ") {
                        // Skip reporting this as an error
                        return;
                    }
                }
                
                let message = if kind == "ERROR" {
                    if text.is_empty() {
                        "Syntax error: unexpected token".to_string()
                    } else {
                        // Truncate long error texts
                        if text.len() > 30 {
                            text = &text[..27];
                            format!("Syntax error: unexpected '{}...'", text)
                        } else {
                            format!("Syntax error: unexpected '{}'", text)
                        }
                    }
                } else {
                    format!("Syntax error: {} '{}'", desc, kind)
                };

                diagnostics.push(Diagnostic {
                    message,
                    start: node.start_byte(),
                    end: node.end_byte(),
                    severity: 1, // Error
                });
            }
        }

        // Pass 2: Semantic Errors (Basic unknown table check)
        // In our grammar (tree-sitter-sequel), tables appear in 'relation' or 'table_reference'
        // containing an 'object_reference' or 'relation_name'
        if kind == "relation_name" || kind == "object_reference" || kind == "table_name" {
            if let Ok(table_name) = node.utf8_text(source.as_bytes()) {
                // If it contains a dot, it's likely schema.table, split it
                let name = table_name.split('.').last().unwrap_or(table_name).to_lowercase();
                
                // Only check if we have some tables in schema, otherwise we might squiggle everything
                // during initialization or if disconnected.
                // ALSO, skip if this name matches a locally defined CTE!
                if !schema.tables.is_empty() && !schema.has_table(&name) && !cte_scopes.contains(&name) {
                    // Primitive check to avoid squiggling subquery aliases or local variables
                    // Real implementation would use the SemanticModel/Scope tree
                    // For now, we only flag it if it's definitely in a FROM/JOIN context
                    if Self::is_in_table_context(node) {
                        diagnostics.push(Diagnostic {
                            message: format!("Unknown table '{}'", table_name),
                            start: node.start_byte(),
                            end: node.end_byte(),
                            severity: 2, // Warning
                        });
                    }
                }
            }
        }

        // Pass 3: Check for duplicate columns in CREATE TABLE
        if kind == "create_table" || kind == "schema_create" {
            let mut column_names = std::collections::HashSet::new();
            
            // Find the sub-node containing the column definitions
            // The structure is typically create_table -> column_definitions -> column_definition
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "column_definitions" {
                    let mut col_cursor = child.walk();
                    for col_def in child.children(&mut col_cursor) {
                        if col_def.kind() == "column_definition" {
                            // The first identifier in column_definition is usually the name
                            if let Some(name_node) = Self::find_first_child_of_kind(col_def, "identifier") {
                                if let Ok(name) = name_node.utf8_text(source.as_bytes()) {
                                    let lower_name = name.to_lowercase();
                                    if !column_names.insert(lower_name) {
                                        // It was already in the set! Duplicate!
                                        diagnostics.push(Diagnostic {
                                            message: format!("Duplicate column '{}'", name),
                                            start: name_node.start_byte(),
                                            end: name_node.end_byte(),
                                            severity: 1, // Error
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Recursively check children
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            Self::traverse(child, source, schema, diagnostics, cte_scopes);
        }

        // Leave scope: remove CTE alias if we added one
        if let Some(alias) = added_cte {
            cte_scopes.remove(&alias);
        }
    }

    /// Pass 4: Text-based dangerous statement detection.
    ///
    /// tree-sitter-sql 0.0.2 only defines named nodes for SELECT and CREATE TABLE —
    /// DELETE/UPDATE/DROP/TRUNCATE all parse as unnamed/error nodes, so AST-based
    /// detection is unreliable. Instead, we examine the text of each top-level statement.
    fn check_dangerous_stmts(root: Node, source: &str, diagnostics: &mut Vec<Diagnostic>) {
        let mut cursor = root.walk();
        for stmt in root.children(&mut cursor) {
            let Ok(text) = stmt.utf8_text(source.as_bytes()) else { continue };
            let trimmed = text.trim();
            if trimmed.is_empty() || trimmed == ";" {
                continue;
            }

            let upper = trimmed.to_uppercase();
            let first_word = upper.split_ascii_whitespace().next().unwrap_or("");

            match first_word {
                "DROP" => {
                    diagnostics.push(Diagnostic {
                        message: "Destructive: DROP cannot be undone".to_string(),
                        start: stmt.start_byte(),
                        end: stmt.end_byte(),
                        severity: 2,
                    });
                }
                "TRUNCATE" => {
                    diagnostics.push(Diagnostic {
                        message: "Destructive: TRUNCATE will delete all rows".to_string(),
                        start: stmt.start_byte(),
                        end: stmt.end_byte(),
                        severity: 2,
                    });
                }
                "DELETE" if !upper.contains("WHERE") => {
                    diagnostics.push(Diagnostic {
                        message: "DELETE without WHERE will erase every row".to_string(),
                        start: stmt.start_byte(),
                        end: stmt.end_byte(),
                        severity: 2,
                    });
                }
                "UPDATE" if !upper.contains("WHERE") => {
                    diagnostics.push(Diagnostic {
                        message: "UPDATE without WHERE will modify every row".to_string(),
                        start: stmt.start_byte(),
                        end: stmt.end_byte(),
                        severity: 2,
                    });
                }
                _ => {}
            }
        }
    }

    /// Helper script to find the first child of a certain kind
    fn find_first_child_of_kind<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == kind {
                return Some(child);
            }
        }
        None
    }

    /// Check if a node is likely a table reference by looking at its parents
    fn is_in_table_context(node: Node) -> bool {
        let mut curr = node;
        while let Some(parent) = curr.parent() {
            let p_kind = parent.kind();
            if p_kind == "from_clause" || p_kind == "join_clause" || p_kind == "relation" || p_kind == "table_reference" {
                return true;
            }
            // Stop if we hit a query/statement to avoid false positives in random identifiers
            if p_kind == "select_statement" || p_kind == "statement" {
                break;
            }
            curr = parent;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::completion::parsing::parse_sql;
    use crate::completion::schema::graph::{SchemaGraph, TableInfo};

    #[test]
    fn test_syntax_error() {
        let sql = "SELECT * FROM ;"; // Missing table
        let tree = parse_sql(sql, None).unwrap();
        let schema = SchemaGraph::new();
        let diagnostics = DiagnosticEngine::check(&tree, sql, &schema);
        
        assert!(!diagnostics.is_empty());
    }

    #[test]
    fn test_unknown_table() {
        let sql = "SELECT * FROM non_existent_table;";
        let tree = parse_sql(sql, None).unwrap();
        
        let mut schema = SchemaGraph::new(); 
        // Add a dummy table so we have a schema
        schema.add_table(TableInfo::new("users", "public", vec![]));
        
        let diagnostics = DiagnosticEngine::check(&tree, sql, &schema);
        
        assert!(diagnostics.iter().any(|d| d.message.contains("Unknown table")));
    }

    #[test]
    fn test_valid_sql_with_semicolon() {
        let sql = "SELECT * FROM albums;";
        let tree = parse_sql(sql, None).unwrap();
        let schema = SchemaGraph::new();
        let diagnostics = DiagnosticEngine::check(&tree, sql, &schema);
        
        // Assert no syntax errors for this valid SQL
        for d in &diagnostics {
            if d.severity == 1 {
                panic!("Unexpected syntax error in valid SQL: {}", d.message);
            }
        }
    }

    #[test]
    fn test_multiple_statements_with_semicolons() {
        let sql = "SELECT 1; SELECT 2;";
        let tree = parse_sql(sql, None).unwrap();
        let schema = SchemaGraph::new();
        let diagnostics = DiagnosticEngine::check(&tree, sql, &schema);
        
        for d in &diagnostics {
            if d.severity == 1 {
                panic!("Unexpected syntax error in multi-statement SQL: {}", d.message);
            }
        }
    }
    #[test]
    fn test_duplicate_column_constraint() {
        let sql = r#"
CREATE TABLE IF NOT EXISTS production.tableau_user_license (
    user_id UUID NOT NULL
        REFERENCES production.users(id) ON DELETE CASCADE,
    organization_id UUID NOT NULL
        REFERENCES production.organization(id) ON DELETE CASCADE,
    tableau_username VARCHAR(255) NULL,
    is_disabled BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    user_id UUID NOT NULL
        REFERENCES production.users(id) ON DELETE CASCADE,
    created_by UUID
        REFERENCES production.users(id) ON DELETE SET NULL,
    modified_by UUID
        REFERENCES production.users(id) ON DELETE SET NULL,
    CONSTRAINT tableau_user_license_user_org_unique UNIQUE (user_id, organization_id)
);
        "#;
        let tree = parse_sql(sql, None).unwrap();
        println!("AST: {}", tree.root_node().to_sexp());
        
        let schema = SchemaGraph::new();
        let diagnostics = DiagnosticEngine::check(&tree, sql, &schema);
        for d in &diagnostics {
            println!("DIAGNOSTIC: {:?}", d);
        }
    }

    #[test]
    fn test_delete_without_where() {
        let sql = "DELETE FROM orders;";
        let tree = parse_sql(sql, None).unwrap();
        let schema = SchemaGraph::new();
        let diagnostics = DiagnosticEngine::check(&tree, sql, &schema);
        assert!(diagnostics.iter().any(|d| d.message.contains("DELETE without WHERE")));
    }

    #[test]
    fn test_delete_with_where() {
        let sql = "DELETE FROM orders WHERE id = 1;";
        let tree = parse_sql(sql, None).unwrap();
        let schema = SchemaGraph::new();
        let diagnostics = DiagnosticEngine::check(&tree, sql, &schema);
        assert!(!diagnostics.iter().any(|d| d.message.contains("DELETE without WHERE")));
    }

    #[test]
    fn test_update_without_where() {
        let sql = "UPDATE users SET active = false;";
        let tree = parse_sql(sql, None).unwrap();
        let schema = SchemaGraph::new();
        let diagnostics = DiagnosticEngine::check(&tree, sql, &schema);
        assert!(diagnostics.iter().any(|d| d.message.contains("UPDATE without WHERE")));
    }

    #[test]
    fn test_drop_table() {
        let sql = "DROP TABLE IF EXISTS users;";
        let tree = parse_sql(sql, None).unwrap();
        let schema = SchemaGraph::new();
        let diagnostics = DiagnosticEngine::check(&tree, sql, &schema);
        assert!(diagnostics.iter().any(|d| d.message.contains("DROP cannot be undone")));
    }

    #[test]
    fn test_truncate() {
        let sql = "TRUNCATE TABLE logs;";
        let tree = parse_sql(sql, None).unwrap();
        let schema = SchemaGraph::new();
        let diagnostics = DiagnosticEngine::check(&tree, sql, &schema);
        assert!(diagnostics.iter().any(|d| d.message.contains("TRUNCATE will delete all rows")));
    }

    #[test]
    fn test_cte_table_unknown() {
        let sql = r#"
        with apples as (
            select * from production.tasks t where t.id is not null
        ) select * FROM apples;
        "#;
        let tree = parse_sql(sql, None).unwrap();
        println!("AST: {}", tree.root_node().to_sexp());
        
        let schema = SchemaGraph::new();
        let diagnostics = DiagnosticEngine::check(&tree, sql, &schema);
        for d in &diagnostics {
            println!("DIAGNOSTIC: {:?}", d);
        }
    }
}
