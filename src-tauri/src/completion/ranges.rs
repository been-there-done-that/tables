use tree_sitter::Tree;

/// Range of a SQL statement (1-based line numbers for Monaco)
#[derive(serde::Serialize, Clone)]
pub struct StatementRange {
    pub start_line: u32, // 1-based for Monaco
    pub end_line: u32,
}

/// Extended range including byte offsets (for extracting query text)
#[derive(serde::Serialize, Clone)]
pub struct StatementRangeWithBytes {
    pub start_line: u32,   // 1-based for Monaco
    pub end_line: u32,
    pub start_byte: usize, // For extracting text
    pub end_byte: usize,
}

/// Find the SQL statement range containing the cursor.
/// 
/// Handles edge case: cursor immediately after semicolon (e.g., `SELECT 1;|`)
/// is considered part of that statement for highlighting purposes.
pub fn find_current_statement_range(tree: &Tree, cursor_offset: usize) -> Option<StatementRange> {
    let root = tree.root_node();
    let mut cursor = root.walk();
    let mut last_statement: Option<StatementRange> = None;

    // Iterate over top-level statements only
    for child in root.children(&mut cursor) {
        let start_byte = child.start_byte();
        let end_byte = child.end_byte();

        // Check if cursor is inside this statement OR immediately after it
        // The +1 allows cursor right after semicolon to still match
        if cursor_offset >= start_byte && cursor_offset <= end_byte {
            let start_point = child.start_position();
            let end_point = child.end_position();

            return Some(StatementRange {
                start_line: (start_point.row + 1) as u32,
                end_line: (end_point.row + 1) as u32,
            });
        }

        // Track last statement in case cursor is right after it
        if cursor_offset == end_byte + 1 {
            let start_point = child.start_position();
            let end_point = child.end_position();
            last_statement = Some(StatementRange {
                start_line: (start_point.row + 1) as u32,
                end_line: (end_point.row + 1) as u32,
            });
        }
    }
    
    // Return last statement if cursor was immediately after it
    last_statement
}

/// Find ALL SQL statement ranges in the document.
/// Used for CodeLens/glyph margin to show run buttons on each query.
/// Filters out comment-only nodes and invalid fragments.
pub fn find_all_statement_ranges(tree: &Tree, text_source: &str) -> Vec<StatementRangeWithBytes> {
    let root = tree.root_node();
    let mut cursor = root.walk();
    let mut ranges = Vec::new();

    for child in root.children(&mut cursor) {
        // Skip comment nodes
        let node_kind = child.kind();
        if node_kind == "comment" || node_kind == "line_comment" || node_kind == "block_comment" {
            continue;
        }
        
        // Skip empty nodes
        if child.start_byte() == child.end_byte() {
            continue;
        }

        // HEURISTIC: Check if it's a valid statement start
        // Tree-sitter sometimes breaks on complex queries and emits fragments like "AND ..." as new statements.
        // We only want run buttons on actual start commands.
        let node_text = child.utf8_text(text_source.as_bytes()).unwrap_or("").trim().to_uppercase();
        
        // List of valid statement starters
        let valid_starts = [
            "SELECT", "WITH", "INSERT", "UPDATE", "DELETE", 
            "CREATE", "DROP", "ALTER", "PRAGMA", "TRUNCATE",
            "EXPLAIN", "DESCRIBE", "SHOW", "USE", "BEGIN", 
            "COMMIT", "ROLLBACK", "TABLE", "VALUES", "SET",
            "GRANT", "REVOKE", "REINDEX", "VACUUM", "ANALYZE",
            "RENAME", "COMMENT", "CALL", "DO", "DECLARE"
        ];

        let starts_valid = valid_starts.iter().any(|start| node_text.starts_with(start));
        
        // If it doesn't start with a valid keyword, skip it (likely a fragment like "AND x=y")
        if !starts_valid {
            continue;
        }

        let start_point = child.start_position();
        let end_point = child.end_position();

        ranges.push(StatementRangeWithBytes {
            start_line: (start_point.row + 1) as u32,
            end_line: (end_point.row + 1) as u32,
            start_byte: child.start_byte(),
            end_byte: child.end_byte(),
        });
    }

    ranges
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::completion::parsing::parse_sql;

    #[test]
    fn test_single_statement_range() {
        let sql = "SELECT * FROM users|";
        let cursor = sql.find('|').unwrap();
        let source = sql.replace('|', "");
        let tree = parse_sql(&source, None).unwrap();
        
        let range = find_current_statement_range(&tree, cursor).unwrap();
        assert_eq!(range.start_line, 1);
        assert_eq!(range.end_line, 1);
    }

    #[test]
    fn test_multiple_statements_range() {
        let sql = "SELECT 1;\nSELECT 2|;\nSELECT 3;";
        let cursor = sql.find('|').unwrap();
        let source = sql.replace('|', "");
        let tree = parse_sql(&source, None).unwrap();
        
        let range = find_current_statement_range(&tree, cursor).unwrap();
        assert_eq!(range.start_line, 2);
        assert_eq!(range.end_line, 2);
    }

    #[test]
    fn test_semicolon_in_string() {
        let sql = "SELECT 'hello; world' FROM t|;";
        let cursor = sql.find('|').unwrap();
        let source = sql.replace('|', "");
        let tree = parse_sql(&source, None).unwrap();
        
        let range = find_current_statement_range(&tree, cursor).unwrap();
        assert_eq!(range.start_line, 1);
        assert_eq!(range.end_line, 1);
    }

    #[test]
    fn test_cursor_at_semicolon() {
        let sql = "SELECT 1|; SELECT 2";
        let cursor = sql.find('|').unwrap();
        let source = sql.replace('|', "");
        let tree = parse_sql(&source, None).unwrap();
        
        // Should include the semicolon
        let range = find_current_statement_range(&tree, cursor).unwrap();
        assert_eq!(range.start_line, 1);
        assert_eq!(range.end_line, 1);
    }

    #[test]
    fn test_cursor_between_statements() {
        let sql = "SELECT 1; \n | \n SELECT 2";
        let cursor = sql.find('|').unwrap();
        let source = sql.replace('|', "");
        let tree = parse_sql(&source, None).unwrap();
        
        let range = find_current_statement_range(&tree, cursor);
        // Ideally should be None if strictly between, or nearest.
        // Tree-sitter root children might not cover whitespace perfectly.
        assert!(range.is_none());
    }
}
