/// Split SQL into individual statement strings with their byte offsets.
/// Splits on `;` boundaries. Ignores semicolons inside single-quoted strings and
/// double-quoted identifiers. Trims whitespace from each segment.
/// Returns (start_byte_of_trimmed_segment, trimmed_segment_str) pairs.
/// Empty segments are excluded.
///
/// Handles:
/// - Single-quoted strings with SQL-standard escaped quotes (`''`)
/// - Double-quoted identifiers
/// - Dollar-quoted strings (PostgreSQL): `$$...$$` or `$tag$...$tag$`
/// - Line comments (`--`): semicolons inside are ignored
/// - Block comments (`/* */`): semicolons inside are ignored
pub fn split_statements(sql: &str) -> Vec<(usize, &str)> {
    let bytes = sql.as_bytes();
    let len = bytes.len();
    let mut results = Vec::new();

    // Current segment start (byte index into `sql`)
    let mut seg_start = 0usize;

    let mut i = 0usize;

    while i < len {
        match bytes[i] {
            // Single-line comment: skip to end of line
            b'-' if i + 1 < len && bytes[i + 1] == b'-' => {
                i += 2;
                while i < len && bytes[i] != b'\n' {
                    i += 1;
                }
                // consume the newline if present
                if i < len {
                    i += 1;
                }
            }

            // Block comment: skip to closing */
            b'/' if i + 1 < len && bytes[i + 1] == b'*' => {
                i += 2;
                while i + 1 < len {
                    if bytes[i] == b'*' && bytes[i + 1] == b'/' {
                        i += 2;
                        break;
                    }
                    i += 1;
                }
                // If we reached end without finding */, just stop
            }

            // Single-quoted string
            b'\'' => {
                i += 1;
                loop {
                    if i >= len {
                        break;
                    }
                    if bytes[i] == b'\'' {
                        // Check for SQL escaped '' (two consecutive single quotes)
                        if i + 1 < len && bytes[i + 1] == b'\'' {
                            i += 2; // skip both quotes, still inside string
                        } else {
                            i += 1; // closing quote
                            break;
                        }
                    } else {
                        i += 1;
                    }
                }
            }

            // Double-quoted identifier
            b'"' => {
                i += 1;
                loop {
                    if i >= len {
                        break;
                    }
                    if bytes[i] == b'"' {
                        // Check for SQL-standard escaped "" (two consecutive double quotes)
                        if i + 1 < len && bytes[i + 1] == b'"' {
                            i += 2; // skip both quotes, still inside identifier
                        } else {
                            i += 1; // closing quote
                            break;
                        }
                    } else {
                        i += 1;
                    }
                }
            }

            // Potential dollar-quote: `$` starts a dollar-quote tag
            b'$' => {
                // Try to find the matching dollar-quote tag
                if let Some((tag_end, tag)) = find_dollar_quote_tag(sql, i) {
                    // tag_end is the index just past the closing `$` of the opening tag
                    // Now scan for the closing tag
                    let close_tag = tag.as_bytes();
                    let mut j = tag_end;
                    let mut found_close = false;
                    while j + close_tag.len() <= len {
                        if &bytes[j..j + close_tag.len()] == close_tag {
                            j += close_tag.len();
                            found_close = true;
                            break;
                        }
                        j += 1;
                    }
                    if found_close {
                        i = j;
                    } else {
                        // No closing tag found, treat as regular character
                        i += 1;
                    }
                } else {
                    i += 1;
                }
            }

            // Statement terminator
            b';' => {
                let segment = &sql[seg_start..i];
                let trimmed = segment.trim();
                if !trimmed.is_empty() {
                    // Find offset of trimmed within original sql
                    let trim_start = seg_start + leading_whitespace_len(segment);
                    results.push((trim_start, trimmed));
                }
                i += 1;
                seg_start = i;
            }

            _ => {
                i += 1;
            }
        }
    }

    // Handle any trailing content after last semicolon (or entire input if no semicolons)
    let segment = &sql[seg_start..len];
    let trimmed = segment.trim();
    if !trimmed.is_empty() {
        let trim_start = seg_start + leading_whitespace_len(segment);
        results.push((trim_start, trimmed));
    }

    results
}

/// Returns the number of leading whitespace bytes in `s`.
fn leading_whitespace_len(s: &str) -> usize {
    s.len() - s.trim_start().len()
}

/// If `sql[pos]` begins a valid dollar-quote opening tag (e.g. `$$` or `$tag$`),
/// returns `(end_of_opening_tag, full_tag_string)` where `full_tag_string` is the
/// entire dollar-quoted delimiter including both `$` signs (e.g. `"$$"` or `"$tag$"`).
/// Returns `None` if `sql[pos]` is not a valid dollar-quote start.
fn find_dollar_quote_tag(sql: &str, pos: usize) -> Option<(usize, &str)> {
    let bytes = sql.as_bytes();
    if bytes[pos] != b'$' {
        return None;
    }
    // The tag body may be empty ($$) or consist of [letter|underscore][letter|digit|underscore]*
    // Tags must NOT start with a digit.
    let mut j = pos + 1;
    // Check the first character of the tag body
    if j < bytes.len() {
        match bytes[j] {
            b'$' => {
                // $$ case — empty tag, valid
                let end = j + 1;
                let tag = &sql[pos..end];
                return Some((end, tag));
            }
            b'A'..=b'Z' | b'a'..=b'z' | b'_' => {
                // Valid first character, continue
                j += 1;
            }
            _ => {
                // Digit or other — invalid tag start
                return None;
            }
        }
    } else {
        return None;
    }
    // Subsequent characters: letters, digits, underscores, or closing $
    while j < bytes.len() {
        match bytes[j] {
            b'$' => {
                // Found closing $ of the opening tag
                let end = j + 1;
                let tag = &sql[pos..end];
                return Some((end, tag));
            }
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'_' => {
                j += 1;
            }
            _ => {
                // Not a valid dollar-quote tag character
                return None;
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper: assert result count and content
    fn stmts(sql: &str) -> Vec<&str> {
        split_statements(sql).into_iter().map(|(_, s)| s).collect()
    }

    // ── Basic splitting ──────────────────────────────────────────────────────

    #[test]
    fn single_statement_no_semicolon() {
        let sql = "SELECT 1";
        assert_eq!(stmts(sql), vec!["SELECT 1"]);
    }

    #[test]
    fn single_statement_with_semicolon() {
        let sql = "SELECT 1;";
        assert_eq!(stmts(sql), vec!["SELECT 1"]);
    }

    #[test]
    fn two_statements() {
        let sql = "SELECT 1; SELECT 2";
        assert_eq!(stmts(sql), vec!["SELECT 1", "SELECT 2"]);
    }

    #[test]
    fn three_statements() {
        let sql = "SELECT 1; SELECT 2; SELECT 3";
        assert_eq!(stmts(sql), vec!["SELECT 1", "SELECT 2", "SELECT 3"]);
    }

    #[test]
    fn semicolon_with_spaces() {
        let sql = "SELECT 1  ;  SELECT 2";
        let r = stmts(sql);
        assert_eq!(r.len(), 2);
        assert_eq!(r[0], "SELECT 1");
        assert_eq!(r[1], "SELECT 2");
    }

    #[test]
    fn only_whitespace_between_statements() {
        let sql = "SELECT 1;   \n   SELECT 2";
        let r = stmts(sql);
        assert_eq!(r.len(), 2);
        assert_eq!(r[0], "SELECT 1");
        assert_eq!(r[1], "SELECT 2");
    }

    // ── Empty / edge cases ───────────────────────────────────────────────────

    #[test]
    fn empty_string() {
        assert!(stmts("").is_empty());
    }

    #[test]
    fn only_whitespace() {
        assert!(stmts("   \n\t  ").is_empty());
    }

    #[test]
    fn only_semicolons() {
        assert!(stmts(";;;").is_empty());
    }

    #[test]
    fn whitespace_only_segment_skipped() {
        // "SELECT 1;   ; SELECT 2" — middle segment is whitespace only
        let sql = "SELECT 1;   ; SELECT 2";
        assert_eq!(stmts(sql), vec!["SELECT 1", "SELECT 2"]);
    }

    #[test]
    fn trailing_semicolon_no_empty_segment() {
        let sql = "SELECT 1;";
        let r = stmts(sql);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0], "SELECT 1");
    }

    #[test]
    fn leading_semicolons() {
        let sql = "; SELECT 1";
        assert_eq!(stmts(sql), vec!["SELECT 1"]);
    }

    #[test]
    fn semicolon_only_input() {
        assert!(stmts(";").is_empty());
    }

    // ── String quoting ───────────────────────────────────────────────────────

    #[test]
    fn single_quoted_string_with_semicolon() {
        let sql = "SELECT 'a;b'";
        assert_eq!(stmts(sql), vec!["SELECT 'a;b'"]);
    }

    #[test]
    fn double_quoted_identifier_with_semicolon() {
        let sql = r#"SELECT "col;name" FROM t"#;
        assert_eq!(stmts(sql), vec![r#"SELECT "col;name" FROM t"#]);
    }

    #[test]
    fn escaped_single_quote_stays_in_string() {
        let sql = "SELECT 'it''s a test'";
        assert_eq!(stmts(sql), vec!["SELECT 'it''s a test'"]);
    }

    #[test]
    fn string_with_multiple_semicolons() {
        let sql = "SELECT 'a;b;c' FROM t";
        assert_eq!(stmts(sql), vec!["SELECT 'a;b;c' FROM t"]);
    }

    #[test]
    fn semicolon_after_closing_quote_splits() {
        let sql = "SELECT 'abc'; SELECT 2";
        assert_eq!(stmts(sql), vec!["SELECT 'abc'", "SELECT 2"]);
    }

    #[test]
    fn empty_single_quoted_string() {
        let sql = "SELECT ''";
        assert_eq!(stmts(sql), vec!["SELECT ''"]);
    }

    #[test]
    fn string_ends_with_escaped_quotes() {
        // 'a''b' should be one string
        let sql = "SELECT 'a''b'; SELECT 2";
        assert_eq!(stmts(sql), vec!["SELECT 'a''b'", "SELECT 2"]);
    }

    // ── Dollar quoting (PostgreSQL) ──────────────────────────────────────────

    #[test]
    fn dollar_quote_basic() {
        let sql = "SELECT $$hello; world$$";
        assert_eq!(stmts(sql), vec!["SELECT $$hello; world$$"]);
    }

    #[test]
    fn dollar_quote_with_tag() {
        let sql = "SELECT $tag$hello; world$tag$";
        assert_eq!(stmts(sql), vec!["SELECT $tag$hello; world$tag$"]);
    }

    #[test]
    fn dollar_quote_followed_by_statement() {
        let sql = "SELECT $$body$$; SELECT 2";
        assert_eq!(stmts(sql), vec!["SELECT $$body$$", "SELECT 2"]);
    }

    #[test]
    fn dollar_quote_nested_different_tags() {
        // Outer $outer$...$outer$, inner $$ shouldn't close the outer
        let sql = "SELECT $outer$hello $$inner$$ world$outer$";
        assert_eq!(stmts(sql), vec!["SELECT $outer$hello $$inner$$ world$outer$"]);
    }

    #[test]
    fn dollar_quote_empty_body() {
        let sql = "SELECT $$$$";
        assert_eq!(stmts(sql), vec!["SELECT $$$$"]);
    }

    #[test]
    fn dollar_quote_with_underscore_tag() {
        let sql = "SELECT $my_tag$hello; world$my_tag$";
        assert_eq!(stmts(sql), vec!["SELECT $my_tag$hello; world$my_tag$"]);
    }

    // ── Byte offset accuracy ─────────────────────────────────────────────────

    #[test]
    fn offset_single_statement_no_leading_whitespace() {
        let sql = "SELECT 1";
        let r = split_statements(sql);
        assert_eq!(r.len(), 1);
        let (off, seg) = r[0];
        assert_eq!(&sql[off..off + seg.len()], seg);
        assert_eq!(off, 0);
    }

    #[test]
    fn offset_leading_whitespace() {
        let sql = "   SELECT 1";
        let r = split_statements(sql);
        assert_eq!(r.len(), 1);
        let (off, seg) = r[0];
        assert_eq!(&sql[off..off + seg.len()], seg);
        assert_eq!(off, 3);
    }

    #[test]
    fn offset_second_statement() {
        let sql = "SELECT 1; SELECT 2";
        let r = split_statements(sql);
        assert_eq!(r.len(), 2);
        for (off, seg) in &r {
            assert_eq!(&sql[*off..*off + seg.len()], *seg);
        }
        // "SELECT 2" starts at index 10
        assert_eq!(r[1].0, 10);
    }

    #[test]
    fn offset_second_statement_with_leading_spaces() {
        let sql = "SELECT 1;   SELECT 2";
        let r = split_statements(sql);
        assert_eq!(r.len(), 2);
        for (off, seg) in &r {
            assert_eq!(&sql[*off..*off + seg.len()], *seg);
        }
    }

    #[test]
    fn offset_verification_all_entries() {
        let sql = "  SELECT 1  ;  SELECT 2  ;  SELECT 3  ";
        let r = split_statements(sql);
        assert_eq!(r.len(), 3);
        for (off, seg) in &r {
            assert_eq!(&sql[*off..*off + seg.len()], *seg,
                "offset {} doesn't match segment {:?}", off, seg);
        }
    }

    // ── Line comments ────────────────────────────────────────────────────────

    #[test]
    fn line_comment_semicolon_ignored() {
        let sql = "SELECT 1 -- end; here\n; SELECT 2";
        let r = stmts(sql);
        assert_eq!(r.len(), 2);
        assert!(r[0].starts_with("SELECT 1"));
        assert_eq!(r[1], "SELECT 2");
    }

    #[test]
    fn line_comment_at_end_no_split() {
        let sql = "SELECT 1 -- no semicolon here";
        assert_eq!(stmts(sql), vec!["SELECT 1 -- no semicolon here"]);
    }

    #[test]
    fn line_comment_only() {
        let sql = "-- just a comment";
        assert_eq!(stmts(sql), vec!["-- just a comment"]);
    }

    #[test]
    fn multiple_line_comments() {
        let sql = "-- comment\nSELECT 1; -- another\nSELECT 2";
        let r = stmts(sql);
        assert_eq!(r.len(), 2);
    }

    // ── Block comments ───────────────────────────────────────────────────────

    #[test]
    fn block_comment_semicolon_ignored() {
        let sql = "SELECT /* semi; colon */ 1";
        assert_eq!(stmts(sql), vec!["SELECT /* semi; colon */ 1"]);
    }

    #[test]
    fn block_comment_before_split() {
        let sql = "SELECT /* comment */ 1; SELECT 2";
        assert_eq!(stmts(sql), vec!["SELECT /* comment */ 1", "SELECT 2"]);
    }

    // ── Multi-line SQL ───────────────────────────────────────────────────────

    #[test]
    fn multiline_statement() {
        let sql = "SELECT\n  id,\n  name\nFROM users\nWHERE id = 1;";
        let r = stmts(sql);
        assert_eq!(r.len(), 1);
        assert!(r[0].starts_with("SELECT\n"));
    }

    #[test]
    fn multiline_two_statements() {
        let sql = "SELECT 1\nFROM dual;\n\nSELECT 2\nFROM dual";
        let r = stmts(sql);
        assert_eq!(r.len(), 2);
    }

    // ── Realistic SQL ────────────────────────────────────────────────────────

    #[test]
    fn cte_with_escaped_string() {
        let sql = "WITH cte AS (SELECT id FROM users WHERE name = 'it''s ok') SELECT * FROM cte; SELECT 1";
        let r = stmts(sql);
        assert_eq!(r.len(), 2);
        assert!(r[0].starts_with("WITH cte AS"));
        assert_eq!(r[1], "SELECT 1");
    }

    #[test]
    fn insert_then_select() {
        let sql = "INSERT INTO t (a, b) VALUES ('hello', 42); SELECT * FROM t";
        let r = stmts(sql);
        assert_eq!(r.len(), 2);
        assert!(r[0].starts_with("INSERT"));
        assert!(r[1].starts_with("SELECT"));
    }

    #[test]
    fn create_function_dollar_quoted() {
        let sql = r#"CREATE FUNCTION foo() RETURNS void AS $$
BEGIN
  RAISE NOTICE 'hello; world';
END;
$$ LANGUAGE plpgsql; SELECT 1"#;
        let r = stmts(sql);
        assert_eq!(r.len(), 2);
        assert!(r[0].starts_with("CREATE FUNCTION"));
        assert_eq!(r[1], "SELECT 1");
    }

    #[test]
    fn insert_with_semicolon_in_value() {
        let sql = "INSERT INTO t (msg) VALUES ('semi;colon'); SELECT 1";
        let r = stmts(sql);
        assert_eq!(r.len(), 2);
        assert!(r[0].starts_with("INSERT"));
        assert_eq!(r[1], "SELECT 1");
    }

    #[test]
    fn offset_after_cte() {
        let sql = "WITH cte AS (SELECT 1); SELECT 2";
        let r = split_statements(sql);
        assert_eq!(r.len(), 2);
        for (off, seg) in &r {
            assert_eq!(&sql[*off..*off + seg.len()], *seg);
        }
    }

    #[test]
    fn complex_multi_statement() {
        let sql = concat!(
            "CREATE TABLE users (id INT, name TEXT);\n",
            "INSERT INTO users VALUES (1, 'Alice');\n",
            "INSERT INTO users VALUES (2, 'Bob''s place');\n",
            "SELECT * FROM users WHERE name = 'Alice';"
        );
        let r = stmts(sql);
        assert_eq!(r.len(), 4);
    }

    // ── Double-quoted identifier with "" escape ──────────────────────────────

    #[test]
    fn double_quoted_identifier_escaped_double_quote() {
        // "col""name" is a single identifier with an embedded double-quote
        let sql = r#"SELECT "col""name" FROM t"#;
        assert_eq!(stmts(sql).len(), 1);
    }

    // ── Dollar-quote tag cannot start with a digit ───────────────────────────

    #[test]
    fn dollar_tag_starting_with_digit_not_dollar_quoted() {
        // $1abc$ is not a valid dollar-quote tag (starts with digit),
        // so the semicolon DOES split the input.
        let sql = r#"$1abc$hello;world$1abc$"#;
        let r = stmts(sql);
        assert_eq!(r.len(), 2, "digit-leading dollar tag should not be treated as dollar-quoted");
    }
}
