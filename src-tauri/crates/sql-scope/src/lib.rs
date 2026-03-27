// sql-scope: multi-dialect SQL scope resolver
pub mod dialect;
pub mod error;
pub mod schema;

pub use dialect::Dialect;
pub use error::ScopeError;
pub use schema::{ForeignKey, SchemaSnapshot, SqlType};
