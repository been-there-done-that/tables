// sql-scope: multi-dialect SQL scope resolver
pub mod dialect;
pub mod error;
pub mod ir;
pub mod parser;
pub mod schema;

pub use dialect::Dialect;
pub use error::ScopeError;
pub use ir::ParsedStatement;
pub use parser::split_statements;
pub use schema::{ForeignKey, SchemaSnapshot, SqlType};
