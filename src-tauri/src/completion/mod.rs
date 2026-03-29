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
pub mod scope_builder;

#[cfg(test)]
pub mod tests;

#[cfg(test)]
pub mod pg_test_helpers;

#[cfg(test)]
pub mod pg_integration_tests;

// Re-exports
