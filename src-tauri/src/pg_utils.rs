//! PostgreSQL value conversion utilities
//!
//! Shared utilities for converting PostgreSQL values to JSON.
//! Used by both the adapter and query commands.

use serde_json;

/// Format a PostgreSQL error with all available details (code, hint, position, etc.)
/// This provides detailed error information matching DataGrip/psql output
pub fn format_postgres_error(err: &tokio_postgres::Error) -> String {
    // Check if this is a database error with rich metadata
    if let Some(db_err) = err.as_db_error() {
        let mut parts = Vec::new();
        
        // Error code (SQLSTATE) + main message
        let code = db_err.code().code();
        parts.push(format!("[{}] {}", code, db_err.message()));
        
        // Hint (if available) - provides actionable guidance
        if let Some(hint) = db_err.hint() {
            parts.push(format!("Hint: {}", hint));
        }
        
        // Detail (if available) - additional context
        if let Some(detail) = db_err.detail() {
            parts.push(format!("Detail: {}", detail));
        }
        
        // Position (if available) - character position in query
        if let Some(position) = db_err.position() {
            use tokio_postgres::error::ErrorPosition;
            let pos_str = match position {
                ErrorPosition::Original(p) => format!("Position: {}", p),
                ErrorPosition::Internal { position, query } => {
                    format!("Internal Position: {} in query: {}", position, query)
                }
            };
            parts.push(pos_str);
        }
        
        // Join all parts with newlines for readability
        parts.join("\n")
    } else {
        // Not a DB error (connection issue, protocol error, etc.)
        format!("Connection/Protocol Error: {}", err)
    }
}


/// Convert a PostgreSQL row value to JSON based on the column type.
///
/// This handles special types like UUID, timestamps, arrays, etc. that
/// require native Rust types for proper deserialization.
pub fn pg_value_to_json(
    row: &tokio_postgres::Row,
    idx: usize,
    col: &tokio_postgres::Column,
) -> serde_json::Value {
    use tokio_postgres::types::Type;

    match *col.type_() {
        Type::BOOL => {
            if let Ok(Some(v)) = row.try_get::<_, Option<bool>>(idx) {
                serde_json::Value::Bool(v)
            } else {
                serde_json::Value::Null
            }
        }
        Type::INT2 => {
            if let Ok(Some(v)) = row.try_get::<_, Option<i16>>(idx) {
                serde_json::Value::Number(v.into())
            } else {
                serde_json::Value::Null
            }
        }
        Type::INT4 => {
            if let Ok(Some(v)) = row.try_get::<_, Option<i32>>(idx) {
                serde_json::Value::Number(v.into())
            } else {
                serde_json::Value::Null
            }
        }
        Type::INT8 => {
            if let Ok(Some(v)) = row.try_get::<_, Option<i64>>(idx) {
                serde_json::Value::Number(v.into())
            } else {
                serde_json::Value::Null
            }
        }
        Type::FLOAT4 => {
            if let Ok(Some(v)) = row.try_get::<_, Option<f32>>(idx) {
                serde_json::Number::from_f64(v as f64)
                    .map(serde_json::Value::Number)
                    .unwrap_or(serde_json::Value::Null)
            } else {
                serde_json::Value::Null
            }
        }
        Type::FLOAT8 => {
            if let Ok(Some(v)) = row.try_get::<_, Option<f64>>(idx) {
                serde_json::Number::from_f64(v)
                    .map(serde_json::Value::Number)
                    .unwrap_or(serde_json::Value::Null)
            } else {
                serde_json::Value::Null
            }
        }
        Type::TEXT | Type::VARCHAR | Type::BPCHAR | Type::NAME | Type::CHAR => {
            if let Ok(Some(v)) = row.try_get::<_, Option<String>>(idx) {
                serde_json::Value::String(v)
            } else {
                serde_json::Value::Null
            }
        }
        Type::JSON | Type::JSONB => {
            // Try native serde_json::Value first (faster), fall back to string parsing
            if let Ok(Some(v)) = row.try_get::<_, Option<serde_json::Value>>(idx) {
                v
            } else if let Ok(Some(v)) = row.try_get::<_, Option<String>>(idx) {
                serde_json::from_str(&v).unwrap_or(serde_json::Value::String(v))
            } else {
                serde_json::Value::Null
            }
        }
        // Date types - format using chrono native types
        Type::DATE => {
            if let Ok(Some(v)) = row.try_get::<_, Option<chrono::NaiveDate>>(idx) {
                serde_json::Value::String(v.format("%Y-%m-%d").to_string())
            } else {
                serde_json::Value::Null
            }
        }
        Type::TIME => {
            if let Ok(Some(v)) = row.try_get::<_, Option<chrono::NaiveTime>>(idx) {
                serde_json::Value::String(v.format("%H:%M:%S%.6f").to_string())
            } else {
                serde_json::Value::Null
            }
        }
        Type::TIMETZ => {
            // TIMETZ doesn't have direct chrono support, fall back to string
            if let Ok(Some(v)) = row.try_get::<_, Option<String>>(idx) {
                serde_json::Value::String(v)
            } else {
                serde_json::Value::Null
            }
        }
        Type::TIMESTAMP => {
            if let Ok(Some(v)) = row.try_get::<_, Option<chrono::NaiveDateTime>>(idx) {
                serde_json::Value::String(v.format("%Y-%m-%d %H:%M:%S%.6f").to_string())
            } else {
                serde_json::Value::Null
            }
        }
        Type::TIMESTAMPTZ => {
            if let Ok(Some(v)) = row.try_get::<_, Option<chrono::DateTime<chrono::Utc>>>(idx) {
                // Add %:z to include timezone offset (e.g. +00:00)
                serde_json::Value::String(v.format("%Y-%m-%d %H:%M:%S%.6f %:z").to_string())
            } else {
                serde_json::Value::Null
            }
        }
        // UUID - format as string using native uuid type
        Type::UUID => {
            if let Ok(Some(v)) = row.try_get::<_, Option<uuid::Uuid>>(idx) {
                serde_json::Value::String(v.to_string())
            } else {
                serde_json::Value::Null
            }
        }
        // Numeric/Money - preserve as string to avoid precision loss
        Type::NUMERIC => {
            if let Ok(Some(v)) = row.try_get::<_, Option<String>>(idx) {
                serde_json::Value::String(v)
            } else {
                serde_json::Value::Null
            }
        }
        Type::MONEY => {
            if let Ok(Some(v)) = row.try_get::<_, Option<String>>(idx) {
                serde_json::Value::String(v)
            } else {
                serde_json::Value::Null
            }
        }
        // OID - display as integer
        Type::OID => {
            if let Ok(Some(v)) = row.try_get::<_, Option<u32>>(idx) {
                serde_json::Value::Number(v.into())
            } else {
                serde_json::Value::Null
            }
        }
        // Bytea - encode as hex for safe display
        Type::BYTEA => {
            if let Ok(Some(v)) = row.try_get::<_, Option<Vec<u8>>>(idx) {
                let hex: String = v.iter().map(|b| format!("{:02x}", b)).collect();
                serde_json::Value::String(format!("\\x{}", hex))
            } else {
                serde_json::Value::Null
            }
        }
        // Array types - convert to JSON arrays
        ref t if matches!(t.kind(), tokio_postgres::types::Kind::Array(_)) => {
            pg_array_to_json(row, idx, col.type_())
        }
        _ => {
            // Default to string representation
            if let Ok(Some(v)) = row.try_get::<_, Option<String>>(idx) {
                serde_json::Value::String(v)
            } else {
                serde_json::Value::Null
            }
        }
    }
}

/// Convert PostgreSQL array types to JSON arrays
pub fn pg_array_to_json(
    row: &tokio_postgres::Row,
    idx: usize,
    col_type: &tokio_postgres::types::Type,
) -> serde_json::Value {
    use tokio_postgres::types::{Kind, Type};

    // Get the element type from the array type
    if let Kind::Array(element_type) = col_type.kind() {
        // Integer arrays
        if *element_type == Type::INT2 {
            if let Ok(Some(v)) = row.try_get::<_, Option<Vec<i16>>>(idx) {
                return serde_json::Value::Array(
                    v.into_iter()
                        .map(|i| serde_json::Value::Number(i.into()))
                        .collect(),
                );
            }
        } else if *element_type == Type::INT4 {
            if let Ok(Some(v)) = row.try_get::<_, Option<Vec<i32>>>(idx) {
                return serde_json::Value::Array(
                    v.into_iter()
                        .map(|i| serde_json::Value::Number(i.into()))
                        .collect(),
                );
            }
        } else if *element_type == Type::INT8 {
            if let Ok(Some(v)) = row.try_get::<_, Option<Vec<i64>>>(idx) {
                return serde_json::Value::Array(
                    v.into_iter()
                        .map(|i| serde_json::Value::Number(i.into()))
                        .collect(),
                );
            }
        // Float arrays
        } else if *element_type == Type::FLOAT4 {
            if let Ok(Some(v)) = row.try_get::<_, Option<Vec<f32>>>(idx) {
                return serde_json::Value::Array(
                    v.into_iter()
                        .map(|f| {
                            serde_json::Number::from_f64(f as f64)
                                .map(serde_json::Value::Number)
                                .unwrap_or(serde_json::Value::Null)
                        })
                        .collect(),
                );
            }
        } else if *element_type == Type::FLOAT8 {
            if let Ok(Some(v)) = row.try_get::<_, Option<Vec<f64>>>(idx) {
                return serde_json::Value::Array(
                    v.into_iter()
                        .map(|f| {
                            serde_json::Number::from_f64(f)
                                .map(serde_json::Value::Number)
                                .unwrap_or(serde_json::Value::Null)
                        })
                        .collect(),
                );
            }
        // Boolean arrays
        } else if *element_type == Type::BOOL {
            if let Ok(Some(v)) = row.try_get::<_, Option<Vec<bool>>>(idx) {
                return serde_json::Value::Array(
                    v.into_iter().map(serde_json::Value::Bool).collect(),
                );
            }
        // Text arrays
        } else if *element_type == Type::TEXT
            || *element_type == Type::VARCHAR
            || *element_type == Type::BPCHAR
            || *element_type == Type::NAME
        {
            if let Ok(Some(v)) = row.try_get::<_, Option<Vec<String>>>(idx) {
                return serde_json::Value::Array(
                    v.into_iter().map(serde_json::Value::String).collect(),
                );
            }
        // UUID arrays
        } else if *element_type == Type::UUID {
            if let Ok(Some(v)) = row.try_get::<_, Option<Vec<uuid::Uuid>>>(idx) {
                return serde_json::Value::Array(
                    v.into_iter()
                        .map(|u| serde_json::Value::String(u.to_string()))
                        .collect(),
                );
            }
        // Timestamp arrays
        } else if *element_type == Type::TIMESTAMP {
            if let Ok(Some(v)) = row.try_get::<_, Option<Vec<chrono::NaiveDateTime>>>(idx) {
                return serde_json::Value::Array(
                    v.into_iter()
                        .map(|t| {
                            serde_json::Value::String(
                                t.format("%Y-%m-%d %H:%M:%S%.6f").to_string(),
                            )
                        })
                        .collect(),
                );
            }
        } else if *element_type == Type::TIMESTAMPTZ {
            if let Ok(Some(v)) =
                row.try_get::<_, Option<Vec<chrono::DateTime<chrono::Utc>>>>(idx)
            {
                return serde_json::Value::Array(
                    v.into_iter()
                        .map(|t| {
                            serde_json::Value::String(
                                t.format("%Y-%m-%d %H:%M:%S%.6f %:z").to_string(),
                            )
                        })
                        .collect(),
                );
            }
        }
        
        // Fallback for unrecognized array element types: try to get as Vec<String>
        // This handles custom types, domains, etc.
        if let Ok(Some(v)) = row.try_get::<_, Option<Vec<String>>>(idx) {
            return serde_json::Value::Array(
                v.into_iter().map(serde_json::Value::String).collect(),
            );
        }
    }

    // Final fallback: try to get as string and parse PostgreSQL array format {a,b,c}
    if let Ok(Some(v)) = row.try_get::<_, Option<String>>(idx) {
        // Check if it looks like a PostgreSQL array format
        let trimmed = v.trim();
        if trimmed.starts_with('{') && trimmed.ends_with('}') {
            // Parse PostgreSQL array format: {val1,val2,val3}
            let inner = &trimmed[1..trimmed.len()-1];
            if inner.is_empty() {
                return serde_json::Value::Array(vec![]);
            }
            // Split by comma (simple parsing - doesn't handle quoted strings with commas)
            let elements: Vec<serde_json::Value> = inner
                .split(',')
                .map(|s| {
                    let s = s.trim();
                    // Remove surrounding quotes if present
                    if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
                        serde_json::Value::String(s[1..s.len()-1].to_string())
                    } else {
                        serde_json::Value::String(s.to_string())
                    }
                })
                .collect();
            return serde_json::Value::Array(elements);
        }
        // Not an array format, return as plain string
        serde_json::Value::String(v)
    } else {
        serde_json::Value::Null
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: Full testing requires a live PostgreSQL connection
    // These are placeholder tests for the module structure
    #[test]
    fn test_module_exists() {
        // Module compiles successfully
        assert!(true);
    }
}
