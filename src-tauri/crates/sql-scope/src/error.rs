use thiserror::Error;

#[derive(Debug, Error)]
pub enum ScopeError {
    #[error("parse error: {0}")]
    Parse(String),
    #[error("unsupported construct: {0}")]
    Unsupported(String),
}
