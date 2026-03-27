use crate::schema::SchemaSnapshot;

/// Infer the most likely JOIN condition between two tables given their aliases.
/// Returns `(condition_sql, confidence: 0-100)` or None if no relationship found.
/// Tier 1: FK relationship (90)
/// Tier 2: naming heuristic (70) — `{right_table_singular}_id`, `{right_table}_id` in left cols
/// Tier 3: shared column names ending in `_id` (40)
pub fn infer_join_condition(
    left_alias: &str,
    left_table: &str,
    right_alias: &str,
    right_table: &str,
    schema: &dyn SchemaSnapshot,
) -> Option<(String, u32)> {
    // Tier 1: FK from left → right
    for fk in schema.foreign_keys(None, left_table) {
        if fk.to_table.to_lowercase() == right_table.to_lowercase() {
            let cond = format!("{}.{} = {}.{}", left_alias, fk.from_column, right_alias, fk.to_column);
            return Some((cond, 90));
        }
    }
    // Tier 1: FK from right → left
    for fk in schema.foreign_keys(None, right_table) {
        if fk.to_table.to_lowercase() == left_table.to_lowercase() {
            let cond = format!("{}.{} = {}.{}", right_alias, fk.from_column, left_alias, fk.to_column);
            return Some((cond, 90));
        }
    }

    // Tier 2: naming heuristic
    let right_singular = right_table.trim_end_matches('s');
    let patterns: &[String] = &[
        format!("{}_id", right_singular),
        format!("{}_id", right_table),
        format!("fk_{}", right_singular),
        format!("{}id", right_singular),
    ];
    if let Some(left_cols) = schema.table_columns(None, left_table) {
        for pat in patterns {
            if left_cols.iter().any(|c| c.to_lowercase() == *pat) {
                if let Some(right_cols) = schema.table_columns(None, right_table) {
                    if right_cols.iter().any(|c| c.to_lowercase() == "id") {
                        let cond = format!("{}.{} = {}.id", left_alias, pat, right_alias);
                        return Some((cond, 70));
                    }
                }
            }
        }
    }

    // Tier 3: shared column names ending in _id
    if let (Some(lcols), Some(rcols)) = (
        schema.table_columns(None, left_table),
        schema.table_columns(None, right_table),
    ) {
        for lcol in &lcols {
            if lcol.ends_with("_id") && rcols.iter().any(|rc| rc == lcol) {
                let cond = format!("{}.{} = {}.{}", left_alias, lcol, right_alias, lcol);
                return Some((cond, 40));
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{ForeignKey, SqlType};
    use std::collections::HashMap;

    struct MockS {
        tables: HashMap<String, Vec<String>>,
        fks: HashMap<String, Vec<ForeignKey>>,
    }
    impl crate::schema::SchemaSnapshot for MockS {
        fn table_exists(&self, _: Option<&str>, t: &str) -> bool { self.tables.contains_key(t) }
        fn table_columns(&self, _: Option<&str>, t: &str) -> Option<Vec<String>> { self.tables.get(t).cloned() }
        fn column_type(&self, _: Option<&str>, _: &str, _: &str) -> Option<SqlType> { None }
        fn foreign_keys(&self, _: Option<&str>, t: &str) -> Vec<ForeignKey> { self.fks.get(t).cloned().unwrap_or_default() }
        fn default_schema(&self) -> Option<&str> { Some("public") }
    }

    fn mock(tables: &[(&str, &[&str])], fks: &[(&str, &str, &str, &str)]) -> MockS {
        MockS {
            tables: tables.iter().map(|(t, cols)| {
                (t.to_string(), cols.iter().map(|c| c.to_string()).collect())
            }).collect(),
            fks: {
                let mut m: HashMap<String, Vec<ForeignKey>> = HashMap::new();
                for (from_t, from_c, to_t, to_c) in fks {
                    m.entry(from_t.to_string()).or_default().push(ForeignKey {
                        from_column: from_c.to_string(),
                        to_table: to_t.to_string(),
                        to_column: to_c.to_string(),
                    });
                }
                m
            },
        }
    }

    #[test]
    fn uses_fk_first() {
        let s = mock(
            &[("orders", &["id", "user_id"]), ("users", &["id"])],
            &[("orders", "user_id", "users", "id")],
        );
        let (cond, conf) = infer_join_condition("o", "orders", "u", "users", &s).unwrap();
        assert!(conf >= 90, "FK should give confidence >= 90, got {}", conf);
        assert!(cond.contains("user_id"), "FK condition should reference user_id");
        assert!(cond.contains("id"), "FK condition should reference id");
    }

    #[test]
    fn fk_right_to_left() {
        // FK from orders → users (right table's FK)
        let s = mock(
            &[("users", &["id"]), ("orders", &["id", "user_id"])],
            &[("orders", "user_id", "users", "id")],
        );
        // reversed: left=users, right=orders
        let (cond, conf) = infer_join_condition("u", "users", "o", "orders", &s).unwrap();
        assert!(conf >= 90);
        assert!(cond.contains("user_id"));
    }

    #[test]
    fn naming_heuristic_fallback() {
        let s = mock(
            &[("orders", &["id", "user_id"]), ("users", &["id"])],
            &[], // no FK
        );
        let (cond, conf) = infer_join_condition("o", "orders", "u", "users", &s).unwrap();
        assert!(conf >= 60, "naming heuristic should give >= 60, got {}", conf);
        assert!(cond.contains("user_id"), "should reference user_id via naming heuristic");
    }

    #[test]
    fn shared_column_tier3_fallback() {
        let s = mock(
            &[("a", &["order_id", "x"]), ("b", &["order_id", "y"])],
            &[],
        );
        let (cond, conf) = infer_join_condition("ta", "a", "tb", "b", &s).unwrap();
        assert_eq!(conf, 40, "shared _id column should give conf=40");
        assert!(cond.contains("order_id"));
    }

    #[test]
    fn no_match_returns_none() {
        let s = mock(&[("foo", &["a"]), ("bar", &["b"])], &[]);
        assert!(infer_join_condition("f", "foo", "b", "bar", &s).is_none());
    }

    #[test]
    fn fk_beats_heuristic() {
        let s = mock(
            &[("orders", &["id", "user_id"]), ("users", &["id"])],
            &[("orders", "user_id", "users", "id")],
        );
        let (_, conf_fk) = infer_join_condition("o", "orders", "u", "users", &s).unwrap();
        let s2 = mock(
            &[("orders", &["id", "user_id"]), ("users", &["id"])],
            &[],
        );
        let (_, conf_heur) = infer_join_condition("o", "orders", "u", "users", &s2).unwrap();
        assert!(conf_fk > conf_heur, "FK ({}) should beat heuristic ({})", conf_fk, conf_heur);
    }
}
