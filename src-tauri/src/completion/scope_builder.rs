/// Build a scope tree using progressively more aggressive SQL sanitization so that
/// incomplete SQL at the cursor doesn't block alias resolution.
///
/// pg_query fails on common in-progress patterns — handled by four passes:
///
///   Pass 1 — original text (succeeds for complete, valid SQL)
///   Pass 2 — replace partial token at cursor with "x" identifiers
///             e.g. "SELECT u., o.name FROM ..." → "SELECT xx, o.name FROM ..."
///             Fixes: trailing-dot mid-SQL, partial identifier at end
///   Pass 3 — blank from the NEAREST (latest) incomplete clause keyword to cursor
///             e.g. "...WHERE t.  " → "...             "
///             Fixes: empty WHERE/HAVING/AND/OR clause after stripping token
///   Pass 4 — insert a synthetic "x" identifier at cursor into s2 (not s3)
///             e.g. "JOIN orders o ON " → "JOIN orders o ON x"
///             Fixes: JOIN ON with no expression yet (s3 strips ON, losing aliases)
///
/// All passes preserve byte offsets before the cursor so that
/// `visible_at(cursor_offset)` resolves correctly. Byte offsets at/after the
/// cursor may shift by 1 in pass 4 (one inserted char), but since we only ask
/// for `visible_at(cursor)`, scopes whose ranges contain cursor are unaffected.
///
/// This is the **same logic the real Tauri command uses**.  Tests must call
/// `build_scope_tree` (not a custom x-placeholder) so they exercise the same
/// code path and catch bugs like `cursor == sql.len()` falling outside scope ranges.
pub fn build_scope_tree(
    text: &str,
    cursor_offset: usize,
    dialect: sql_scope::Dialect,
    schema: &dyn sql_scope::schema::SchemaSnapshot,
) -> sql_scope::ScopeTree {
    let clamped = cursor_offset.min(text.len());

    // Pass 1: original text
    if let Ok(tree) = sql_scope::resolve(text, dialect, schema) {
        return tree;
    }

    // Pass 2: replace the partial token at cursor with a valid placeholder identifier.
    // Using "x" chars (not spaces) so that `u.` → `xx`, keeping the SQL syntactically
    // valid even when cursor is mid-SQL (e.g. "SELECT u.| , o.name FROM ...").
    let s2 = {
        let before = &text[..clamped];
        let token_start = before
            .rfind(|c: char| !c.is_alphanumeric() && c != '_' && c != '.')
            .map(|i| i + 1)
            .unwrap_or(0);
        let mut s = text.to_string();
        if token_start < clamped {
            s.replace_range(token_start..clamped, &"x".repeat(clamped - token_start));
        }
        s
    };
    if let Ok(tree) = sql_scope::resolve(&s2, dialect, schema) {
        return tree;
    }

    // Pass 3: strip from the NEAREST (latest) incomplete clause keyword before cursor.
    // Using max (not min) so we only blank the incomplete tail of the nearest clause,
    // not everything back to an earlier keyword like JOIN.
    let s3 = {
        let before_upper = s2[..clamped].to_uppercase();
        let clause_kws = [" WHERE", " HAVING", " ON ", " AND ", " OR ", " JOIN"];
        let mut strip_from: Option<usize> = None;
        for kw in &clause_kws {
            if let Some(pos) = before_upper.rfind(kw) {
                strip_from = Some(strip_from.map_or(pos, |prev| prev.max(pos)));
            }
        }
        let mut s = s2.clone();
        if let Some(from) = strip_from {
            if from < clamped {
                s.replace_range(from..clamped, &" ".repeat(clamped - from));
            }
        }
        s
    };
    if let Ok(tree) = sql_scope::resolve(&s3, dialect, schema) {
        return tree;
    }

    // Pass 4: insert a synthetic placeholder identifier at cursor (into s2, not s3).
    // Handles cases like "JOIN t2 ON " where the clause keyword needs a non-empty
    // expression. s2 still has the ON clause intact; s3 stripped it, losing aliases.
    // visible_at(cursor_offset) remains correct because the placeholder is inserted
    // AT cursor, so all scopes starting before cursor are unaffected.
    let s4 = {
        let mut s = s2.clone();
        s.insert_str(clamped, "x");
        s
    };
    if let Ok(tree) = sql_scope::resolve(&s4, dialect, schema) {
        return tree;
    }

    sql_scope::ScopeTree::new()
}
