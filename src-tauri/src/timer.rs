// Timer system for managing workblocks and 15-minute intervals

use crate::db::{
    add_interval, get_active_workblock, get_current_interval, get_interval_by_id,
    get_workblock_by_id, update_interval_words, complete_workblock, IntervalStatus,
};
use crate::tray::{TrayIconState, TrayManager};
use crate::window_manager::WindowManager;
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::Mutex;
use tokio::time::{interval, Duration};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimerState {
    pub workblock_id: Option<i64>,
    pub current_interval_id: Option<i64>,
    pub current_interval_number: i32,
    pub interval_start_time: Option<DateTime<Local>>,
    pub prompt_shown_time: Option<DateTime<Local>>, // When prompt window was shown
    pub is_running: bool,
}

impl Default for TimerState {
    fn default() -> Self {
        Self {
            workblock_id: None,
            current_interval_id: None,
            current_interval_number: 0,
            interval_start_time: None,
            prompt_shown_time: None,
            is_running: false,
        }
    }
}

pub struct TimerManager {
    state: Arc<Mutex<TimerState>>,
    app: AppHandle,
    interval_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    auto_away_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

impl TimerManager {
    pub fn new(app: AppHandle) -> Self {
        Self {
            state: Arc::new(Mutex::new(TimerState::default())),
            app,
            interval_handle: Arc::new(Mutex::new(None)),
            auto_away_handle: Arc::new(Mutex::new(None)),
        }
    }

    /// Start a workblock timer
    pub async fn start_workblock(&self, workblock_id: i64, duration_minutes: i32) -> Result<(), String> {
        let mut state = self.state.lock().await;
        
        if state.is_running {
            return Err("A workblock is already running".to_string());
        }

        // Calculate number of intervals
        // TESTING: Calculate intervals based on 10-second intervals instead of 15-minute
        // For testing: 1 interval per 10 seconds, so duration_minutes * 6 intervals per minute
        let total_intervals = duration_minutes * 6; // TESTING: Changed from duration_minutes / 15
        
        // Initialize state
        state.workblock_id = Some(workblock_id);
        state.current_interval_number = 0;
        state.is_running = true;
        
        // Create first interval and set its start time
        match add_interval(&self.app, workblock_id, 1) {
            Ok(interval) => {
                state.current_interval_id = interval.id;
                state.current_interval_number = 1;
                state.interval_start_time = Some(Local::now()); // Set start time when interval is created
            }
            Err(e) => {
                state.is_running = false;
                return Err(format!("Failed to create interval: {}", e));
            }
        }

        // Start the interval timer
        let state_clone = Arc::clone(&self.state);
        let app_clone = self.app.clone();
        
        let handle = tokio::spawn(async move {
            // TESTING: 10 seconds instead of 15 minutes
            let mut interval_timer = interval(Duration::from_secs(10)); // TESTING: Changed from 15 * 60
            interval_timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            // Consume the immediate first tick to establish the baseline "now"
            // After this, each tick represents a full interval duration passing
            interval_timer.tick().await;

            // Start with interval 1 (the first interval that was already created)
            let mut current_interval_num = 1;
            let total_intervals = total_intervals;
            
            loop {
                // Wait for the current interval to complete (full duration)
                interval_timer.tick().await;
                
                // Check if timer should still be running
                let state = state_clone.lock().await;
                if !state.is_running || state.workblock_id.is_none() {
                    break;
                }
                let workblock_id = state.workblock_id.unwrap();
                drop(state);
                
                // Emit interval-complete event with interval info
                // Use the current interval number BEFORE incrementing
                let state = state_clone.lock().await;
                let interval_id = state.current_interval_id;
                let interval_number = state.current_interval_number; // Use state's interval number
                let prompt_time = Local::now();
                drop(state);
                
                if let Some(interval_id) = interval_id {
                    println!("[TIMER] Emitting interval-complete: interval_id={}, interval_number={}", interval_id, interval_number);
                    let _ = app_clone.emit("interval-complete", serde_json::json!({
                        "workblock_id": workblock_id,
                        "interval_id": interval_id,
                        "interval_number": interval_number
                    }));
                    
                    // Update prompt shown time
                    let mut state = state_clone.lock().await;
                    state.prompt_shown_time = Some(prompt_time);
                    drop(state);
                    
                    // Emit event to show prompt window (frontend will handle it)
                    // The frontend will listen for interval-complete and call show_prompt_window_cmd
                }
                
                // Check if we've reached the total number of intervals
                // Increment for next interval
                current_interval_num += 1;
                if current_interval_num > total_intervals {
                    // We've completed the final interval tick.
                    // IMPORTANT: Do NOT mark the workblock completed here.
                    // The workblock should only complete after the final interval gets recorded
                    // (either user submission or auto-away).
                    println!(
                        "[TIMER] Final interval tick complete (interval_number={}); awaiting final prompt submission/auto-away",
                        interval_number
                    );
                    break;
                }
                
                // Create next interval (for the next cycle)
                let mut state = state_clone.lock().await;
                if let Ok(new_interval) = add_interval(&app_clone, workblock_id, current_interval_num) {
                    state.current_interval_id = new_interval.id;
                    state.current_interval_number = current_interval_num; // Update state with new interval number
                    state.interval_start_time = Some(Local::now());
                    // Don't set prompt_shown_time here - it will be set when the prompt actually appears
                    println!("[TIMER] Created next interval: interval_number={}", current_interval_num);
                }
                drop(state);
            }
        });
        
        *self.interval_handle.lock().await = Some(handle);
        
        Ok(())
    }

    /// Complete the current workblock (when it naturally finishes)
    pub async fn complete_workblock(&self, workblock_id: i64) -> Result<(), String> {
        let mut state = self.state.lock().await;
        
        if state.workblock_id != Some(workblock_id) {
            return Err("Workblock ID mismatch".to_string());
        }
        
        state.is_running = false;
        drop(state);
        
        // Cancel interval timer
        if let Some(handle) = self.interval_handle.lock().await.take() {
            handle.abort();
        }
        
        // Cancel auto-away timer
        if let Some(handle) = self.auto_away_handle.lock().await.take() {
            handle.abort();
        }
        
        // Complete the workblock
        complete_workblock(&self.app, workblock_id)
            .map_err(|e| format!("Failed to complete workblock: {}", e))?;
        
        // Emit workblock-complete event
        let _ = self.app.emit("workblock-complete", workblock_id);
        
        // Reset state
        let mut state = self.state.lock().await;
        *state = TimerState::default();
        
        Ok(())
    }

    /// Cancel the current workblock (when user clicks cancel)
    pub async fn cancel_workblock(&self, workblock_id: i64) -> Result<(), String> {
        let mut state = self.state.lock().await;
        
        if state.workblock_id != Some(workblock_id) {
            return Err("Workblock ID mismatch".to_string());
        }
        
        state.is_running = false;
        drop(state);
        
        // Cancel interval timer
        if let Some(handle) = self.interval_handle.lock().await.take() {
            handle.abort();
        }
        
        // Cancel auto-away timer
        if let Some(handle) = self.auto_away_handle.lock().await.take() {
            handle.abort();
        }
        
        // Cancel the workblock (sets status to cancelled)
        crate::db::cancel_workblock(&self.app, workblock_id)
            .map_err(|e| format!("Failed to cancel workblock: {}", e))?;
        
        // Emit workblock-complete event (frontend can check status to see if cancelled)
        let _ = self.app.emit("workblock-complete", workblock_id);
        
        // Reset state
        let mut state = self.state.lock().await;
        *state = TimerState::default();
        
        Ok(())
    }

    /// Start the auto-away timer (10 minutes after prompt is shown)
    pub async fn start_auto_away_timer(&self, interval_id: i64) -> Result<(), String> {
        // Cancel any existing auto-away timer
        if let Some(handle) = self.auto_away_handle.lock().await.take() {
            handle.abort();
        }
        
        let app_clone = self.app.clone();
        let state_clone = Arc::clone(&self.state);
        let interval_handle_clone = Arc::clone(&self.interval_handle);
        
        let handle = tokio::spawn(async move {
            // TESTING: 5 seconds instead of 10 minutes
            tokio::time::sleep(Duration::from_secs(5)).await; // TESTING: Changed from 10 * 60
            
            // Check if the specific interval still has no recorded words
            if let Ok(interval) = get_interval_by_id(&app_clone, interval_id) {
                if interval.words.is_none() {
                    // Auto-away: record "Away from workspace"
                    let _ = update_interval_words(
                        &app_clone,
                        interval_id,
                        "Away from workspace".to_string(),
                        IntervalStatus::AutoAway,
                    );
                    
                    // Hide prompt window - emit events that frontend will handle
                    println!("[TIMER] Auto-away: Recording 'Away from workspace' for interval {}", interval_id);
                    
                    // Emit auto-away event (PromptWindow listens for this)
                    let _ = app_clone.emit("auto-away", interval_id);
                    
                    // Also emit prompt-hide to ensure window closes
                    let _ = app_clone.emit("prompt-hide", ());
                    
                    // Call hide command directly to ensure window closes
                    // Note: We use try_state which returns Option, and Tauri uses async_runtime::Mutex
                    if let Some(window_mgr_state) = app_clone.try_state::<Arc<tauri::async_runtime::Mutex<WindowManager>>>() {
                        let window_mgr = window_mgr_state.lock().await;
                        let _ = window_mgr.hide_prompt_window().await;
                        println!("[TIMER] Auto-away: Called hide_prompt_window");
                    }

                    // If this was the last interval, finalize the workblock now.
                    // (Timer loop intentionally does not complete the workblock on the last tick.)
                    if let Ok(workblock) = get_workblock_by_id(&app_clone, interval.workblock_id) {
                        let total_intervals = workblock.duration_minutes.unwrap_or(60) * 6; // TESTING
                        let is_last_interval = interval.interval_number >= total_intervals;

                        if is_last_interval {
                            println!(
                                "[TIMER] Auto-away on final interval; completing workblock_id={}",
                                interval.workblock_id
                            );

                            let _ = complete_workblock(&app_clone, interval.workblock_id);
                            let _ = app_clone.emit("workblock-complete", interval.workblock_id);

                            // Update tray state to SummaryReady
                            if let Some(tray_mgr_state) = app_clone.try_state::<Arc<Mutex<TrayManager>>>() {
                                let mut tray = tray_mgr_state.lock().await;
                                tray.update_icon_state(TrayIconState::SummaryReady).await;
                            }

                            // Reset timer state
                            let mut state = state_clone.lock().await;
                            *state = TimerState::default();
                            drop(state);

                            // Stop interval ticking task if it still exists
                            if let Some(h) = interval_handle_clone.lock().await.take() {
                                h.abort();
                            }
                        }
                    }
                }
            }
        });
        
        *self.auto_away_handle.lock().await = Some(handle);
        
        Ok(())
    }

    /// Cancel the auto-away timer (when user submits words)
    pub async fn cancel_auto_away_timer(&self) {
        if let Some(handle) = self.auto_away_handle.lock().await.take() {
            handle.abort();
        }
    }

    /// Get current timer state
    pub async fn get_state(&self) -> TimerState {
        self.state.lock().await.clone()
    }

    /// Get time remaining in current interval (in seconds)
    pub async fn get_interval_time_remaining(&self) -> Option<i64> {
        let state = self.state.lock().await;
        
        if let Some(start_time) = state.interval_start_time {
            let elapsed = (Local::now() - start_time).num_seconds();
            let remaining = 10 - elapsed; // TESTING: 10 seconds (normally 15 * 60 = 900)
            Some(remaining.max(0))
        } else {
            None
        }
    }

    /// Check if there's an active workblock and restore timer if needed
    pub async fn restore_active_workblock(&self) -> Result<(), String> {
        // Check database for active workblock
        match get_active_workblock(&self.app) {
            Ok(Some(workblock)) => {
                let workblock_id = workblock.id.unwrap();
                let duration = workblock.duration_minutes.unwrap_or(60);
                
                // Get current interval
                if let Ok(Some(current_interval)) = get_current_interval(&self.app, workblock_id) {
                    let mut state = self.state.lock().await;
                    state.workblock_id = Some(workblock_id);
                    state.current_interval_id = current_interval.id;
                    state.current_interval_number = current_interval.interval_number;
                    state.interval_start_time = Some(
                        DateTime::parse_from_rfc3339(&current_interval.start_time)
                            .unwrap()
                            .with_timezone(&Local),
                    );
                    state.is_running = true;
                    drop(state);
                    
                    // Calculate remaining intervals
                    let elapsed_intervals = current_interval.interval_number;
                    // TESTING: 10-second intervals (duration_minutes * 6 per minute)
                    let total_intervals = duration * 6; // TESTING: Changed from duration / 15
                    let remaining_intervals = total_intervals - elapsed_intervals;
                    
                    if remaining_intervals > 0 {
                        // Restart timer for remaining intervals
                        // Note: This is a simplified version - in production, you'd want to
                        // calculate the exact time remaining in the current interval
                        self.start_workblock(workblock_id, duration).await?;
                    }
                } else {
                    // No current interval, start fresh
                    self.start_workblock(workblock_id, duration).await?;
                }
            }
            Ok(None) => {
                // No active workblock, reset state
                let mut state = self.state.lock().await;
                *state = TimerState::default();
            }
            Err(e) => {
                return Err(format!("Failed to get active workblock: {}", e));
            }
        }
        
        Ok(())
    }
}
