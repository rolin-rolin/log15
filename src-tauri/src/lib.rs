pub mod db;

use db::{
    init_db, create_workblock, get_active_workblock, complete_workblock, cancel_workblock,
    get_workblocks_by_date,
    add_interval, update_interval_words, get_intervals_by_workblock, get_current_interval,
    check_and_reset_daily, get_archived_day, get_today_date,
    generate_workblock_visualization, generate_daily_aggregate, generate_daily_visualization_data,
};

// Re-export types for frontend
pub use db::{Workblock, Interval, DailyArchive, WorkblockStatus, IntervalStatus};

// ============================================================================
// Tauri Commands
// ============================================================================

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn init_database(app: tauri::AppHandle) -> Result<(), String> {
    init_db(&app).map_err(|e| e.to_string())?;
    Ok(())
}

// Workblock commands
#[tauri::command]
fn start_workblock(app: tauri::AppHandle, duration_minutes: i32) -> Result<Workblock, String> {
    // Check and reset daily if needed
    check_and_reset_daily(&app).map_err(|e| e.to_string())?;
    
    // Check if there's already an active workblock
    if let Ok(Some(active)) = get_active_workblock(&app) {
        return Err(format!("Workblock {} is already active", active.id.unwrap()));
    }
    
    create_workblock(&app, duration_minutes).map_err(|e| e.to_string())
}

#[tauri::command]
fn stop_workblock(app: tauri::AppHandle, workblock_id: i64) -> Result<Workblock, String> {
    complete_workblock(&app, workblock_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn cancel_workblock_cmd(app: tauri::AppHandle, workblock_id: i64) -> Result<Workblock, String> {
    cancel_workblock(&app, workblock_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_active_workblock_cmd(app: tauri::AppHandle) -> Result<Option<Workblock>, String> {
    get_active_workblock(&app).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_workblocks_by_date_cmd(app: tauri::AppHandle, date: String) -> Result<Vec<Workblock>, String> {
    get_workblocks_by_date(&app, &date).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_today_workblocks(app: tauri::AppHandle) -> Result<Vec<Workblock>, String> {
    let today = get_today_date();
    get_workblocks_by_date(&app, &today).map_err(|e| e.to_string())
}

// Interval commands
#[tauri::command]
fn create_interval(app: tauri::AppHandle, workblock_id: i64, interval_number: i32) -> Result<Interval, String> {
    add_interval(&app, workblock_id, interval_number).map_err(|e| e.to_string())
}

#[tauri::command]
fn submit_interval_words(
    app: tauri::AppHandle,
    interval_id: i64,
    words: String,
) -> Result<Interval, String> {
    update_interval_words(&app, interval_id, words, IntervalStatus::Recorded)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn auto_away_interval(app: tauri::AppHandle, interval_id: i64) -> Result<Interval, String> {
    update_interval_words(&app, interval_id, "Away from workspace".to_string(), IntervalStatus::AutoAway)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_intervals_by_workblock_cmd(app: tauri::AppHandle, workblock_id: i64) -> Result<Vec<Interval>, String> {
    get_intervals_by_workblock(&app, workblock_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_current_interval_cmd(app: tauri::AppHandle, workblock_id: i64) -> Result<Option<Interval>, String> {
    get_current_interval(&app, workblock_id).map_err(|e| e.to_string())
}

// Daily commands
#[tauri::command]
fn check_and_reset_daily_cmd(app: tauri::AppHandle) -> Result<Option<String>, String> {
    check_and_reset_daily(&app).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_today_date_cmd() -> String {
    get_today_date()
}

#[tauri::command]
fn get_archived_day_cmd(app: tauri::AppHandle, date: String) -> Result<Option<DailyArchive>, String> {
    get_archived_day(&app, &date).map_err(|e| e.to_string())
}

// Visualization commands
#[tauri::command]
fn get_workblock_visualization(app: tauri::AppHandle, workblock_id: i64) -> Result<String, String> {
    let viz = generate_workblock_visualization(&app, workblock_id)
        .map_err(|e| e.to_string())?;
    serde_json::to_string(&viz).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_daily_aggregate_cmd(app: tauri::AppHandle, date: String) -> Result<String, String> {
    let aggregate = generate_daily_aggregate(&app, &date)
        .map_err(|e| e.to_string())?;
    serde_json::to_string(&aggregate).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_daily_visualization_data_cmd(app: tauri::AppHandle, date: String) -> Result<String, String> {
    let data = generate_daily_visualization_data(&app, &date)
        .map_err(|e| e.to_string())?;
    serde_json::to_string(&data).map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // Initialize database on app startup
            if let Err(e) = init_db(&app.handle()) {
                eprintln!("Failed to initialize database: {}", e);
            }
            
            // Check and reset daily on startup
            if let Err(e) = check_and_reset_daily(&app.handle()) {
                eprintln!("Failed to check daily reset: {}", e);
            }
            
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            init_database,
            start_workblock,
            stop_workblock,
            cancel_workblock_cmd,
            get_active_workblock_cmd,
            get_workblocks_by_date_cmd,
            get_today_workblocks,
            create_interval,
            submit_interval_words,
            auto_away_interval,
            get_intervals_by_workblock_cmd,
            get_current_interval_cmd,
            check_and_reset_daily_cmd,
            get_today_date_cmd,
            get_archived_day_cmd,
            get_workblock_visualization,
            get_daily_aggregate_cmd,
            get_daily_visualization_data_cmd,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
