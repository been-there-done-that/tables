//! SQLite utility functions
//!
//! Shared utilities for SQLite, including error formatting and value conversion.

use rusqlite::Error;
use serde_json::Value;

/// Format a SQLite error with available details
/// SQLite errors don't have hints/positions like PostgreSQL, but we can provide error codes and context
pub fn format_sqlite_error(err: &Error) -> String {
    match err {
        // Database errors with SQLite error codes
        Error::SqliteFailure(sqlite_err, msg) => {
            let mut parts = Vec::new();
            
            // SQLite error code (e.g., SQLITE_CONSTRAINT, SQLITE_ERROR)
            let code = sqlite_err.code as i32;
            let code_name = match code {
                1 => "SQLITE_ERROR",
                2 => "SQLITE_INTERNAL",
                3 => "SQLITE_PERM",
                4 => "SQLITE_ABORT",
                5 => "SQLITE_BUSY",
                6 => "SQLITE_LOCKED",
                7 => "SQLITE_NOMEM",
                8 => "SQLITE_READONLY",
                9 => "SQLITE_INTERRUPT",
                10 => "SQLITE_IOERR",
                11 => "SQLITE_CORRUPT",
                12 => "SQLITE_NOTFOUND",
                13 => "SQLITE_FULL",
                14 => "SQLITE_CANTOPEN",
                15 => "SQLITE_PROTOCOL",
                16 => "SQLITE_EMPTY",
                17 => "SQLITE_SCHEMA",
                18 => "SQLITE_TOOBIG",
                19 => "SQLITE_CONSTRAINT",
                20 => "SQLITE_MISMATCH",
                21 => "SQLITE_MISUSE",
                _ => "SQLITE_UNKNOWN",
            };
            
            if let Some(message) = msg {
                parts.push(format!("[{}] {}", code_name, message));
            } else {
                parts.push(format!("[{}] Error code {}", code_name, code));
            }
            
            // Extended error code provides more detail
            if sqlite_err.extended_code != code {
                parts.push(format!("Extended Code: {}", sqlite_err.extended_code));
            }
            
            parts.join("\n")
        }
        
        // Query-related errors
        Error::QueryReturnedNoRows => {
            "Query returned no rows (expected at least one)".to_string()
        }
        
        // Type conversion errors
        Error::InvalidColumnType(col_idx, name, actual_type) => {
            format!("Invalid column type at index {}: expected {}, got {:?}", col_idx, name, actual_type)
        }
        
        // Other error types - use default formatting
        _ => format!("SQLite Error: {}", err),
    }
}

/// Convert a SQLite row value to JSON
pub fn sqlite_value_to_json(row: &rusqlite::Row, idx: usize) -> Value {
    // Try different types in order of likelihood
    if let Ok(v) = row.get::<_, Option<i64>>(idx) {
        if let Some(n) = v {
            return Value::Number(n.into());
        }
        return Value::Null;
    }
    if let Ok(v) = row.get::<_, Option<f64>>(idx) {
        if let Some(n) = v {
            return serde_json::Number::from_f64(n)
                .map(Value::Number)
                .unwrap_or(Value::Null);
        }
        return Value::Null;
    }
    if let Ok(v) = row.get::<_, Option<String>>(idx) {
        if let Some(s) = v {
            // Try to parse as JSON if it looks like it
            let trimmed = s.trim();
            if (trimmed.starts_with('{') && trimmed.ends_with('}')) || (trimmed.starts_with('[') && trimmed.ends_with(']')) {
                if let Ok(json) = serde_json::from_str::<Value>(&s) {
                    return json;
                }
            }
            return Value::String(s);
        }
        return Value::Null;
    }
    if let Ok(v) = row.get::<_, Option<bool>>(idx) {
        if let Some(b) = v {
            return Value::Bool(b);
        }
        return Value::Null;
    }
    if let Ok(v) = row.get::<_, Option<Vec<u8>>>(idx) {
        if let Some(b) = v {
            return Value::String(format!("[{} bytes]", b.len()));
        }
        return Value::Null;
    }
    Value::Null
}
