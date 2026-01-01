//! Schema graph module.
//!
//! The SchemaGraph represents database metadata:
//! - Tables and their columns
//! - Foreign key relationships
//! - Index information for ranking

pub mod graph;
pub mod loader;

pub use graph::{SchemaGraph, TableInfo, ColumnInfo, ForeignKey};
pub use loader::MockSchemaLoader;
