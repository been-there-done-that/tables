use std::{
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};
use rusqlite::{Connection, params};
use log::{info, warn, error};

#[derive(Debug, Default)]
struct Metrics {
    lock_wait_ms: Vec<u128>,
    busy_errors: usize,
}

fn simulate_introspection(db: Arc<Mutex<Connection>>, metrics: Arc<Mutex<Metrics>>) {
    info!("Starting introspection simulator (writer)...");
    let start = Instant::now();
    let mut conn = db.lock().unwrap();

    let waited = start.elapsed().as_millis();
    metrics.lock().unwrap().lock_wait_ms.push(waited);

    info!("Writer acquired lock after {}ms. Starting transaction...", waited);

    let tx_res = conn.transaction();
    match tx_res {
        Ok(tx) => {
            for i in 0..100 {
                if let Err(e) = tx.execute(
                    "INSERT OR REPLACE INTO meta_columns (connection_id, table_name, ordinal_position, column_name, raw_type, logical_type, nullable, is_primary_key)
                     VALUES (?1, 'test_table', ?2, ?3, 'TEXT', 'text', 1, 0)",
                    params!["test_conn", i, format!("col_{i}")],
                ) {
                    error!("Writer insert failed: {}", e);
                }

                // Artificial delay to amplify contention
                thread::sleep(Duration::from_millis(10));
            }

            if let Err(e) = tx.commit() {
                error!("Writer commit failed: {}", e);
            } else {
                info!("Writer committed successfully.");
            }
        }
        Err(e) => {
            error!("Writer failed to start transaction: {}", e);
        }
    }
}

fn simulate_ui_read(id: usize, db: Arc<Mutex<Connection>>, metrics: Arc<Mutex<Metrics>>) {
    info!("Starting UI reader {}...", id);
    for _i in 0..20 {
        let start = Instant::now();

        let conn_lock = db.lock().unwrap();
        let waited = start.elapsed().as_millis();
        metrics.lock().unwrap().lock_wait_ms.push(waited);

        let result = conn_lock.query_row(
            "SELECT COUNT(*) FROM meta_columns",
            [],
            |row| row.get::<_, i64>(0),
        );

        match result {
            Ok(_count) => {
                // info!("Reader {} count: {}", id, _count);
            }
            Err(e) => {
                if e.to_string().contains("busy") {
                    warn!("Reader {} hit SQLITE_BUSY", id);
                    metrics.lock().unwrap().busy_errors += 1;
                } else {
                    error!("Reader {} query failed: {}", id, e);
                }
            }
        }

        drop(conn_lock);
        // Small gap between reads
        thread::sleep(Duration::from_millis(50));
    }
    info!("UI reader {} finished.", id);
}

pub fn run_lock_contention_test(db: Arc<Mutex<Connection>>) -> String {
    let metrics = Arc::new(Mutex::new(Metrics::default()));

    let mut handles = vec![];

    // One writer
    {
        let db = db.clone();
        let metrics = metrics.clone();
        handles.push(thread::spawn(move || {
            simulate_introspection(db, metrics);
        }));
    }

    // Three readers
    for i in 0..3 {
        let db = db.clone();
        let metrics = metrics.clone();
        handles.push(thread::spawn(move || {
            simulate_ui_read(i, db, metrics);
        }));
    }

    for h in handles {
        let _ = h.join();
    }

    let m = metrics.lock().unwrap();
    let avg_wait = if m.lock_wait_ms.is_empty() {
        0.0
    } else {
        m.lock_wait_ms.iter().sum::<u128>() as f64 / m.lock_wait_ms.len() as f64
    };
    let max_wait = m.lock_wait_ms.iter().max().cloned().unwrap_or(0);

    let report = format!(
        "--- Lock Contention Test Results ---\nSamples: {}\nAvg Wait: {:.2}ms\nMax Wait: {}ms\nBusy Errors: {}",
        m.lock_wait_ms.len(),
        avg_wait,
        max_wait,
        m.busy_errors
    );
    
    info!("{}", report);
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrations;
    
    #[test]
    fn test_sqlite_contention() {
        // Init in-memory DB
        let conn = Connection::open_in_memory().unwrap();
        
        // Setup WAL and busy timeout
        conn.pragma_update(None, "journal_mode", "WAL").unwrap();
        conn.pragma_update(None, "busy_timeout", 5000).unwrap();
        
        // Apply migrations
        migrations::apply(&conn, || 0).unwrap();
        
        let db = Arc::new(Mutex::new(conn));
        let report = run_lock_contention_test(db);
        
        // Check results
        assert!(report.contains("Busy Errors: 0"), "Should have 0 busy errors");
        info!("Test passed with report: {}", report);
    }
}
