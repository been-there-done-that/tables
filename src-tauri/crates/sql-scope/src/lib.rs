// sql-scope: multi-dialect SQL scope resolver
pub mod dialect;
pub mod diagnostics;
pub mod error;
pub mod ir;
pub mod join;
pub mod r#match;
pub mod parser;
pub mod schema;
pub mod scope;
pub mod types;

pub use dialect::Dialect;
pub use error::ScopeError;
pub use ir::ParsedStatement;
pub use join::infer_join_condition;
pub use r#match::match_score;
pub use parser::split_statements;
pub use schema::{ForeignKey, SchemaSnapshot, SqlType};
pub use scope::{
    ColumnRef, CteInfo, DiagSeverity, Scope, ScopeId, ScopeDiagnostic, ScopeTree, ScopeType,
    Source, VisibleSymbols,
};
pub use scope::traverse_scope;
pub use diagnostics::run_diagnostics;
pub use types::resolve_column_type;
