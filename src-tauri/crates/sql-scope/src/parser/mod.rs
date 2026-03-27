pub mod splitter;
pub mod postgres;
pub mod sqlite;
pub mod mysql;

pub use splitter::split_statements;
