pub mod symbol;
pub mod tree;
pub mod resolver;
pub mod cte;

pub use tree::{CteInfo, DiagSeverity, Scope, ScopeDiagnostic, ScopeTree, ScopeType};
pub use symbol::{ColumnRef, ScopeId, Source, VisibleSymbols};
pub use resolver::traverse_scope;
