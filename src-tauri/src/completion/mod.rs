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
pub mod engine;
pub mod analysis;
pub mod schema;

#[cfg(test)]
pub mod tests;

// Re-exports
pub use document::DocumentState;
pub use context::{Context, CursorContext};
pub use engine::{CompletionEngine, CompletionItem};
pub use analysis::SemanticModel;
pub use schema::SchemaGraph;
