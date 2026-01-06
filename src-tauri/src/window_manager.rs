// Window manager for overlay prompt windows

use tauri::{AppHandle, Manager, Emitter, WebviewUrl, WebviewWindowBuilder};
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct WindowManager {
    app: AppHandle,
    prompt_window: Arc<Mutex<Option<tauri::WebviewWindow>>>,
    current_interval_id: Arc<Mutex<Option<i64>>>,
}

impl WindowManager {
    pub fn new(app: AppHandle) -> Self {
        Self {
            app,
            prompt_window: Arc::new(Mutex::new(None)),
            current_interval_id: Arc::new(Mutex::new(None)),
        }
    }

    /// Show the prompt window for an interval
    pub async fn show_prompt_window(&self, interval_id: i64) -> Result<(), String> {
        // Check if window already exists
        let mut prompt = self.prompt_window.lock().await;
        
        if prompt.is_some() {
            // Window already exists, just update the interval ID
            *self.current_interval_id.lock().await = Some(interval_id);
            return Ok(());
        }

        // Create the prompt window
        // For now, we'll use a URL that points to a route in the main app
        // In production, you might want a separate HTML file
        let window = WebviewWindowBuilder::new(
            &self.app,
            "prompt",
            WebviewUrl::App("index.html#/prompt".into()),
        )
        .title("Log15 - What did you do?")
        .inner_size(300.0, 120.0)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .visible(false) // Start hidden, will be shown after positioning
        .build()
        .map_err(|e| format!("Failed to create prompt window: {}", e))?;

        // Position window at bottom-right of screen
        if let Ok(monitor) = window.current_monitor() {
            if let Some(monitor) = monitor {
                let screen_size = monitor.size();
                // Use default size for positioning (window might not have size yet)
                let window_width = 300.0;
                let window_height = 120.0;
                
                let x = screen_size.width as f64 - window_width - 20.0; // 20px margin
                let y = screen_size.height as f64 - window_height - 20.0; // 20px margin
                
                let _ = window.set_position(tauri::LogicalPosition::new(x, y));
            }
        }

        // Store interval ID
        *self.current_interval_id.lock().await = Some(interval_id);

        // Show window with fade-in (handled by frontend CSS)
        window.show().map_err(|e| format!("Failed to show window: {}", e))?;
        window.set_focus().ok();

        // Send interval ID to frontend
        window
            .emit("prompt-interval-id", interval_id)
            .map_err(|e| format!("Failed to emit interval ID: {}", e))?;

        *prompt = Some(window);

        Ok(())
    }

    /// Hide the prompt window
    pub async fn hide_prompt_window(&self) -> Result<(), String> {
        let mut prompt = self.prompt_window.lock().await;
        
        if let Some(window) = prompt.take() {
            // Trigger fade-out animation (handled by frontend)
            window
                .emit("prompt-hide", ())
                .map_err(|e| format!("Failed to emit hide event: {}", e))?;
            
            // Wait a bit for animation, then actually hide
            tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
            window.hide().map_err(|e| format!("Failed to hide window: {}", e))?;
            
            *self.current_interval_id.lock().await = None;
        }

        Ok(())
    }

    /// Get current interval ID
    pub async fn get_current_interval_id(&self) -> Option<i64> {
        *self.current_interval_id.lock().await
    }
}
