// SQL Completion Engine
//
// A production-grade SQL auto-completion engine with:
// - Tree-sitter incremental parsing
// - Semantic scope analysis
// - Schema-aware join inference
// - DataGrip-parity test coverage

pub mod parsing;
pub mod document;
pub mod context;
pub mod items;
pub mod schema;
pub mod ranges;
pub mod diagnostics;
pub mod engines;

#[cfg(test)]
pub mod tests;

// Re-exports
