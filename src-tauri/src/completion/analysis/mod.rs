//! Semantic analysis module.
//!
//! Builds a scope graph from the AST to support:
//! - Alias resolution (table aliases → real tables)
//! - Scope walking (subqueries, CTEs)
//! - Symbol lookup

pub mod scope;
pub mod builder;

pub use scope::{SemanticModel, Scope, Symbol, SymbolKind};
pub use builder::build_semantic_model;
