use tauri::State;
use serde::{Deserialize, Serialize};
use log::{debug, info};
use crate::{DatabaseState, ConnectionManagerState, ConnectionManager};
use crate::commands::query_commands::{QuerySessionManager, get_or_create_postgres_client_pub, DBSession};
use std::time::Instant;

// ── Public types ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExplainResult {
    pub planning_ms: f64,
    pub execution_ms: f64,
    pub total_rows: u64,
    pub plan: PlanNode,
    pub issues: Vec<PlanIssue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanNode {
    pub node_type: String,
    pub relation_name: Option<String>,
    pub index_name: Option<String>,
    pub total_ms: f64,
    pub exclusive_ms: f64,
    pub pct_of_total: f64,
    pub planned_rows: u64,
    pub actual_rows: u64,
    pub loops: u64,
    pub buffers_hit: Option<u64>,
    pub buffers_read: Option<u64>,
    pub depth: u32,
    pub children: Vec<PlanNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanIssue {
    pub severity: String, // "danger" | "warning"
    pub kind: String,     // "seq_scan" | "row_estimate_mismatch"
    pub node_type: String,
    pub relation: Option<String>,
    pub message: String,
    pub suggestion: String,
}

// ── Tauri command ─────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn explain_query(
    connection_id: String,
    session_id: String,
    database: String,
    query: String,
    db_state: State<'_, DatabaseState>,
    conn_state: State<'_, ConnectionManagerState>,
    session_state: State<'_, QuerySessionManager>,
) -> Result<ExplainResult, String> {
    info!("[explain_query] Explaining query on database: {}", database);

    let engine = {
        let sessions = session_state.sessions.lock().await;
        match sessions.get(&(connection_id.clone(), session_id.clone())) {
            Some(DBSession::Postgres(_)) => "postgres".to_string(),
            Some(DBSession::Sqlite(_)) => "sqlite".to_string(),
            None => {
                let manager = ConnectionManager::from_state(&db_state, &conn_state);
                manager.get_connection(&connection_id)
                    .map(|(c, _)| c.engine)
                    .map_err(|e| e)?
            }
        }
    };

    match engine.as_str() {
        "postgres" | "postgresql" => {
            explain_postgres(&connection_id, &session_id, &database, &query, &db_state, &conn_state, &session_state).await
        }
        "sqlite" => {
            explain_sqlite(&query)
        }
        other => Err(format!("EXPLAIN not supported for engine: {}", other)),
    }
}

// ── PostgreSQL explain ────────────────────────────────────────────────────────

async fn explain_postgres(
    connection_id: &str,
    session_id: &str,
    database: &str,
    query: &str,
    db_state: &State<'_, DatabaseState>,
    conn_state: &State<'_, ConnectionManagerState>,
    session_state: &State<'_, QuerySessionManager>,
) -> Result<ExplainResult, String> {
    let manager = ConnectionManager::from_state(db_state, conn_state);
    let (connection, credentials) = manager.get_connection(connection_id)?;
    let config_val = serde_json::to_value(&connection).ok();

    let client = get_or_create_postgres_client_pub(
        session_state,
        connection_id,
        session_id,
        config_val.as_ref(),
        Some(&credentials),
        database,
    ).await?;

    let trimmed = query.trim().trim_end_matches(';');
    let explain_sql = format!("EXPLAIN (ANALYZE, BUFFERS, FORMAT JSON) {}", trimmed);

    debug!("[explain_query] Running: {}", explain_sql);

    let _start = Instant::now();
    let rows = client.query(&explain_sql, &[]).await
        .map_err(|e| crate::pg_utils::format_postgres_error(&e))?;

    if rows.is_empty() {
        return Err("EXPLAIN returned no output".to_string());
    }

    // The result is a single row with a single JSON column
    let plan_json: serde_json::Value = rows[0].get(0);

    parse_explain_json(&plan_json)
}

fn parse_explain_json(raw: &serde_json::Value) -> Result<ExplainResult, String> {
    // EXPLAIN (FORMAT JSON) returns an array with one element: [{Plan: {...}, Planning Time: ..., Execution Time: ...}]
    let entry = match raw {
        serde_json::Value::Array(arr) if !arr.is_empty() => &arr[0],
        _ => raw,
    };

    let planning_ms = entry.get("Planning Time")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

    let execution_ms = entry.get("Execution Time")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

    let plan_obj = entry.get("Plan")
        .ok_or("No 'Plan' key in EXPLAIN output")?;

    let root = parse_plan_node(plan_obj, 0, execution_ms);
    let total_rows = root.actual_rows;

    let mut issues = Vec::new();
    collect_issues(&root, &mut issues);

    Ok(ExplainResult {
        planning_ms,
        execution_ms,
        total_rows,
        plan: root,
        issues,
    })
}

fn parse_plan_node(node: &serde_json::Value, depth: u32, execution_ms: f64) -> PlanNode {
    let node_type = node.get("Node Type").and_then(|v| v.as_str()).unwrap_or("Unknown").to_string();
    let relation_name = node.get("Relation Name").and_then(|v| v.as_str()).map(|s| s.to_string());
    let index_name = node.get("Index Name").and_then(|v| v.as_str()).map(|s| s.to_string());

    let total_ms = node.get("Actual Total Time").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let loops = node.get("Actual Loops").and_then(|v| v.as_u64()).unwrap_or(1);
    let planned_rows = node.get("Plan Rows").and_then(|v| v.as_u64()).unwrap_or(0);
    let actual_rows = node.get("Actual Rows").and_then(|v| v.as_u64()).unwrap_or(0);
    let buffers_hit = node.get("Shared Hit Blocks").and_then(|v| v.as_u64());
    let buffers_read = node.get("Shared Read Blocks").and_then(|v| v.as_u64());

    // Parse child nodes
    let children: Vec<PlanNode> = node.get("Plans")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().map(|child| parse_plan_node(child, depth + 1, execution_ms)).collect())
        .unwrap_or_default();

    // Exclusive time = total_ms - sum of children total_ms
    let children_total: f64 = children.iter().map(|c| c.total_ms).sum();
    let exclusive_ms = (total_ms - children_total).max(0.0);

    let pct_of_total = if execution_ms > 0.0 {
        (exclusive_ms / execution_ms * 100.0).min(100.0).max(0.0)
    } else {
        0.0
    };

    PlanNode {
        node_type,
        relation_name,
        index_name,
        total_ms,
        exclusive_ms,
        pct_of_total,
        planned_rows,
        actual_rows,
        loops,
        buffers_hit,
        buffers_read,
        depth,
        children,
    }
}

fn collect_issues(node: &PlanNode, issues: &mut Vec<PlanIssue>) {
    // Detect sequential scan on large tables
    if node.node_type == "Seq Scan" && node.actual_rows > 1000 {
        let relation = node.relation_name.clone().unwrap_or_else(|| "unknown".to_string());
        issues.push(PlanIssue {
            severity: "danger".to_string(),
            kind: "seq_scan".to_string(),
            node_type: node.node_type.clone(),
            relation: Some(relation.clone()),
            message: format!("Sequential scan on `{}` scanned {} rows to return {}",
                relation, node.actual_rows, node.actual_rows),
            suggestion: format!("Consider adding an index on the filter column(s) used with `{}`", relation),
        });
    }

    // Detect row estimate mismatch (off by 10x or more)
    if node.planned_rows > 0 && node.actual_rows > 0 {
        let ratio = node.actual_rows as f64 / node.planned_rows as f64;
        if ratio > 10.0 || ratio < 0.1 {
            let relation = node.relation_name.clone().unwrap_or_else(|| node.node_type.clone());
            issues.push(PlanIssue {
                severity: "warning".to_string(),
                kind: "row_estimate_mismatch".to_string(),
                node_type: node.node_type.clone(),
                relation: Some(relation.clone()),
                message: format!("Row estimate mismatch on `{}`: planned {}, actual {} ({:.0}×)",
                    relation, node.planned_rows, node.actual_rows, ratio.max(1.0 / ratio)),
                suggestion: format!("Table statistics may be stale — run `ANALYZE {}`", relation),
            });
        }
    }

    // Recurse into children
    for child in &node.children {
        collect_issues(child, issues);
    }
}

// ── SQLite fallback ───────────────────────────────────────────────────────────

fn explain_sqlite(_query: &str) -> Result<ExplainResult, String> {
    // SQLite EXPLAIN returns a bytecode listing, not timing info
    // Return a minimal result with a "raw" node so the frontend can display a fallback
    Ok(ExplainResult {
        planning_ms: 0.0,
        execution_ms: 0.0,
        total_rows: 0,
        plan: PlanNode {
            node_type: "SQLite (limited)".to_string(),
            relation_name: None,
            index_name: None,
            total_ms: 0.0,
            exclusive_ms: 0.0,
            pct_of_total: 0.0,
            planned_rows: 0,
            actual_rows: 0,
            loops: 1,
            buffers_hit: None,
            buffers_read: None,
            depth: 0,
            children: vec![],
        },
        issues: vec![],
    })
}
