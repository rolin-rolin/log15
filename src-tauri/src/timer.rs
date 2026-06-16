use crate::db::{
    add_interval, get_active_session, get_current_interval, get_interval_by_id,
    update_interval_words, IntervalStatus,
};
use crate::window_manager::WindowManager;
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::env;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::Mutex;
use tokio::time::{interval, Duration};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimerConfigInfo {
    pub dev_mode: bool,
    pub interval_tick_seconds: u64,
    pub auto_away_delay_seconds: u64,
    pub logical_interval_minutes: i32,
}

#[derive(Debug, Clone)]
struct TimerConfig {
    dev_mode: bool,
    interval_tick: Duration,
    auto_away_delay: Duration,
    logical_interval_minutes: i32,
}

impl TimerConfig {
    fn load() -> Self {
        let mut cfg = Self {
            dev_mode: false,
            interval_tick: Duration::from_secs(15 * 60),
            auto_away_delay: Duration::from_secs(10 * 60),
            logical_interval_minutes: 15,
        };

        if cfg!(debug_assertions) {
            let dev_enabled = env::var("LOG15_DEV_TIMERS")
                .ok()
                .map(|v| matches!(v.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
                .unwrap_or(false);

            if dev_enabled {
                cfg.dev_mode = true;
                let tick_secs = env::var("LOG15_DEV_INTERVAL_SECONDS")
                    .ok()
                    .and_then(|v| v.parse::<u64>().ok())
                    .unwrap_or(10);
                let auto_away_secs = env::var("LOG15_DEV_AUTO_AWAY_SECONDS")
                    .ok()
                    .and_then(|v| v.parse::<u64>().ok())
                    .unwrap_or(5);
                let logical_minutes = env::var("LOG15_DEV_LOGICAL_INTERVAL_MINUTES")
                    .ok()
                    .and_then(|v| v.parse::<i32>().ok())
                    .unwrap_or(15)
                    .max(1);

                cfg.interval_tick = Duration::from_secs(tick_secs.max(1));
                cfg.auto_away_delay = Duration::from_secs(auto_away_secs.max(1));
                cfg.logical_interval_minutes = logical_minutes;

                println!(
                    "[TIMER] Dev timers enabled: interval_tick={}s, auto_away_delay={}s",
                    cfg.interval_tick.as_secs(),
                    cfg.auto_away_delay.as_secs(),
                );
            }
        }

        cfg
    }

    fn info(&self) -> TimerConfigInfo {
        TimerConfigInfo {
            dev_mode: self.dev_mode,
            interval_tick_seconds: self.interval_tick.as_secs(),
            auto_away_delay_seconds: self.auto_away_delay.as_secs(),
            logical_interval_minutes: self.logical_interval_minutes,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimerState {
    pub session_id: Option<i64>,
    pub current_interval_id: Option<i64>,
    pub current_interval_number: i32,
    pub interval_start_time: Option<DateTime<Local>>,
    pub prompt_shown_time: Option<DateTime<Local>>,
    pub is_running: bool,
}

impl Default for TimerState {
    fn default() -> Self {
        Self {
            session_id: None,
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
    config: TimerConfig,
}

impl TimerManager {
    pub fn new(app: AppHandle) -> Self {
        Self {
            state: Arc::new(Mutex::new(TimerState::default())),
            app,
            interval_handle: Arc::new(Mutex::new(None)),
            auto_away_handle: Arc::new(Mutex::new(None)),
            config: TimerConfig::load(),
        }
    }

    pub fn get_config_info(&self) -> TimerConfigInfo {
        self.config.info()
    }

    pub fn logical_interval_minutes(&self) -> i32 {
        self.config.logical_interval_minutes
    }

    /// Start (or resume) a session. Reuses an existing pending interval if present,
    /// otherwise creates interval 1.
    pub async fn start_session(&self, session_id: i64) -> Result<(), String> {
        let mut state = self.state.lock().await;

        if state.is_running {
            return Err("A session is already running".to_string());
        }

        // Reuse an existing pending interval (app-restart resume) or create a fresh one
        let (current_interval_id, current_interval_number) =
            match get_current_interval(&self.app, session_id) {
                Ok(Some(existing)) => (existing.id, existing.interval_number),
                _ => {
                    let first = add_interval(&self.app, session_id, 1)
                        .map_err(|e| format!("Failed to create interval: {}", e))?;
                    (first.id, 1)
                }
            };

        state.session_id = Some(session_id);
        state.current_interval_id = current_interval_id;
        state.current_interval_number = current_interval_number;
        state.interval_start_time = Some(Local::now());
        state.is_running = true;
        drop(state);

        let state_clone = Arc::clone(&self.state);
        let app_clone = self.app.clone();
        let config = self.config.clone();
        let start_interval_num = current_interval_number;

        let handle = tokio::spawn(async move {
            let mut interval_timer = interval(config.interval_tick);
            interval_timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            interval_timer.tick().await; // consume immediate first tick

            let mut current_num = start_interval_num;

            loop {
                interval_timer.tick().await;

                let state = state_clone.lock().await;
                if !state.is_running || state.session_id.is_none() {
                    break;
                }
                let session_id = state.session_id.unwrap();
                let interval_id = state.current_interval_id;
                let interval_number = state.current_interval_number;
                let prompt_time = Local::now();
                drop(state);

                if let Some(interval_id) = interval_id {
                    let _ = app_clone.emit("interval-complete", serde_json::json!({
                        "session_id": session_id,
                        "interval_id": interval_id,
                        "interval_number": interval_number
                    }));

                    let mut state = state_clone.lock().await;
                    state.prompt_shown_time = Some(prompt_time);
                    drop(state);

                    if let Some(timer_mgr) = app_clone.try_state::<Arc<Mutex<TimerManager>>>() {
                        let timer = timer_mgr.lock().await;
                        let _ = timer.start_auto_away_timer(interval_id).await;
                    }
                }

                // Create next interval before sleeping again
                current_num += 1;
                let mut state = state_clone.lock().await;
                if !state.is_running { break; }
                if let Ok(new_interval) = add_interval(&app_clone, session_id, current_num) {
                    state.current_interval_id = new_interval.id;
                    state.current_interval_number = current_num;
                    state.interval_start_time = Some(Local::now());
                }
                drop(state);
            }
        });

        *self.interval_handle.lock().await = Some(handle);
        Ok(())
    }

    /// Stop the in-memory timer state and cancel background tasks.
    /// DB cleanup (pending interval deletion, session status) is handled by the command layer.
    pub async fn stop_session(&self) -> Result<(), String> {
        let mut state = self.state.lock().await;
        *state = TimerState::default();
        drop(state);

        if let Some(handle) = self.interval_handle.lock().await.take() {
            handle.abort();
        }
        if let Some(handle) = self.auto_away_handle.lock().await.take() {
            handle.abort();
        }

        Ok(())
    }

    /// Start the auto-away timer (fires if the user doesn't respond to a prompt).
    pub async fn start_auto_away_timer(&self, interval_id: i64) -> Result<(), String> {
        if let Some(handle) = self.auto_away_handle.lock().await.take() {
            handle.abort();
        }

        let app_clone = self.app.clone();
        let state_clone = Arc::clone(&self.state);
        let config = self.config.clone();

        let handle = tokio::spawn(async move {
            tokio::time::sleep(config.auto_away_delay).await;

            if let Ok(interval) = get_interval_by_id(&app_clone, interval_id) {
                if interval.words.is_none() {
                    let _ = update_interval_words(
                        &app_clone,
                        interval_id,
                        "Away from workspace".to_string(),
                        IntervalStatus::AutoAway,
                    );

                    println!("[TIMER] Auto-away recorded for interval {}", interval_id);

                    let state = state_clone.lock().await;
                    let still_running = state.is_running;
                    drop(state);

                    if still_running {
                        let _ = app_clone.emit("auto-away", interval_id);
                        let _ = app_clone.emit("prompt-hide", ());

                        if let Some(wm) = app_clone.try_state::<Arc<tauri::async_runtime::Mutex<WindowManager>>>() {
                            let wm = wm.lock().await;
                            let _ = wm.hide_prompt_window().await;
                        }
                    }
                }
            }
        });

        *self.auto_away_handle.lock().await = Some(handle);
        Ok(())
    }

    pub async fn cancel_auto_away_timer(&self) {
        if let Some(handle) = self.auto_away_handle.lock().await.take() {
            handle.abort();
        }
    }

    pub async fn get_state(&self) -> TimerState {
        self.state.lock().await.clone()
    }

    pub async fn get_interval_time_remaining(&self) -> Option<i64> {
        let state = self.state.lock().await;
        if let Some(start_time) = state.interval_start_time {
            let elapsed = (Local::now() - start_time).num_seconds();
            let interval_secs = self.config.interval_tick.as_secs() as i64;
            Some((interval_secs - elapsed).max(0))
        } else {
            None
        }
    }

    /// Called on app startup: resumes an active session if one exists in the DB.
    pub async fn restore_active_session(&self) -> Result<(), String> {
        match get_active_session(&self.app) {
            Ok(Some(session)) => {
                let session_id = session.id.unwrap();
                self.start_session(session_id).await?;
            }
            Ok(None) => {}
            Err(e) => return Err(format!("Failed to get active session: {}", e)),
        }
        Ok(())
    }
}
