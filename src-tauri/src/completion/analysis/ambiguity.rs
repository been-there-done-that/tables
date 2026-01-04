//! Column ambiguity detection.
//!
//! Detects when a column reference is ambiguous (exists in multiple tables)
//! and provides resolution state for UI warnings/errors.

use std::collections::HashSet;

/// Tracks column resolution with source information.
#[derive(Debug, Clone)]
pub struct ColumnResolution {
    /// The column name
    pub column: String,
    /// All tables/aliases where this column exists
    pub sources: Vec<TableSource>,
}

/// A table source for a column.
#[derive(Debug, Clone)]
pub struct TableSource {
    /// Table name or alias
    pub name: String,
    /// Schema if known
    pub schema: Option<String>,
    /// Is this a derived table (from subquery)?
    pub is_derived: bool,
}

/// The ambiguity state of a column reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AmbiguityState {
    /// Single source - unambiguous
    Resolved,
    /// Multiple base tables - ambiguous but legal SQL
    /// (Postgres will pick one, but it's risky)
    AmbiguousLegal,
    /// Multiple derived tables - invalid ambiguity
    /// (Must be qualified)
    AmbiguousInvalid,
}

impl ColumnResolution {
    /// Create a new column resolution.
    pub fn new(column: &str) -> Self {
        Self {
            column: column.to_string(),
            sources: Vec::new(),
        }
    }
    
    /// Add a source table for this column.
    pub fn add_source(&mut self, name: &str, schema: Option<&str>, is_derived: bool) {
        self.sources.push(TableSource {
            name: name.to_string(),
            schema: schema.map(|s| s.to_string()),
            is_derived,
        });
    }
    
    /// Determine the ambiguity state.
    pub fn state(&self) -> AmbiguityState {
        match self.sources.len() {
            0 | 1 => AmbiguityState::Resolved,
            _ => {
                // If any source is derived, it's invalid ambiguity
                let has_derived = self.sources.iter().any(|s| s.is_derived);
                if has_derived {
                    AmbiguityState::AmbiguousInvalid
                } else {
                    AmbiguityState::AmbiguousLegal
                }
            }
        }
    }
    
    /// Check if the column is ambiguous.
    pub fn is_ambiguous(&self) -> bool {
        self.sources.len() > 1
    }
    
    /// Get a human-readable description of the ambiguity.
    pub fn description(&self) -> Option<String> {
        if !self.is_ambiguous() {
            return None;
        }
        
        let names: Vec<&str> = self.sources.iter()
            .map(|s| s.name.as_str())
            .collect();
        
        Some(format!(
            "Column '{}' exists in: {}. Qualify as {}.{}",
            self.column,
            names.join(", "),
            names[0],
            self.column
        ))
    }
}

/// Check if a column name exists in multiple visible tables.
/// Returns a ColumnResolution with all matching sources.
pub fn check_column_ambiguity(
    column_name: &str,
    visible_tables: &[(String, Option<String>, bool)], // (name, schema, is_derived)
    columns_by_table: &std::collections::HashMap<String, HashSet<String>>,
) -> ColumnResolution {
    let mut resolution = ColumnResolution::new(column_name);
    let col_lower = column_name.to_lowercase();
    
    for (table_name, schema, is_derived) in visible_tables {
        if let Some(columns) = columns_by_table.get(&table_name.to_lowercase()) {
            if columns.contains(&col_lower) {
                resolution.add_source(table_name, schema.as_deref(), *is_derived);
            }
        }
    }
    
    resolution
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_unambiguous_column() {
        let mut res = ColumnResolution::new("email");
        res.add_source("users", Some("public"), false);
        
        assert_eq!(res.state(), AmbiguityState::Resolved);
        assert!(!res.is_ambiguous());
    }

    #[test]
    fn test_ambiguous_base_tables() {
        let mut res = ColumnResolution::new("id");
        res.add_source("users", Some("public"), false);
        res.add_source("orders", Some("public"), false);
        
        assert_eq!(res.state(), AmbiguityState::AmbiguousLegal);
        assert!(res.is_ambiguous());
    }

    #[test]
    fn test_ambiguous_with_derived() {
        let mut res = ColumnResolution::new("id");
        res.add_source("x", None, true); // derived table
        res.add_source("y", None, true); // derived table
        
        assert_eq!(res.state(), AmbiguityState::AmbiguousInvalid);
    }
}
