// Timer system for managing workblocks and 15-minute intervals

use crate::db::{
    add_interval, get_active_workblock, get_current_interval, update_interval_words,
    complete_workblock, IntervalStatus,
};
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
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
        let total_intervals = duration_minutes / 15;
        
        // Initialize state
        state.workblock_id = Some(workblock_id);
        state.current_interval_number = 0;
        state.is_running = true;
        state.interval_start_time = Some(Local::now());
        
        // Create first interval
        match add_interval(&self.app, workblock_id, 1) {
            Ok(interval) => {
                state.current_interval_id = interval.id;
                state.current_interval_number = 1;
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
            let mut interval_timer = interval(Duration::from_secs(15 * 60)); // 15 minutes
            interval_timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            
            // Wait for first tick (15 minutes from now)
            interval_timer.tick().await;
            
            let mut current_interval_num = 1;
            let total_intervals = total_intervals;
            
            loop {
                // Check if timer should still be running
                let state = state_clone.lock().await;
                if !state.is_running || state.workblock_id.is_none() {
                    break;
                }
                let workblock_id = state.workblock_id.unwrap();
                drop(state);
                
                // Emit interval-complete event with interval info
                let state = state_clone.lock().await;
                let interval_id = state.current_interval_id;
                let prompt_time = Local::now();
                drop(state);
                
                if let Some(interval_id) = interval_id {
                    let _ = app_clone.emit("interval-complete", serde_json::json!({
                        "workblock_id": workblock_id,
                        "interval_id": interval_id,
                        "interval_number": current_interval_num
                    }));
                    
                    // Update prompt shown time
                    let mut state = state_clone.lock().await;
                    state.prompt_shown_time = Some(prompt_time);
                    drop(state);
                    
                    // Start auto-away timer in a separate task
                    let state_for_auto_away = Arc::clone(&state_clone);
                    let app_for_auto_away = app_clone.clone();
                    tokio::spawn(async move {
                        tokio::time::sleep(Duration::from_secs(10 * 60)).await;
                        
                        let state = state_for_auto_away.lock().await;
                        if let Some(current_interval_id) = state.current_interval_id {
                            if current_interval_id == interval_id {
                                // Check if interval was already recorded
                                if let Ok(Some(interval)) = get_current_interval(&app_for_auto_away, workblock_id) {
                                    if interval.id == Some(interval_id) && interval.words.is_none() {
                                        // Auto-away: record "Away from workspace"
                                        let _ = update_interval_words(
                                            &app_for_auto_away,
                                            interval_id,
                                            "Away from workspace".to_string(),
                                            IntervalStatus::AutoAway,
                                        );
                                        
                                        // Emit auto-away event
                                        let _ = app_for_auto_away.emit("auto-away", interval_id);
                                    }
                                }
                            }
                        }
                    });
                }
                
                // Check if we've reached the total number of intervals
                current_interval_num += 1;
                if current_interval_num > total_intervals {
                    // Workblock is complete
                    let mut state = state_clone.lock().await;
                    state.is_running = false;
                    drop(state);
                    
                    // Complete the workblock
                    let _ = complete_workblock(&app_clone, workblock_id);
                    
                    // Emit workblock-complete event
                    let _ = app_clone.emit("workblock-complete", workblock_id);
                    break;
                }
                
                // Create next interval
                let mut state = state_clone.lock().await;
                if let Ok(new_interval) = add_interval(&app_clone, workblock_id, current_interval_num) {
                    state.current_interval_id = new_interval.id;
                    state.current_interval_number = current_interval_num;
                    state.interval_start_time = Some(Local::now());
                    state.prompt_shown_time = Some(Local::now()); // Prompt should be shown now
                }
                drop(state);
                
                // Wait for next interval
                interval_timer.tick().await;
            }
        });
        
        *self.interval_handle.lock().await = Some(handle);
        
        Ok(())
    }

    /// Stop the current workblock
    pub async fn stop_workblock(&self, workblock_id: i64) -> Result<(), String> {
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

    /// Start the auto-away timer (10 minutes after prompt is shown)
    pub async fn start_auto_away_timer(&self, interval_id: i64) -> Result<(), String> {
        // Cancel any existing auto-away timer
        if let Some(handle) = self.auto_away_handle.lock().await.take() {
            handle.abort();
        }
        
        let app_clone = self.app.clone();
        let state_clone = Arc::clone(&self.state);
        
        let handle = tokio::spawn(async move {
            // Wait 10 minutes
            tokio::time::sleep(Duration::from_secs(10 * 60)).await;
            
            // Check if interval still exists and hasn't been recorded
            let state = state_clone.lock().await;
            if let Some(current_interval_id) = state.current_interval_id {
                if current_interval_id == interval_id {
                    // Auto-away: record "Away from workspace"
                    let _ = update_interval_words(
                        &app_clone,
                        interval_id,
                        "Away from workspace".to_string(),
                        IntervalStatus::AutoAway,
                    );
                    
                    // Emit auto-away event
                    let _ = app_clone.emit("auto-away", interval_id);
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
            let remaining = (15 * 60) - elapsed; // 15 minutes in seconds
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
                    let total_intervals = duration / 15;
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
