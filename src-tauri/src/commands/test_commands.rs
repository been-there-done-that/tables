use tauri::State;
use crate::DatabaseState;
use crate::db_test::run_lock_contention_test;
use log::info;

#[tauri::command]
pub async fn run_db_contention_test(db_state: State<'_, DatabaseState>) -> Result<String, String> {
    info!("Triggering DB contention test...");
    let db = db_state.conn.clone();
    let report = run_lock_contention_test(db);
    Ok(report)
}
