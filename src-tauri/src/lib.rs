pub mod db;
pub mod timer;
pub mod tray;
pub mod window_manager;
#[cfg(test)]
mod db_test;

pub use tray::TrayManager;

use db::{
    init_db, create_session, get_active_session, stop_session,
    add_interval, update_interval_words, get_intervals_by_session, get_current_interval,
    check_and_reset_daily, archive_all_unarchived_dates, archive_daily_data, get_archived_day,
    get_all_archived_dates, get_today_date, generate_daily_visualization_data, delete_day,
};
use timer::TimerManager;
use window_manager::WindowManager;
use std::sync::Arc;
use tokio::sync::Mutex;
use tauri::{Manager, Emitter, async_runtime};
use timer::TimerConfigInfo;

pub use db::{Session, Interval, DailyArchive, SessionStatus, IntervalStatus};

// ============================================================================
// Tauri Commands
// ============================================================================

#[tauri::command]
fn init_database(app: tauri::AppHandle) -> Result<(), String> {
    init_db(&app).map_err(|e| e.to_string())?;
    Ok(())
}

// Session commands

#[tauri::command]
async fn start_session_cmd(app: tauri::AppHandle) -> Result<Session, String> {
    check_and_reset_daily(&app).map_err(|e| e.to_string())?;

    if let Ok(Some(active)) = get_active_session(&app) {
        return Err(format!("Session {} is already active", active.id.unwrap()));
    }

    let session = create_session(&app).map_err(|e| e.to_string())?;
    let session_id = session.id.unwrap();

    let timer_manager = app.state::<Arc<Mutex<TimerManager>>>();
    let timer = timer_manager.lock().await;
    timer.start_session(session_id).await?;

    Ok(session)
}

#[tauri::command]
async fn stop_session_cmd(app: tauri::AppHandle, session_id: i64) -> Result<Session, String> {
    // Verify there is an active session matching this ID
    let active = get_active_session(&app)
        .map_err(|e| format!("Failed to get active session: {}", e))?
        .ok_or_else(|| "No active session found".to_string())?;

    if active.id != Some(session_id) {
        return Err(format!(
            "Session ID mismatch: expected {}, got {:?}",
            session_id, active.id
        ));
    }

    // Hide prompt window if open
    let window_manager = app.state::<Arc<Mutex<WindowManager>>>();
    let window_mgr = window_manager.lock().await;
    window_mgr.hide_prompt_window().await.ok();
    drop(window_mgr);

    // Stop the timer
    let timer_manager = app.state::<Arc<Mutex<TimerManager>>>();
    let timer = timer_manager.lock().await;
    timer.cancel_auto_away_timer().await;
    timer.stop_session().await?;
    drop(timer);

    // Finalize in DB (deletes pending intervals, sets end_time + status)
    let stopped = stop_session(&app, session_id).map_err(|e| e.to_string())?;

    // Archive the day now that the session is complete
    let _ = archive_daily_data(&app, &stopped.date);

    let _ = app.emit("session-stopped", session_id);

    // Update tray state to Idle
    let tray_manager = app.state::<Arc<Mutex<TrayManager>>>();
    let mut tray = tray_manager.lock().await;
    tray.update_icon_state(crate::tray::TrayIconState::Idle).await;

    Ok(stopped)
}

#[tauri::command]
fn get_active_session_cmd(app: tauri::AppHandle) -> Result<Option<Session>, String> {
    get_active_session(&app).map_err(|e| e.to_string())
}

// Interval commands

#[tauri::command]
fn create_interval(app: tauri::AppHandle, session_id: i64, interval_number: i32) -> Result<Interval, String> {
    add_interval(&app, session_id, interval_number).map_err(|e| e.to_string())
}

#[tauri::command]
async fn submit_interval_words(
    app: tauri::AppHandle,
    interval_id: i64,
    words: String,
) -> Result<serde_json::Value, String> {
    // Cancel auto-away since user submitted words
    let timer_manager = app.state::<Arc<Mutex<TimerManager>>>();
    let timer = timer_manager.lock().await;
    timer.cancel_auto_away_timer().await;
    drop(timer);

    let interval = update_interval_words(&app, interval_id, words, IntervalStatus::Recorded)
        .map_err(|e| e.to_string())?;

    // Close prompt window after checkmark animation
    let app_clone = app.clone();
    async_runtime::spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;
        let window_manager = app_clone.state::<Arc<Mutex<WindowManager>>>();
        let wm = window_manager.lock().await;
        let _ = wm.hide_prompt_window().await;
    });

    Ok(serde_json::json!({ "interval": interval }))
}

#[tauri::command]
fn auto_away_interval(app: tauri::AppHandle, interval_id: i64) -> Result<Interval, String> {
    update_interval_words(&app, interval_id, "Away from workspace".to_string(), IntervalStatus::AutoAway)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_intervals_by_session_cmd(app: tauri::AppHandle, session_id: i64) -> Result<Vec<Interval>, String> {
    get_intervals_by_session(&app, session_id).map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_current_interval_cmd(
    app: tauri::AppHandle,
    session_id: i64,
) -> Result<Option<Interval>, String> {
    get_current_interval(&app, session_id).map_err(|e| e.to_string())
}

// Timer commands

#[tauri::command]
async fn get_timer_state(app: tauri::AppHandle) -> Result<timer::TimerState, String> {
    let timer_manager = app.state::<Arc<Mutex<TimerManager>>>();
    let timer = timer_manager.lock().await;
    Ok(timer.get_state().await)
}

#[tauri::command]
async fn get_interval_time_remaining(app: tauri::AppHandle) -> Result<Option<i64>, String> {
    let timer_manager = app.state::<Arc<Mutex<TimerManager>>>();
    let timer = timer_manager.lock().await;
    Ok(timer.get_interval_time_remaining().await)
}

#[tauri::command]
async fn get_timer_config_cmd(app: tauri::AppHandle) -> Result<TimerConfigInfo, String> {
    let timer_manager = app.state::<Arc<Mutex<TimerManager>>>();
    let timer = timer_manager.lock().await;
    Ok(timer.get_config_info())
}

// Window management commands

#[tauri::command]
async fn show_prompt_window_cmd(
    app: tauri::AppHandle,
    interval_id: i64,
) -> Result<(), String> {
    let window_manager = app.state::<Arc<Mutex<WindowManager>>>();
    let window_mgr = window_manager.lock().await;
    window_mgr.show_prompt_window(interval_id).await
}

#[tauri::command]
async fn hide_prompt_window_cmd(app: tauri::AppHandle) -> Result<(), String> {
    let window_manager = app.state::<Arc<Mutex<WindowManager>>>();
    let window_mgr = window_manager.lock().await;

    let was_summary = window_mgr.is_summary_ready().await;
    window_mgr.hide_prompt_window().await?;

    if was_summary {
        let tray_manager = app.state::<Arc<Mutex<TrayManager>>>();
        let mut tray = tray_manager.lock().await;
        tray.update_icon_state(crate::tray::TrayIconState::Idle).await;
    }

    Ok(())
}

// Daily / archive commands

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

#[tauri::command]
fn get_all_archived_dates_cmd(app: tauri::AppHandle) -> Result<Vec<DailyArchive>, String> {
    let _ = archive_all_unarchived_dates(&app);
    get_all_archived_dates(&app).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_day_cmd(app: tauri::AppHandle, date: String) -> Result<(), String> {
    delete_day(&app, &date).map_err(|e| e.to_string())
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
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            if let Err(e) = init_db(&app.handle()) {
                eprintln!("Failed to initialize database: {}", e);
            }

            if let Err(e) = check_and_reset_daily(&app.handle()) {
                eprintln!("Failed to check daily reset: {}", e);
            }

            let timer_manager = Arc::new(Mutex::new(TimerManager::new(app.handle().clone())));
            app.manage(timer_manager.clone());

            let tray_manager = Arc::new(Mutex::new(TrayManager::new(app.handle().clone())));
            app.manage(tray_manager.clone());

            let window_manager = Arc::new(Mutex::new(WindowManager::new(app.handle().clone())));
            app.manage(window_manager);

            if let Err(e) = TrayManager::setup_tray(&app.handle()) {
                eprintln!("Failed to setup system tray: {}", e);
            }

            let timer_clone = timer_manager.clone();
            let tray_clone = tray_manager.clone();
            async_runtime::spawn(async move {
                let timer = timer_clone.lock().await;
                if let Err(e) = timer.restore_active_session().await {
                    eprintln!("Failed to restore active session: {}", e);
                }
                drop(timer);

                let mut tray = tray_clone.lock().await;
                tray.refresh_state().await;
            });

            Ok(())
        })
        .on_tray_icon_event(|app, event| {
            TrayManager::handle_tray_event(app, event);
        })
        .on_menu_event(|app, event| {
            let id_str = event.id.0.as_str();
            match id_str {
                "start_session" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                        let _ = window.emit("tray-start-session", ());
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
            init_database,
            start_session_cmd,
            stop_session_cmd,
            get_active_session_cmd,
            create_interval,
            submit_interval_words,
            auto_away_interval,
            get_intervals_by_session_cmd,
            get_current_interval_cmd,
            check_and_reset_daily_cmd,
            get_today_date_cmd,
            get_archived_day_cmd,
            get_all_archived_dates_cmd,
            get_daily_visualization_data_cmd,
            get_timer_state,
            get_interval_time_remaining,
            get_timer_config_cmd,
            show_prompt_window_cmd,
            hide_prompt_window_cmd,
            delete_day_cmd,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
