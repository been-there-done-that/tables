//! Schema graph using petgraph.
//!
//! Stores tables, columns, and FK relationships as a graph
//! to enable efficient join inference.

use std::collections::HashMap;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;

/// The schema graph for a database connection.
#[derive(Debug)]
pub struct SchemaGraph {
    /// Table name → TableInfo
    pub tables: HashMap<String, TableInfo>,
    /// Graph of FK relationships: edges point from FK table to PK table
    pub fk_graph: DiGraph<String, ForeignKey>,
    /// Table name → node index in fk_graph
    node_indices: HashMap<String, NodeIndex>,
}

/// Information about a table.
#[derive(Debug, Clone)]
pub struct TableInfo {
    pub name: String,
    pub schema: String,
    pub columns: Vec<ColumnInfo>,
}

/// Information about a column.
#[derive(Debug, Clone)]
pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
    pub is_primary_key: bool,
    pub is_indexed: bool,
    pub is_nullable: bool,
}

/// Foreign key relationship.
#[derive(Debug, Clone)]
pub struct ForeignKey {
    pub from_table: String,
    pub from_column: String,
    pub to_table: String,
    pub to_column: String,
}

impl SchemaGraph {
    pub fn new() -> Self {
        Self {
            tables: HashMap::new(),
            fk_graph: DiGraph::new(),
            node_indices: HashMap::new(),
        }
    }

    /// Add a table to the schema.
    pub fn add_table(&mut self, table: TableInfo) {
        let name = table.name.to_lowercase();
        
        // Add to graph if not present
        if !self.node_indices.contains_key(&name) {
            let idx = self.fk_graph.add_node(name.clone());
            self.node_indices.insert(name.clone(), idx);
        }
        
        self.tables.insert(name, table);
    }

    /// Add a foreign key relationship.
    pub fn add_foreign_key(&mut self, fk: ForeignKey) {
        let from_name = fk.from_table.to_lowercase();
        let to_name = fk.to_table.to_lowercase();
        
        // Ensure both tables have nodes
        if !self.node_indices.contains_key(&from_name) {
            let idx = self.fk_graph.add_node(from_name.clone());
            self.node_indices.insert(from_name.clone(), idx);
        }
        if !self.node_indices.contains_key(&to_name) {
            let idx = self.fk_graph.add_node(to_name.clone());
            self.node_indices.insert(to_name.clone(), idx);
        }
        
        let from_idx = self.node_indices[&from_name];
        let to_idx = self.node_indices[&to_name];
        
        self.fk_graph.add_edge(from_idx, to_idx, fk);
    }

    /// Get a table by name.
    pub fn get_table(&self, name: &str) -> Option<&TableInfo> {
        self.tables.get(&name.to_lowercase())
    }

    /// Check if a table exists.
    pub fn has_table(&self, name: &str) -> bool {
        self.tables.contains_key(&name.to_lowercase())
    }

    /// Get columns for a table.
    pub fn get_columns(&self, table_name: &str) -> Vec<&ColumnInfo> {
        self.tables
            .get(&table_name.to_lowercase())
            .map(|t| t.columns.iter().collect())
            .unwrap_or_default()
    }

    /// Find FK path between two tables.
    /// Returns the FK if a direct relationship exists.
    pub fn find_fk_path(&self, from_table: &str, to_table: &str) -> Option<&ForeignKey> {
        let from_name = from_table.to_lowercase();
        let to_name = to_table.to_lowercase();
        
        let from_idx = self.node_indices.get(&from_name)?;
        let to_idx = self.node_indices.get(&to_name)?;
        
        // Check direct edge from → to
        for edge in self.fk_graph.edges(*from_idx) {
            if edge.target() == *to_idx {
                return Some(edge.weight());
            }
        }
        
        // Check reverse edge to → from
        for edge in self.fk_graph.edges(*to_idx) {
            if edge.target() == *from_idx {
                return Some(edge.weight());
            }
        }
        
        None
    }

    /// Infer join condition using hybrid approach:
    /// 1. Check FK graph (Gold Standard - Score 100)
    /// 2. Check naming heuristics (Silver Standard - Score 70)
    /// 3. Check common column names (Bronze Standard - Score 30)
    pub fn infer_join_condition(
        &self, 
        left_table: &str, 
        right_table: &str,
        left_alias: Option<&str>,
        right_alias: Option<&str>
    ) -> Option<(String, u32)> {
        let left = left_table.to_lowercase();
        let right = right_table.to_lowercase();
        
        let l_name = left_alias.unwrap_or(&left);
        let r_name = right_alias.unwrap_or(&right);
        
        // Check 1: FK relationship (Gold Standard)
        if let Some(fk) = self.find_fk_path(&left, &right) {
            // Determine which side of the FK corresponds to which alias
            // FK: from_table -> to_table
            // If from_table == left, then left alias uses from_column
            
            let (l_col, r_col) = if fk.from_table.to_lowercase() == left {
                (&fk.from_column, &fk.to_column)
            } else {
                (&fk.to_column, &fk.from_column)
            };

            let condition = format!(
                "{}.{} = {}.{}",
                l_name, l_col,
                r_name, r_col
            );
            return Some((condition, 100));
        }

        // Check 2: Naming heuristics (Silver Standard)
        // Look for patterns like: table_id, tableid, fk_table
        let left_table_info = self.get_table(&left);
        let right_table_info = self.get_table(&right);
        
        if let (Some(left_info), Some(right_info)) = (left_table_info, right_table_info) {
            // Check right table for columns referencing left table
            for col in &right_info.columns {
                let col_lower = col.name.to_lowercase();
                let patterns = [
                    format!("{}_id", left.trim_end_matches('s')), // users → user_id
                    format!("{}_id", left), // users → users_id
                    format!("fk_{}", left.trim_end_matches('s')), // fk_user
                    format!("{}id", left.trim_end_matches('s')), // userid
                ];
                
                for pattern in &patterns {
                    if col_lower == *pattern {
                        // Find PK of left table
                        if let Some(pk) = left_info.columns.iter().find(|c| c.is_primary_key) {
                            let condition = format!(
                                "{}.{} = {}.{}",
                                l_name, pk.name,
                                r_name, col.name
                            );
                            return Some((condition, 70));
                        }
                    }
                }
            }
            
            // Check left table for columns referencing right table
            for col in &left_info.columns {
                let col_lower = col.name.to_lowercase();
                let patterns = [
                    format!("{}_id", right.trim_end_matches('s')),
                    format!("{}_id", right),
                    format!("fk_{}", right.trim_end_matches('s')),
                    format!("{}id", right.trim_end_matches('s')),
                ];
                
                for pattern in &patterns {
                    if col_lower == *pattern {
                        if let Some(pk) = right_info.columns.iter().find(|c| c.is_primary_key) {
                            let condition = format!(
                                "{}.{} = {}.{}",
                                l_name, col.name,
                                r_name, pk.name
                            );
                            return Some((condition, 70));
                        }
                    }
                }
            }

            // Check 3: Common column names (Bronze Standard)
            for left_col in &left_info.columns {
                for right_col in &right_info.columns {
                    if left_col.name.to_lowercase() == right_col.name.to_lowercase() 
                       && left_col.name.to_lowercase() != "id" // Don't match on just 'id'
                       && (left_col.name.to_lowercase().ends_with("_id") 
                           || left_col.name.to_lowercase().ends_with("_by")
                           || left_col.name.to_lowercase() == "created_by") 
                    {
                        let condition = format!(
                            "{}.{} = {}.{}",
                            l_name, left_col.name,
                            r_name, right_col.name
                        );
                        return Some((condition, 30));
                    }
                }
            }
        }

        None
    }

    /// Get all table names.
    pub fn table_names(&self) -> Vec<&str> {
        self.tables.keys().map(|s| s.as_str()).collect()
    }
}

impl Default for SchemaGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl ColumnInfo {
    pub fn new(name: &str, data_type: &str) -> Self {
        Self {
            name: name.to_string(),
            data_type: data_type.to_string(),
            is_primary_key: false,
            is_indexed: false,
            is_nullable: true,
        }
    }

    pub fn primary_key(mut self) -> Self {
        self.is_primary_key = true;
        self.is_indexed = true;
        self.is_nullable = false;
        self
    }

    pub fn indexed(mut self) -> Self {
        self.is_indexed = true;
        self
    }

    pub fn not_null(mut self) -> Self {
        self.is_nullable = false;
        self
    }
}

impl TableInfo {
    pub fn new(name: &str, schema: &str, columns: Vec<ColumnInfo>) -> Self {
        Self {
            name: name.to_string(),
            schema: schema.to_string(),
            columns,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_schema() -> SchemaGraph {
        let mut schema = SchemaGraph::new();
        
        // Users table
        schema.add_table(TableInfo::new("users", "public", vec![
            ColumnInfo::new("id", "integer").primary_key(),
            ColumnInfo::new("email", "varchar"),
            ColumnInfo::new("created_at", "timestamp"),
        ]));
        
        // Orders table with FK to users
        schema.add_table(TableInfo::new("orders", "public", vec![
            ColumnInfo::new("id", "integer").primary_key(),
            ColumnInfo::new("user_id", "integer").indexed(),
            ColumnInfo::new("amount", "decimal"),
            ColumnInfo::new("created_at", "timestamp"),
        ]));
        
        // Add FK relationship
        schema.add_foreign_key(ForeignKey {
            from_table: "orders".to_string(),
            from_column: "user_id".to_string(),
            to_table: "users".to_string(),
            to_column: "id".to_string(),
        });
        
        schema
    }

    #[test]
    fn test_fk_join_inference() {
        let schema = create_test_schema();
        
        // FK exists: should get Gold Standard (100)
        let result = schema.infer_join_condition("users", "orders", None, None);
        assert!(result.is_some());
        let (condition, score) = result.unwrap();
        assert_eq!(score, 100);
        assert!(condition.contains("user_id") && condition.contains("id"));
    }

    #[test]
    fn test_heuristic_join_inference() {
        let mut schema = SchemaGraph::new();
        
        // No FK, but naming convention exists
        schema.add_table(TableInfo::new("users", "public", vec![
            ColumnInfo::new("id", "integer").primary_key(),
        ]));
        
        schema.add_table(TableInfo::new("orders", "public", vec![
            ColumnInfo::new("id", "integer").primary_key(),
            ColumnInfo::new("user_id", "integer"),
        ]));
        // No FK added!
        
        let result = schema.infer_join_condition("users", "orders", None, None);
        assert!(result.is_some());
        let (condition, score) = result.unwrap();
        assert_eq!(score, 70); // Silver Standard
        assert!(condition.contains("user_id"));
    }

    #[test]
    fn test_column_ranking_by_index() {
        let schema = create_test_schema();
        let columns = schema.get_columns("orders");
        
        // user_id should be indexed
        let user_id = columns.iter().find(|c| c.name == "user_id").unwrap();
        assert!(user_id.is_indexed);
    }
}
