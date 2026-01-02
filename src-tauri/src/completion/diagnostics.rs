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
        
        // Traverse the tree once for efficiency
        Self::traverse(tree.root_node(), source, schema, &mut diagnostics);
        
        diagnostics
    }

    fn traverse(node: Node, source: &str, schema: &SchemaGraph, diagnostics: &mut Vec<Diagnostic>) {
        let kind = node.kind();

        // Pass 1: Syntax Errors (from Tree-sitter)
        if node.is_error() || node.is_missing() {
            let desc = if node.is_missing() { "missing" } else { "unexpected" };
            let mut text = node.utf8_text(source.as_bytes()).unwrap_or("").trim();
            
            // False positive fix: Many SQL grammars flag a trailing semicolon as an ERROR
            // even if it's perfectly valid between statements or at the end.
            if kind == "ERROR" && text == ";" {
                // Skip reporting this as an error
            } else {
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
                if !schema.tables.is_empty() && !schema.has_table(&name) {
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

        // Recursively check children
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            Self::traverse(child, source, schema, diagnostics);
        }
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
}
