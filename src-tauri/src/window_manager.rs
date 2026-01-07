// Window manager for overlay prompt windows

use tauri::{AppHandle, Manager, Emitter, WebviewUrl, WebviewWindowBuilder};
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct WindowManager {
    app: AppHandle,
    prompt_window: Arc<Mutex<Option<tauri::WebviewWindow>>>,
    current_interval_id: Arc<Mutex<Option<i64>>>,
    is_summary_ready: Arc<Mutex<bool>>,
}

impl WindowManager {
    pub fn new(app: AppHandle) -> Self {
        Self {
            app,
            prompt_window: Arc::new(Mutex::new(None)),
            current_interval_id: Arc::new(Mutex::new(None)),
            is_summary_ready: Arc::new(Mutex::new(false)),
        }
    }

    /// Show the prompt window for an interval
    /// Always creates a fresh window - closes any existing window first
    pub async fn show_prompt_window(&self, interval_id: i64) -> Result<(), String> {
        println!("[WINDOW_MGR] show_prompt_window called with interval_id={}", interval_id);
        
        // First, close any existing window (simplifies state management)
        self.hide_prompt_window().await.ok(); // Ignore errors if no window exists
        
        // Wait a moment for window to fully close before creating a new one
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        // Double-check: if window still exists in Tauri, try to close it again
        if let Some(existing_window) = self.app.get_webview_window("prompt") {
            println!("[WINDOW_MGR] Window still exists after hide, force closing");
            let _ = existing_window.close();
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
        
        // Store the new interval ID
        *self.current_interval_id.lock().await = Some(interval_id);

        println!("[WINDOW_MGR] Creating new prompt window");
        // Create the prompt window
        // For now, we'll use a URL that points to a route in the main app
        // In production, you might want a separate HTML file
        let window = WebviewWindowBuilder::new(
            &self.app,
            "prompt",
            WebviewUrl::App("index.html#/prompt".into()),
        )
        .title("Log15 - What did you do?")
        .inner_size(300.0, 180.0) // Increased height for summary view
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .visible(true) // Start visible - we'll position it immediately
        .build()
        .map_err(|e| {
            eprintln!("[WINDOW_MGR] Failed to create window: {}", e);
            format!("Failed to create prompt window: {}", e)
        })?;
        
        println!("[WINDOW_MGR] Window created successfully");

        // Position window at top-right of screen
        // Wait a moment for window to be ready before positioning
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        if let Ok(monitor) = window.current_monitor() {
            if let Some(monitor) = monitor {
                let screen_size = monitor.size();
                // Convert physical size to logical size (accounting for DPI scaling)
                let scale_factor = monitor.scale_factor();
                let logical_width = screen_size.width as f64 / scale_factor;
                let logical_height = screen_size.height as f64 / scale_factor;
                
                // Use default size for positioning
                let window_width = 300.0;
                let window_height = 180.0;
                
                let x = logical_width - window_width - 20.0; // 20px margin from right
                let y = 20.0; // 20px margin from top
                
                println!("[WINDOW_MGR] Positioning window at logical ({}, {}) on screen logical size ({}, {}), scale_factor: {}", 
                    x, y, logical_width, logical_height, scale_factor);
                
                let pos_result = window.set_position(tauri::LogicalPosition::new(x, y));
                match pos_result {
                    Ok(_) => println!("[WINDOW_MGR] Window positioned successfully"),
                    Err(e) => eprintln!("[WINDOW_MGR] Failed to position window: {}", e),
                }
            } else {
                eprintln!("[WINDOW_MGR] No monitor found");
            }
        } else {
            eprintln!("[WINDOW_MGR] Failed to get current monitor");
        }

        // Show window with fade-in (handled by frontend CSS)
        println!("[WINDOW_MGR] Showing window");
        
        window.show().map_err(|e| {
            eprintln!("[WINDOW_MGR] Failed to show window: {}", e);
            format!("Failed to show window: {}", e)
        })?;
        
        window.set_focus().ok();
        
        // Verify window is actually visible
        let is_visible = window.is_visible().unwrap_or(false);
        println!("[WINDOW_MGR] Window shown and focused. Is visible: {}", is_visible);
        
        // Get window position for debugging
        if let Ok(pos) = window.outer_position() {
            println!("[WINDOW_MGR] Window position: {:?}", pos);
        }
        
        if let Ok(size) = window.outer_size() {
            println!("[WINDOW_MGR] Window size: {:?}", size);
        }

        // Send interval ID to frontend BEFORE storing (so event is ready when window loads)
        println!("[WINDOW_MGR] Emitting prompt-interval-id event with interval_id={}", interval_id);
        window
            .emit("prompt-interval-id", interval_id)
            .map_err(|e| {
                eprintln!("[WINDOW_MGR] Failed to emit interval ID: {}", e);
                format!("Failed to emit interval ID: {}", e)
            })?;
        println!("[WINDOW_MGR] Event emitted successfully");

        // Store window in state AFTER everything is set up
        let mut prompt = self.prompt_window.lock().await;
        *prompt = Some(window);

        Ok(())
    }

    /// Show summary ready view (transitions from prompt to summary)
    pub async fn show_summary_ready(&self) -> Result<(), String> {
        let prompt = self.prompt_window.lock().await;
        
        if let Some(window) = prompt.as_ref() {
            // Set summary ready state
            *self.is_summary_ready.lock().await = true;
            
            // Emit event to show summary view
            window
                .emit("show-summary-ready", ())
                .map_err(|e| format!("Failed to emit show-summary event: {}", e))?;
        }

        Ok(())
    }

    /// Hide the prompt window
    /// Closes the window and clears all state
    pub async fn hide_prompt_window(&self) -> Result<(), String> {
        println!("[WINDOW_MGR] hide_prompt_window called");
        let mut prompt = self.prompt_window.lock().await;
        
        // Get window to hide - check our state first
        let window_to_close = if let Some(window) = prompt.as_ref() {
            // Window exists in our state
            Some(window)
        } else if let Some(window) = self.app.get_webview_window("prompt") {
            // Window exists in Tauri but not in our state - restore it temporarily
            // This can happen if state was cleared but window wasn't closed
            println!("[WINDOW_MGR] Window exists in Tauri but not in state, closing it");
            *prompt = Some(window);
            prompt.as_ref()
        } else {
            println!("[WINDOW_MGR] No window to hide");
            return Ok(());
        };
        
        if let Some(window) = window_to_close {
            // Check if summary is showing
            let is_summary = *self.is_summary_ready.lock().await;
            
            if is_summary {
                // Emit close event for summary
                window
                    .emit("close-summary", ())
                    .map_err(|e| format!("Failed to emit close-summary event: {}", e))?;
                
                // Wait for fade-out animation
                tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
                
                // Reset summary state
                *self.is_summary_ready.lock().await = false;
            } else {
                // Trigger fade-out animation (handled by frontend)
                window
                    .emit("prompt-hide", ())
                    .map_err(|e| format!("Failed to emit hide event: {}", e))?;
                
                // Wait a bit for animation, then actually hide
                tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
            }
            
            // Close the window
            window.close().map_err(|e| format!("Failed to close window: {}", e))?;
            
            // Clear all state
            *self.current_interval_id.lock().await = None;
            *prompt = None;
            println!("[WINDOW_MGR] Window closed successfully");
        }

        Ok(())
    }
    
    /// Check if summary window is currently showing
    pub async fn is_summary_ready(&self) -> bool {
        *self.is_summary_ready.lock().await
    }

    /// Get current interval ID
    pub async fn get_current_interval_id(&self) -> Option<i64> {
        *self.current_interval_id.lock().await
    }
}
