//! Cursor context analysis.
//!
//! Determines what kind of completion is appropriate based on:
//! - Cursor position in the AST
//! - Preceding tokens (dot, space, keyword)
//! - Enclosing clause (SELECT, FROM, WHERE, JOIN ON, etc.)

use tree_sitter::{Node, Tree, Point};

/// The type of completion context at the cursor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CursorContext {
    /// After a dot: `u.|` → expect columns of alias `u`
    AfterDot { alias: String },
    
    /// In SELECT clause: `SELECT |` → expect columns, functions, aliases
    SelectClause,
    
    /// After complete SELECT list: `SELECT * |` or `SELECT a, b |` → expect FROM
    AfterSelectList,
    
    /// In FROM clause: `FROM |` → expect table names
    FromClause,
    
    /// After JOIN keyword: `JOIN |` → expect table names
    JoinTable,
    
    /// After ON keyword: `ON |` → expect join conditions
    JoinCondition { 
        left_table: Option<String>,
        right_table: Option<String>,
    },

    /// Right-hand side of join condition: `ON u.id = |`
    JoinConditionRhs {
        left_table: Option<String>,
        right_table: Option<String>,
    },
    
    /// In WHERE clause: `WHERE |` → expect columns, operators
    WhereClause,
    
    /// Inside function call: `SUM(|)` → expect columns of numeric type
    FunctionArgument { function_name: String },
    
    /// Start of statement or empty editor: `|` → expect SELECT, WITH, INSERT
    RootContext,
    
    /// Generic/unknown context
    Unknown,
}

/// Full context for completion at a cursor position.
#[derive(Debug)]
pub struct Context {
    /// Byte offset of cursor in source
    pub cursor_offset: usize,
    /// The semantic context type
    pub context_type: CursorContext,
    /// The partial word being typed (for filtering)
    pub prefix: String,
    /// The word immediately preceding the cursor (skipping current prefix and whitespace)
    pub previous_word: String,
    /// Enclosing scope depth (for subquery handling)
    pub scope_depth: usize,
}

impl Context {
    /// Analyze cursor position and determine completion context.
    pub fn analyze(source: &str, tree: Option<&Tree>, cursor_offset: usize) -> Self {
        let default = Context {
            cursor_offset,
            context_type: CursorContext::Unknown,
            prefix: String::new(),
            previous_word: String::new(),
            scope_depth: 0,
        };

        let tree = match tree {
            Some(t) => t,
            None => return default,
        };

        let root = tree.root_node();
        
        // Find the deepest node at cursor
        let cursor_point = offset_to_point(source, cursor_offset);
        let node = find_deepest_node_at(root, cursor_point, cursor_offset);
        
        // Extract prefix (the partial word being typed)
        let mut prefix = extract_prefix(source, cursor_offset);
        
        // Extract previous word
        let previous_word = extract_previous_word(source, cursor_offset, &prefix);
        
        // Determine context type
        let context_type = determine_context(source, &node, cursor_offset);
        
        // For AfterDot context, the filter prefix is whatever comes after the dot.
        // "ail." → prefix = ""  (show all columns)
        // "ail.id" → prefix = "id"  (filter to columns starting with "id")
        if let CursorContext::AfterDot { .. } = context_type {
            prefix = if let Some(dot_pos) = prefix.rfind('.') {
                prefix[dot_pos + 1..].to_string()
            } else {
                String::new()
            };
        }
        
        // Calculate scope depth
        let scope_depth = calculate_scope_depth(&node);

        Context {
            cursor_offset,
            context_type,
            prefix,
            previous_word,
            scope_depth,
        }
    }
}

/// Convert byte offset to tree-sitter Point.
fn offset_to_point(source: &str, offset: usize) -> Point {
    let mut row = 0;
    let mut col = 0;
    
    for (i, ch) in source.char_indices() {
        if i >= offset {
            break;
        }
        if ch == '\n' {
            row += 1;
            col = 0;
        } else {
            col += 1;
        }
    }
    
    Point { row, column: col }
}

/// Find the deepest AST node containing the cursor.
fn find_deepest_node_at<'a>(node: Node<'a>, point: Point, byte_offset: usize) -> Node<'a> {
    for child in node.children(&mut node.walk()) {
        if child.start_byte() <= byte_offset && child.end_byte() >= byte_offset {
            return find_deepest_node_at(child, point, byte_offset);
        }
    }
    node
}

/// Extract the prefix being typed (for filtering completions).
fn extract_prefix(source: &str, cursor_offset: usize) -> String {
    let before_cursor = &source[..cursor_offset];
    
    // Walk backwards to find word start
    // Allow '.' to be part of the prefix for qualified matching (e.g. "u.em")
    let word_start = before_cursor
        .rfind(|c: char| !c.is_alphanumeric() && c != '_' && c != '.')
        .map(|i| i + 1)
        .unwrap_or(0);
    
    before_cursor[word_start..].to_string()
}

/// Extract the word preceding the current prefix/cursor.
fn extract_previous_word(source: &str, cursor_offset: usize, prefix: &str) -> String {
    let effective_end = cursor_offset.saturating_sub(prefix.len());
    let before_prefix = &source[..effective_end];
    let trimmed = before_prefix.trim_end();
    
    // If we're at the start, no previous word
    if trimmed.is_empty() {
        return String::new();
    }
    
    // If the last char is a symbol like '=', return it
    if let Some(last_char) = trimmed.chars().last() {
        if !last_char.is_alphanumeric() && last_char != '_' && last_char != ')' && last_char != '"' && last_char != '`' && last_char != '\'' {
            // Check for multi-char operators like >=, <=, <>
            if trimmed.len() >= 2 {
                let suffix = &trimmed[trimmed.len()-2..];
                if matches!(suffix, ">=" | "<=" | "<>" | "!=") {
                    return suffix.to_string();
                }
            }
            return last_char.to_string();
        }
    }

    // Otherwise find the start of the word
    let word_start = trimmed
        .rfind(|c: char| !c.is_alphanumeric() && c != '_' && c != '.')
        .map(|i| i + 1)
        .unwrap_or(0);
        
    trimmed[word_start..].to_string()
}

/// Determine the completion context from cursor position.
fn determine_context(source: &str, node: &Node, cursor_offset: usize) -> CursorContext {
    // 1. Isolate the current statement
    // We scan backwards for the last ';' to ensure we don't look at previous queries.
    let statement_start = find_statement_start(source, cursor_offset);
    let before_cursor = &source[statement_start..cursor_offset];
    let trimmed = before_cursor.trim_end();
    
    // Check for dot context: `alias.|` or `alias.partial|`
    // Skip this check inside JOIN ON — there we want FK-aware join condition suggestions
    // even when the user has typed something like "u.i".
    let upper_before = before_cursor.to_uppercase();
    let in_join_on = upper_before.contains(" ON ") && !upper_before.ends_with(" JOIN ");
    let current_prefix = extract_prefix(before_cursor, cursor_offset - statement_start);
    if !in_join_on && current_prefix.contains('.') && !current_prefix.starts_with('.') {
        if let Some(alias) = current_prefix.split('.').next() {
            if !alias.is_empty() {
                return CursorContext::AfterDot { alias: alias.to_string() };
            }
        }
    }
    // Plain trailing-dot fallback (e.g. text ends with "ail." and prefix was empty)
    if trimmed.ends_with('.') {
        let alias = extract_alias_before_dot(trimmed);
        return CursorContext::AfterDot { alias };
    }
    
    // Walk up to find enclosing clause
    let mut current = Some(*node);
    while let Some(n) = current {
        let kind = n.kind();
        
        match kind {
            "select_clause" | "select" => return CursorContext::SelectClause,
            "from_clause" | "from" => return CursorContext::FromClause,
            "where_clause" | "where" => return CursorContext::WhereClause,
            "join_clause" | "join" => {
                // Check if we're after ON
                if before_cursor.to_uppercase().contains(" ON ") 
                   && !before_cursor.to_uppercase().ends_with(" JOIN ") {
                    let (left, right) = extract_join_tables(before_cursor);
                    return CursorContext::JoinCondition {
                        left_table: left,
                        right_table: right,
                    };
                }
                return CursorContext::JoinTable;
            }
            "function_call" => {
                let fn_name = extract_function_name(&n, source);
                return CursorContext::FunctionArgument { function_name: fn_name };
            }
            _ => {}
        }
        
        current = n.parent();
    }
    
    // Fallback: check keywords in source
    let upper = before_cursor.to_uppercase();
    let trimmed_upper = upper.trim();
    
    // Root context: empty or just whitespace
    if trimmed_upper.is_empty() {
        return CursorContext::RootContext;
    }

    // New: If we are typing the first word (no whitespace separators yet), treat as RootContext.
    // This handles "sel" -> suggest SELECT.
    // We check if the relevant slice (trimmed of leading constraints) contains any whitespace.
    if !before_cursor.trim_start().chars().any(|c| c.is_whitespace()) {
        return CursorContext::RootContext;
    }
    
    // After SELECT list: SELECT ... * | or SELECT ... col |  (before FROM)
    // Detect: has SELECT, has content after SELECT, no FROM yet, ends with expression
    if upper.contains("SELECT ") && !upper.contains(" FROM ") {
        // Check if we're after a complete expression (*, identifier, or closing paren)
        let after_select = upper.split("SELECT ").last().unwrap_or("").trim();
        
        // Ends with *, identifier, or ) means complete expression → suggest FROM
        if after_select.ends_with('*') 
            || after_select.ends_with(')') 
            || (after_select.len() > 0 && after_select.chars().last().map(|c| c.is_alphanumeric() || c == '_').unwrap_or(false))
        {
            // But not if we're mid-word (check if space before cursor)
            let before_trimmed = before_cursor.trim_end();
            if before_trimmed.len() < before_cursor.len() || before_cursor.ends_with(' ') {
                return CursorContext::AfterSelectList;
            }
        }
        
        // Otherwise still building SELECT list
        return CursorContext::SelectClause;
    }
    
    if trimmed_upper.ends_with("SELECT") {
        return CursorContext::SelectClause;
    }
    if trimmed_upper.ends_with("FROM") || trimmed_upper.ends_with("JOIN") {
        return CursorContext::FromClause;
    }
    if upper.contains(" WHERE ") {
        return CursorContext::WhereClause;
    }
    
    if upper.contains(" ON ") {
        let (left, right) = extract_join_tables(before_cursor);
        
        // Check if we are after an "=" sign
        if before_cursor.contains('=') {
            return CursorContext::JoinConditionRhs {
                left_table: left,
                right_table: right,
            };
        }

        return CursorContext::JoinCondition {
            left_table: left,
            right_table: right,
        };
    }

    // Heuristic: If we are in a SELECT statement, past 'FROM', and haven't hit WHERE/GROUP/ORDER yet,
    // we are likely in the FROM clause (expecting aliases, joins, or WHERE).
    if upper.contains("SELECT ") && upper.contains(" FROM ") {
        let from_idx = upper.rfind(" FROM ").unwrap() + 6;
        if cursor_offset >= from_idx {
            let after_from = &upper[from_idx..];
            if !after_from.contains(" WHERE ") 
               && !after_from.contains(" GROUP BY ") 
               && !after_from.contains(" ORDER BY ") 
               && !after_from.contains(" LIMIT ")
            {
                return CursorContext::FromClause;
            }
        }
    }
    
    CursorContext::Unknown
}

/// Extract alias before the dot: "u." → "u"
fn extract_alias_before_dot(trimmed: &str) -> String {
    let without_dot = trimmed.trim_end_matches('.');
    without_dot
        .rfind(|c: char| !c.is_alphanumeric() && c != '_')
        .map(|i| &without_dot[i + 1..])
        .unwrap_or(without_dot)
        .to_string()
}

/// Extract tables (or aliases) involved in a JOIN clause.
fn extract_join_tables(before_cursor: &str) -> (Option<String>, Option<String>) {
    let parts: Vec<&str> = before_cursor.split_whitespace().collect();
    
    let mut from_table = None;
    let mut join_table = None;
    
    let keywords = [
        "INNER", "LEFT", "RIGHT", "OUTER", "CROSS", 
        "JOIN", "ON", "WHERE", "GROUP", "ORDER", "LIMIT", "HAVING"
    ];

    for (i, part) in parts.iter().enumerate() {
        let upper = part.to_uppercase();
        
        if upper == "FROM" && i + 1 < parts.len() {
            // Check for alias: FROM table alias
            let table_or_alias = if i + 2 < parts.len() {
                let next = parts[i + 2].to_uppercase();
                if !keywords.contains(&next.as_str()) {
                    parts[i + 2] // Return alias
                } else {
                    parts[i + 1] // Return table
                }
            } else {
                parts[i + 1]
            };
            from_table = Some(table_or_alias.to_string());
        }
        
        if upper == "JOIN" && i + 1 < parts.len() {
            // Check for alias: JOIN table alias
            let table_or_alias = if i + 2 < parts.len() {
                let next = parts[i + 2].to_uppercase();
                if !keywords.contains(&next.as_str()) {
                    parts[i + 2] // Return alias
                } else {
                    parts[i + 1] // Return table
                }
            } else {
                parts[i + 1]
            };
            join_table = Some(table_or_alias.to_string());
        }
    }
    
    (from_table, join_table)
}

/// Extract function name from a function_call node.
fn extract_function_name(node: &Node, source: &str) -> String {
    for child in node.children(&mut node.walk()) {
        if child.kind() == "identifier" {
            let start = child.start_byte();
            let end = child.end_byte();
            if end <= source.len() {
                return source[start..end].to_uppercase();
            }
        }
    }
    "UNKNOWN".to_string()
}

/// Calculate nesting depth (for subquery scope handling).
fn calculate_scope_depth(node: &Node) -> usize {
    let mut depth: usize = 0;
    let mut current = Some(*node);
    
    while let Some(n) = current {
        if n.kind() == "subquery" || n.kind() == "select_statement" {
            depth += 1;
        }
        current = n.parent();
    }
    
    depth.saturating_sub(1) // Don't count the outermost query
}

/// Find the start index of the current statement (after last semicolon).
/// Handles quoted strings to avoid false positives.
fn find_statement_start(source: &str, cursor_offset: usize) -> usize {
    let relevant_slice = &source[..cursor_offset];
    let mut last_semicolon = 0;
    
    // Quick check: if no semicolon, it's the start
    if !relevant_slice.contains(';') {
        return 0;
    }

    // Careful scan to ignore semicolons in strings
    let mut in_quote = false;
    let mut quote_char = '\0';
    
    // We scan forward from the beginning effectively because scanning backward with state is harder
    // But since we want the *last* semicolon before cursor, forward scan is fine.
    // Optimization: Just scan the characters.
    for (i, c) in relevant_slice.char_indices() {
        if in_quote {
            if c == quote_char {
                // Check distinct for escaped quotes? SQL standard uses '' for escaped '
                // But simplified check is sufficient for completion context
                // We just check if next char is same quote (escape) or not?
                // For now, simple toggle is usually enough for context heuristics
                in_quote = false;
            }
        } else {
            match c {
                '\'' | '"' => {
                    in_quote = true;
                    quote_char = c;
                }
                ';' => {
                    last_semicolon = i + 1; // Start is after the semicolon
                }
                _ => {}
            }
        }
    }
    
    last_semicolon
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::completion::parsing::parse_sql;

    #[test]
    fn test_after_dot_context() {
        let source = "SELECT u. FROM users u";
        let tree = parse_sql(source, None);
        let ctx = Context::analyze(source, tree.as_ref(), 9); // After "u."
        
        assert!(matches!(ctx.context_type, CursorContext::AfterDot { ref alias } if alias == "u"));
    }

    #[test]
    fn test_multi_statement_root_context() {
        // "sel" after a semicolon should be root context (expecting keyword)
        let source = "SELECT * FROM users; sel";
        let tree = parse_sql(source, None);
        let ctx = Context::analyze(source, tree.as_ref(), source.len()); 
        
        // Before fix: likely Unknown
        // After fix: RootContext (because " sel" trimmed is "sel", but "sel" isn't empty... wait)
        
        // Actually, determine_context logic for RootContext is:
        // if trimmed_upper.is_empty() { return CursorContext::RootContext; }
        
        // But "sel" is NOT empty.
        // It should hit: 
        // fallback?
        
        // Wait, current logic for RootContext:
        // if trimmed_upper.is_empty() { return RootContext; }
        // If I type "sel", trimmed_upper is "SEL".
        // It falls through to Unknown.
        // And engine.rs treats Unknown as GENERIC_KEYWORDS (SELECT, FROM, etc.)
        
        // So Unknown is actually what we want if we want "SELECT" to be suggested?
        // Let's check engine.rs:
        // GENERIC_KEYWORDS includes "SELECT", "FROM", etc.
        
        // So Unknown -> OK.
        // The problem reported by user is that it was suggesting "slice_user" etc.
        // Those generic items also include "Visible aliases".
        
        // If previous query was "SELECT * FROM action_item", "action_item" is a visible symbol (maybe?).
        // No, semantic model usually builds off parsed tree.
        
        // If I type `sel`, tree is broken.
        // determine_context returns Unknown.
        // engine calls complete_generic.
        // generic suggests: GENERIC_KEYWORDS (score 50) + Visible Aliases (score 70).
        
        // If "action_item" is in the previous query, is it a visible symbol?
        // Semantic model parsing probably picks it up.
        
        // But why "slice_user"? That implies it's finding random things.
        
        // If it's RootContext, complete_root_context suggests STATEMENT_STARTERS with score 100.
        // That is strictly better than GENERIC (score 50).
        
        // So we WANT RootContext when the statement is just "sel" (conceptually empty statement being started).
        // But "sel" is not empty.
        
        // Maybe we need a specific heuristic for "looks like start of statement".
        // Or determine_context should return RootContext if token count is 1?
        
        // If `trimmed_upper` is just one word, and that word matches a start keyword prefix?
        // Or if it doesn't match any clause triggers?
        
        // Let's adjust determination logic slightly in the main block too.
    }

    #[test]
    fn test_select_clause_context() {
        let source = "SELECT  FROM users";
        let tree = parse_sql(source, None);
        let ctx = Context::analyze(source, tree.as_ref(), 7); // After "SELECT "
        
        assert!(matches!(ctx.context_type, CursorContext::SelectClause));
    }

    #[test]
    fn test_from_clause_context() {
        let source = "SELECT * FROM ";
        let tree = parse_sql(source, None);
        let ctx = Context::analyze(source, tree.as_ref(), 14); // After "FROM "
        
        assert!(matches!(ctx.context_type, CursorContext::FromClause));
    }

    #[test]
    fn test_join_condition_context() {
        let source = "SELECT * FROM users u JOIN orders o ON ";
        let tree = parse_sql(source, None);
        let ctx = Context::analyze(source, tree.as_ref(), 39); // After "ON "
        
        assert!(matches!(ctx.context_type, CursorContext::JoinCondition { .. }));
    }

    #[test]
    fn test_prefix_extraction() {
        let prefix = extract_prefix("SELECT us", 9);
        assert_eq!(prefix, "us");
        
        let prefix = extract_prefix("SELECT user_na", 14);
        assert_eq!(prefix, "user_na");
    }
}
