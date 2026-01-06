pub mod db;
pub mod timer;
pub mod tray;

pub use tray::TrayManager;

use db::{
    init_db, create_workblock, get_active_workblock, cancel_workblock,
    get_workblocks_by_date,
    add_interval, update_interval_words, get_intervals_by_workblock, get_current_interval,
    check_and_reset_daily, get_archived_day, get_today_date,
    generate_workblock_visualization, generate_daily_aggregate, generate_daily_visualization_data,
};
use timer::TimerManager;
use std::sync::Arc;
use tokio::sync::Mutex;
use tauri::{Manager, Emitter};

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
async fn start_workblock(
    app: tauri::AppHandle,
    duration_minutes: i32,
) -> Result<Workblock, String> {
    // Check and reset daily if needed
    check_and_reset_daily(&app).map_err(|e| e.to_string())?;
    
    // Check if there's already an active workblock
    if let Ok(Some(active)) = get_active_workblock(&app) {
        return Err(format!("Workblock {} is already active", active.id.unwrap()));
    }
    
    // Create workblock
    let workblock = create_workblock(&app, duration_minutes).map_err(|e| e.to_string())?;
    let workblock_id = workblock.id.unwrap();
    
    // Get timer manager from app state
    let timer_manager = app.state::<Arc<Mutex<TimerManager>>>();
    let timer = timer_manager.lock().await;
    
    // Start the timer
    timer.start_workblock(workblock_id, duration_minutes).await?;
    
    Ok(workblock)
}

#[tauri::command]
async fn stop_workblock(
    app: tauri::AppHandle,
    workblock_id: i64,
) -> Result<Workblock, String> {
    // Get the workblock first (before it's completed)
    let workblock = get_active_workblock(&app)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Workblock not found".to_string())?;
    
    // Get timer manager and stop the timer
    let timer_manager = app.state::<Arc<Mutex<TimerManager>>>();
    let timer = timer_manager.lock().await;
    
    // Stop the timer (this will also complete the workblock)
    timer.stop_workblock(workblock_id).await?;
    
    // Get the completed workblock
    get_workblocks_by_date(&app, &workblock.date)
        .map_err(|e| e.to_string())?
        .into_iter()
        .find(|wb| wb.id == Some(workblock_id))
        .ok_or_else(|| "Completed workblock not found".to_string())
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
async fn submit_interval_words(
    app: tauri::AppHandle,
    interval_id: i64,
    words: String,
) -> Result<Interval, String> {
    // Cancel auto-away timer since user submitted words
    let timer_manager = app.state::<Arc<Mutex<TimerManager>>>();
    let timer = timer_manager.lock().await;
    timer.cancel_auto_away_timer().await;
    drop(timer);
    
    // Update interval with words
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
async fn get_current_interval_cmd(
    app: tauri::AppHandle,
    workblock_id: i64,
) -> Result<Option<Interval>, String> {
    get_current_interval(&app, workblock_id).map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_timer_state(app: tauri::AppHandle) -> Result<timer::TimerState, String> {
    let timer_manager = app.state::<Arc<Mutex<TimerManager>>>();
    let timer = timer_manager.lock().await;
    Ok(timer.get_state().await)
}

#[tauri::command]
async fn get_interval_time_remaining(app: tauri::AppHandle) -> Result<Option<i64>, String> {
    let timer_manager = app.state::<Arc<Mutex<TimerManager>>>();
    let timer: tokio::sync::MutexGuard<'_, TimerManager> = timer_manager.lock().await;
    Ok(timer.get_interval_time_remaining().await)
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
            
            // Initialize timer manager
            let timer_manager = Arc::new(Mutex::new(TimerManager::new(app.handle().clone())));
            app.manage(timer_manager.clone());
            
            // Initialize tray manager
            let tray_manager = Arc::new(Mutex::new(TrayManager::new(app.handle().clone())));
            app.manage(tray_manager.clone());
            
            // Setup system tray
            if let Err(e) = TrayManager::setup_tray(&app.handle()) {
                eprintln!("Failed to setup system tray: {}", e);
            }
            
            // Restore active workblock if one exists (for app restart scenarios)
            tokio::spawn(async move {
                let timer = timer_manager.lock().await;
                if let Err(e) = timer.restore_active_workblock().await {
                    eprintln!("Failed to restore active workblock: {}", e);
                }
                drop(timer);
                
                // Refresh tray state after restoring workblock
                let mut tray = tray_manager.lock().await;
                tray.refresh_state().await;
            });
            
            Ok(())
        })
        .on_tray_icon_event(|app, event| {
            TrayManager::handle_tray_event(app, event);
        })
        .on_menu_event(|app, event| {
            // Handle menu item clicks
            let id_str = event.id.0.as_str();
            match id_str {
                "start_workblock" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                        let _ = window.emit("tray-start-workblock", ());
                    }
                }
                "view_summary" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                        let _ = window.emit("tray-view-summary", ());
                    }
                }
                "view_last_words" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                        let _ = window.emit("tray-view-last-words", ());
                    }
                }
                "show_window" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
                "hide_window" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.hide();
                    }
                }
                "quit" => {
                    app.exit(0);
                }
                _ => {}
            }
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
            get_timer_state,
            get_interval_time_remaining,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
