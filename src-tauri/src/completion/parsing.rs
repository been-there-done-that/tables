//! Thread-local parser pool for Tree-sitter.
//!
//! CRITICAL: `tree_sitter::Parser` is NOT thread-safe.
//! This module provides zero-contention parsing via thread-local storage.
//! Cost: ~200KB per worker thread. Gain: No mutex locks.

use std::cell::RefCell;
use tree_sitter::{Parser, Tree, Language};

thread_local! {
    /// Each blocking worker thread gets its own parser.
    pub static LOCAL_PARSER: RefCell<Parser> = RefCell::new({
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_sequel::LANGUAGE.into())
            .expect("Failed to load SQL grammar");
        parser
    });
}

/// Parse SQL source on current thread using thread-local parser.
/// 
/// # Arguments
/// * `source` - The SQL source code to parse
/// * `old_tree` - Optional previous tree for incremental parsing
///
/// # Returns
/// Parsed tree, or None if parsing fails
pub fn parse_sql(source: &str, old_tree: Option<&Tree>) -> Option<Tree> {
    LOCAL_PARSER.with(|p| {
        p.borrow_mut().parse(source, old_tree)
    })
}

/// Get the SQL language for Tree-sitter.
pub fn sql_language() -> Language {
    tree_sitter_sequel::LANGUAGE.into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_select() {
        let tree = parse_sql("SELECT * FROM users", None);
        assert!(tree.is_some(), "Should parse simple SELECT");
        
        let tree = tree.unwrap();
        let root = tree.root_node();
        assert!(!root.has_error(), "AST should have no errors");
    }

    #[test]
    fn test_parse_with_error_recovery() {
        // Incomplete SQL - tree-sitter should still produce a tree with ERROR nodes
        let tree = parse_sql("SELECT * FROM", None);
        assert!(tree.is_some(), "Should parse incomplete SQL");
        
        let tree = tree.unwrap();
        let root = tree.root_node();
        // Tree exists but may have error nodes
        assert!(root.child_count() > 0, "Should have some nodes");
    }

    #[test]
    fn test_incremental_parse() {
        let source1 = "SELECT * FROM users";
        let tree1 = parse_sql(source1, None).unwrap();
        
        // Simulate edit: add WHERE clause
        let source2 = "SELECT * FROM users WHERE id = 1";
        let tree2 = parse_sql(source2, Some(&tree1));
        
        assert!(tree2.is_some(), "Incremental parse should succeed");
        let tree2 = tree2.unwrap();
        assert!(!tree2.root_node().has_error(), "Should parse cleanly");
    }
}
