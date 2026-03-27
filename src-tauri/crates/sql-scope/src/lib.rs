// sql-scope: multi-dialect SQL scope resolver
pub mod dialect;
pub mod diagnostics;
pub mod error;
pub mod ir;
pub mod parser;
pub mod schema;
pub mod scope;

pub use dialect::Dialect;
pub use error::ScopeError;
pub use ir::ParsedStatement;
pub use parser::split_statements;
pub use schema::{ForeignKey, SchemaSnapshot, SqlType};
pub use scope::{
    ColumnRef, CteInfo, DiagSeverity, Scope, ScopeId, ScopeDiagnostic, ScopeTree, ScopeType,
    Source, VisibleSymbols,
};
pub use scope::traverse_scope;
pub use diagnostics::run_diagnostics;
