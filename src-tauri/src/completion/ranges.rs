use tree_sitter::Tree;

/// Range of a SQL statement (1-based line numbers for Monaco)
#[derive(serde::Serialize, Clone, Debug)]
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
/// Handles edge cases:
/// - Cursor inside statement → returns that statement
/// - Cursor on semicolon node → returns preceding statement  
/// - Cursor immediately after semicolon → returns preceding statement
/// - Cursor on new line below statement → returns None
/// - Cursor on new line below statement → returns None
pub fn find_current_statement_range(tree: &Tree, text_source: &str, cursor_offset: usize) -> Option<StatementRange> {
    let root = tree.root_node();
    let mut cursor = root.walk();
    
    // Track the last actual statement (not semicolon nodes)
    let mut last_statement: Option<StatementRange> = None;
    let mut last_statement_end_byte: usize = 0;

    // Iterate over top-level children
    for child in root.children(&mut cursor) {
        let start_byte = child.start_byte();
        let end_byte = child.end_byte();
        let kind = child.kind();

        // If cursor is BEFORE this node begins
        if cursor_offset < start_byte {
            // Return last statement if cursor is within 2 chars of its end
            if let Some(ref last) = last_statement {
                // If the gap contains a newline, assume meaningful separation
                // This prevents `SELECT 1;\n|` from counting as part of the previous statement
                if last_statement_end_byte <= cursor_offset && cursor_offset <= text_source.len() {
                    let gap = &text_source[last_statement_end_byte..cursor_offset];
                    if gap.contains('\n') {
                        return None;
                    }
                }

                if cursor_offset <= last_statement_end_byte + 2 {
                    return Some(last.clone());
                }
            }
            return None;
        }

        // Check if cursor is inside this node
        if cursor_offset >= start_byte && cursor_offset <= end_byte {
            // If this is a semicolon node, return the preceding statement
            if kind == ";" {
                return last_statement;
            }
            
            // Skip comment nodes
            if kind == "comment" || kind == "line_comment" || kind == "block_comment" {
                continue;
            }

            // This is a real statement node
            let start_point = child.start_position();
            let end_point = child.end_position();

            return Some(StatementRange {
                start_line: (start_point.row + 1) as u32,
                end_line: (end_point.row + 1) as u32,
            });
        }

        // Track last statement (excluding semicolons and comments)
        if kind != ";" && kind != "comment" && kind != "line_comment" && kind != "block_comment" {
            let start_point = child.start_position();
            let end_point = child.end_position();
            last_statement = Some(StatementRange {
                start_line: (start_point.row + 1) as u32,
                end_line: (end_point.row + 1) as u32,
            });
            last_statement_end_byte = end_byte;
        }
    }
    
    // If cursor is after all nodes, check if close to last statement
    if let Some(ref last) = last_statement {
        if cursor_offset <= last_statement_end_byte + 2 {
            return Some(last.clone());
        }
    }
    
    None
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
        
        let range = find_current_statement_range(&tree, &source, cursor).unwrap();
        assert_eq!(range.start_line, 1);
        assert_eq!(range.end_line, 1);
    }

    #[test]
    fn test_multiple_statements_range() {
        let sql = "SELECT 1;\nSELECT 2|;\nSELECT 3;";
        let cursor = sql.find('|').unwrap();
        let source = sql.replace('|', "");
        let tree = parse_sql(&source, None).unwrap();
        
        let range = find_current_statement_range(&tree, &source, cursor).unwrap();
        assert_eq!(range.start_line, 2);
        assert_eq!(range.end_line, 2);
    }

    #[test]
    fn test_semicolon_in_string() {
        let sql = "SELECT 'hello; world' FROM t|;";
        let cursor = sql.find('|').unwrap();
        let source = sql.replace('|', "");
        let tree = parse_sql(&source, None).unwrap();
        
        let range = find_current_statement_range(&tree, &source, cursor).unwrap();
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
        let range = find_current_statement_range(&tree, &source, cursor).unwrap();
        assert_eq!(range.start_line, 1);
        assert_eq!(range.end_line, 1);
    }

    #[test]
    fn test_cursor_between_statements() {
        let sql = "SELECT 1; \n | \n SELECT 2";
        let cursor = sql.find('|').unwrap();
        let source = sql.replace('|', "");
        let tree = parse_sql(&source, None).unwrap();
        
        let range = find_current_statement_range(&tree, &source, cursor);
        // Cursor is on a new line (more than 2 chars from end of statement 1)
        // So it should return None, not highlight the previous statement.
        assert!(range.is_none());
    }

    // ============ USER-REQUESTED TEST CASES ============

    /// Multi-line query, cursor at end of semicolon
    /// Expected: Should highlight the full query (lines 1-8)
    #[test]
    fn test_multiline_cursor_at_end_of_semicolon() {
        let sql = r#"SELECT
  *
FROM
  production.tasks t
WHERE
  t.id IS NOT NULL
LIMIT
  2 ;|"#;
        let cursor = sql.find('|').unwrap();
        let source = sql.replace('|', "");
        let tree = parse_sql(&source, None).unwrap();
        
        // Debug: Print tree structure
        println!("=== TREE STRUCTURE DEBUG ===");
        println!("Source length: {}", source.len());
        println!("Cursor offset: {}", cursor);
        let root = tree.root_node();
        let mut walker = root.walk();
        for (i, child) in root.children(&mut walker).enumerate() {
            println!("Child {}: kind='{}' start_byte={} end_byte={} start_line={} end_line={}", 
                i, child.kind(), child.start_byte(), child.end_byte(), 
                child.start_position().row + 1, child.end_position().row + 1);
        }
        println!("===========================");
        
        let range = find_current_statement_range(&tree, &source, cursor);
        println!("Test 1 - Cursor at end of semicolon:");
        println!("  cursor_offset = {}", cursor);
        println!("  range = {:?}", range);
        
        assert!(range.is_some(), "Should highlight when cursor is at end of semicolon");
        let r = range.unwrap();
        // The range should cover the full query
        assert_eq!(r.start_line, 1, "Should start at line 1");
        assert_eq!(r.end_line, 8, "Should end at line 8");
    }

    /// Multi-line query, cursor one line BELOW the semicolon
    /// Expected: Should NOT highlight (cursor is on empty line below statement)
    #[test]
    fn test_multiline_cursor_one_line_below() {
        let sql = r#"SELECT
  *
FROM
  production.tasks t
WHERE
  t.id IS NOT NULL
LIMIT
  2 ;
|"#;
        let cursor = sql.find('|').unwrap();
        let source = sql.replace('|', "");
        let tree = parse_sql(&source, None).unwrap();
        
        let range = find_current_statement_range(&tree, &source, cursor);
        println!("Test 2 - Cursor one line below semicolon:");
        println!("  cursor_offset = {}", cursor);
        println!("  range = {:?}", range);
        
        assert!(range.is_none(), "Should NOT highlight when cursor is on line below statement");
    }

    /// Multi-line query, cursor after `2` and one space, BEFORE semicolon
    /// Expected: Should highlight the full query (cursor is still inside statement text area)
    #[test]
    fn test_multiline_cursor_after_2_and_space() {
        let sql = r#"SELECT
  *
FROM
  production.tasks t
WHERE
  t.id IS NOT NULL
LIMIT
  2 |;"#;
        let cursor = sql.find('|').unwrap();
        let source = sql.replace('|', "");
        let tree = parse_sql(&source, None).unwrap();
        
        let range = find_current_statement_range(&tree, &source, cursor);
        println!("Test 3 - Cursor after '2 ' (space before semicolon):");
        println!("  cursor_offset = {}", cursor);
        println!("  range = {:?}", range);
        
        assert!(range.is_some(), "Should highlight when cursor is before semicolon");
        let r = range.unwrap();
        assert_eq!(r.start_line, 1);
        assert_eq!(r.end_line, 8);
    }
}
