//! Semantic analysis module.
//!
//! Builds a scope graph from the AST to support:
//! - Alias resolution (table aliases → real tables)
//! - Scope walking (subqueries, CTEs)
//! - Symbol lookup
//! - Column ambiguity detection

pub mod scope;
pub mod builder;
pub mod ambiguity;

pub use scope::{SemanticModel, SymbolKind};
pub use builder::build_semantic_model;

