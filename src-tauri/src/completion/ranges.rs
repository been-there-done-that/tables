use tree_sitter::Tree;

#[derive(serde::Serialize)]
pub struct StatementRange {
    pub start_line: u32, // 1-based for Monaco
    pub end_line: u32,
}

pub fn find_current_statement_range(tree: &Tree, cursor_offset: usize) -> Option<StatementRange> {
    let root = tree.root_node();
    let mut cursor = root.walk();

    // Iterate over top-level statements only
    for child in root.children(&mut cursor) {
        let start_byte = child.start_byte();
        let end_byte = child.end_byte();

        // Check if our cursor is inside this statement
        // Note: We use <= end_byte to include the semicolon at the end
        if cursor_offset >= start_byte && cursor_offset <= end_byte {
            let start_point = child.start_position();
            let end_point = child.end_position();

            return Some(StatementRange {
                start_line: (start_point.row + 1) as u32,
                end_line: (end_point.row + 1) as u32,
            });
        }
    }
    
    None
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
