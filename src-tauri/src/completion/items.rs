//! Completion item types shared across the completion engine.

/// A completion item to return to the editor.
#[derive(Debug, Clone)]
pub struct CompletionItem {
    /// Display label
    pub label: String,
    /// Item kind for icon
    pub kind: CompletionKind,
    /// Detail text (e.g., "column of users")
    pub detail: Option<String>,
    /// Text to insert
    pub insert_text: String,
    /// Ranking score (higher = better)
    pub score: u32,
}

/// Kind of completion item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[repr(u8)]
pub enum CompletionKind {
    Table = 0,
    Column = 1,
    Alias = 2,
    Keyword = 3,
    Function = 4,
    JoinCondition = 5,
    Schema = 6,
    Operator = 7,
}
