//! Document state with incremental parsing support.
//!
//! Each open SQL file/tab has its own DocumentState that maintains:
//! - The current source text
//! - The parsed Tree-sitter AST
//! - Dialect information for SQL flavor-specific completions

use serde::{Deserialize, Serialize};
use tree_sitter::{Tree, InputEdit, Point};

use super::parsing::parse_sql;

/// SQL dialect for flavor-specific completions.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Dialect {
    #[default]
    Postgres,
    MySQL,
    SQLite,
}

/// State for a single open SQL document (editor tab).
#[derive(Debug)]
pub struct DocumentState {
    /// The parsed AST (None if never parsed)
    pub tree: Option<Tree>,
    /// Full source text of the document
    pub source: String,
    /// SQL dialect for this document
    pub dialect: Dialect,
    /// Version counter, incremented on each edit
    pub version: u64,
}

impl DocumentState {
    /// Create a new document with empty content.
    pub fn new(dialect: Dialect) -> Self {
        Self {
            tree: None,
            source: String::new(),
            dialect,
            version: 0,
        }
    }

    /// Create a document from existing source text.
    pub fn from_source(source: String, dialect: Dialect) -> Self {
        let tree = parse_sql(&source, None);
        Self {
            tree,
            source,
            dialect,
            version: 1,
        }
    }

    /// Apply an incremental edit and reparse.
    /// 
    /// This is the key to <10ms parsing speeds on large files.
    /// Tree-sitter reuses unchanged nodes from the previous parse.
    pub fn apply_edit(&mut self, edit: &TextEdit) {
        let start_byte = edit.start_byte;
        let old_end = edit.old_end_byte;
        
        // 1. Apply tree-sitter edit metadata if tree exists
        if let Some(ref mut tree) = self.tree {
            tree.edit(&InputEdit {
                start_byte,
                old_end_byte: old_end,
                new_end_byte: start_byte + edit.new_text.len(),
                start_position: edit.start_position.into(),
                old_end_position: edit.old_end_position.into(),
                new_end_position: edit.new_end_position.into(),
            });
        }

        // 2. Update source string
        if old_end <= self.source.len() {
            self.source.replace_range(start_byte..old_end, &edit.new_text);
        }

        // 3. Reparse incrementally (tree-sitter reuses unchanged nodes)
        self.tree = parse_sql(&self.source, self.tree.as_ref());
        self.version += 1;
    }

    /// Full reparse of the document (after major changes).
    pub fn reparse(&mut self) {
        self.tree = parse_sql(&self.source, None);
        self.version += 1;
    }

    /// Check if the AST has any error nodes.
    pub fn has_errors(&self) -> bool {
        self.tree.as_ref().map_or(true, |t| t.root_node().has_error())
    }

    /// Get the byte offset for a line/column position.
    pub fn offset_at(&self, line: usize, column: usize) -> usize {
        let mut offset = 0;
        for (i, line_content) in self.source.lines().enumerate() {
            if i == line {
                return offset + column.min(line_content.len());
            }
            offset += line_content.len() + 1; // +1 for newline
        }
        self.source.len()
    }
}

/// A position in the document (row/column).
/// This is our own type that can be serialized, converted to tree_sitter::Point when needed.
#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize)]
pub struct Position {
    pub row: usize,
    pub column: usize,
}

impl Position {
    pub fn new(row: usize, column: usize) -> Self {
        Self { row, column }
    }
}

impl From<Position> for Point {
    fn from(pos: Position) -> Self {
        Point { row: pos.row, column: pos.column }
    }
}

impl From<Point> for Position {
    fn from(point: Point) -> Self {
        Self { row: point.row, column: point.column }
    }
}

/// A text edit to apply to the document.
#[derive(Debug, Clone, Deserialize)]
pub struct TextEdit {
    pub start_byte: usize,
    pub old_end_byte: usize,
    pub new_text: String,
    pub start_position: Position,
    pub old_end_position: Position,
    pub new_end_position: Position,
}

impl TextEdit {
    /// Create a simple insert edit.
    pub fn insert(offset: usize, text: &str, row: usize, column: usize) -> Self {
        let new_lines: Vec<&str> = text.split('\n').collect();
        let new_end_row = row + new_lines.len() - 1;
        let new_end_col = if new_lines.len() == 1 {
            column + text.len()
        } else {
            new_lines.last().map_or(0, |l| l.len())
        };

        Self {
            start_byte: offset,
            old_end_byte: offset,
            new_text: text.to_string(),
            start_position: Position { row, column },
            old_end_position: Position { row, column },
            new_end_position: Position { row: new_end_row, column: new_end_col },
        }
    }

    /// Create a replace edit.
    pub fn replace(start: usize, end: usize, text: &str, start_pos: Position, end_pos: Position) -> Self {
        let new_lines: Vec<&str> = text.split('\n').collect();
        let new_end_row = start_pos.row + new_lines.len() - 1;
        let new_end_col = if new_lines.len() == 1 {
            start_pos.column + text.len()
        } else {
            new_lines.last().map_or(0, |l| l.len())
        };

        Self {
            start_byte: start,
            old_end_byte: end,
            new_text: text.to_string(),
            start_position: start_pos,
            old_end_position: end_pos,
            new_end_position: Position { row: new_end_row, column: new_end_col },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_document_from_source() {
        let doc = DocumentState::from_source(
            "SELECT * FROM users".to_string(),
            Dialect::Postgres,
        );
        
        assert!(doc.tree.is_some());
        assert!(!doc.has_errors());
        assert_eq!(doc.version, 1);
    }

    #[test]
    fn test_incremental_edit() {
        let mut doc = DocumentState::from_source(
            "SELECT * FROM users".to_string(),
            Dialect::Postgres,
        );
        
        // Add " WHERE id = 1" at the end
        let edit = TextEdit::insert(
            19, // end of "SELECT * FROM users"
            " WHERE id = 1",
            0, 19,
        );
        
        doc.apply_edit(&edit);
        
        assert_eq!(doc.source, "SELECT * FROM users WHERE id = 1");
        assert!(doc.tree.is_some());
        assert!(!doc.has_errors());
        assert_eq!(doc.version, 2);
    }

    #[test]
    fn test_offset_at() {
        let doc = DocumentState::from_source(
            "SELECT *\nFROM users\nWHERE id = 1".to_string(),
            Dialect::Postgres,
        );
        
        assert_eq!(doc.offset_at(0, 0), 0);
        assert_eq!(doc.offset_at(0, 6), 6); // "SELECT"
        assert_eq!(doc.offset_at(1, 0), 9); // Start of "FROM"
        assert_eq!(doc.offset_at(2, 0), 20); // Start of "WHERE"
    }
}
