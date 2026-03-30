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
pub use ir::DangerousKind;
pub use parser::postgres::parse_postgres as parse_postgres_stmt;

/// Resolve scope for a single SQL statement.
/// Use `split_statements()` first for multi-statement input.
pub fn resolve(
    sql: &str,
    dialect: Dialect,
    schema: &dyn schema::SchemaSnapshot,
) -> Result<ScopeTree, error::ScopeError> {
    let stmt = match dialect {
        Dialect::Postgres => parser::postgres::parse_postgres(sql)
            .ok_or_else(|| error::ScopeError::Parse("PostgreSQL parse failed".into()))?,
        Dialect::Sqlite => parser::sqlite::parse_sqlite(sql)
            .ok_or_else(|| error::ScopeError::Parse("SQLite parse failed".into()))?,
        Dialect::Mysql => parser::mysql::parse_mysql(sql)
            .ok_or_else(|| error::ScopeError::Parse("MySQL parse failed".into()))?,
    };
    Ok(scope::resolver::traverse_scope(&stmt, schema))
}
