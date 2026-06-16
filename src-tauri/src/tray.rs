// System tray integration for Log15

use crate::db::{get_active_session, get_today_date};
use tauri::{
    AppHandle, Manager, tray::{TrayIconBuilder, TrayIconEvent},
    menu::{Menu, MenuItem},
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TrayIconState {
    Idle,          // No active session
    Active,        // Session in progress
    SummaryReady,  // Session completed, summary available
}

pub struct TrayManager {
    app: AppHandle,
    current_state: TrayIconState,
}

impl TrayManager {
    pub fn new(app: AppHandle) -> Self {
        Self {
            app,
            current_state: TrayIconState::Idle,
        }
    }

    /// Create and setup the system tray
    pub fn setup_tray(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
        // Create menu items
        let start_session = MenuItem::with_id(app, "start_session", "Start Session", true, None::<&str>)?;
        let view_summary = MenuItem::with_id(app, "view_summary", "View Summary", false, None::<&str>)?;
        let view_last_words = MenuItem::with_id(app, "view_last_words", "View Last Words", false, None::<&str>)?;
        let show_window = MenuItem::with_id(app, "show_window", "Show Window", true, None::<&str>)?;
        let hide_window = MenuItem::with_id(app, "hide_window", "Hide Window", false, None::<&str>)?;
        let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

        // Create menu
        let menu = Menu::with_items(app, &[
            &start_session,
            &view_summary,
            &view_last_words,
            &show_window,
            &hide_window,
            &quit,
        ])?;

        // Build tray icon
        // Note: Icon loading from file requires image decoding
        // For MVP, we'll use default icon (can be enhanced later with custom icons for different states)
        let _tray_icon = TrayIconBuilder::new()
            .menu(&menu)
            .tooltip("Log15")
            .build(app)?;

        Ok(())
    }

    /// Update tray icon state
    pub async fn update_icon_state(&mut self, state: TrayIconState) {
        if self.current_state == state {
            return; // No change needed
        }

        self.current_state = state;

        // Update tooltip based on state
        let _tooltip = match state {
            TrayIconState::Idle => "Log15",
            TrayIconState::Active => "Log15 - Session in progress",
            TrayIconState::SummaryReady => "Log15 - Summary ready",
        };

        // Update tooltip (icon state changes would require different icon files)
        // For MVP, we'll update tooltip and menu visibility
        self.update_menu().await;
    }

    /// Update tray menu based on current state
    pub async fn update_menu(&self) {
        let _today = get_today_date();
        // Menu item visibility updates would require recreating the menu;
        // state is managed via update_icon_state calls from the command layer.
    }

    /// Handle tray events (click events)
    pub fn handle_tray_event(app: &AppHandle, event: TrayIconEvent) {
        match event {
            TrayIconEvent::Click { button, .. } => {
                if button == tauri::tray::MouseButton::Left {
                    // Toggle main window visibility
                    if let Some(window) = app.get_webview_window("main") {
                        let is_visible = window.is_visible().unwrap_or(false);
                        if is_visible {
                            let _ = window.hide();
                        } else {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                }
            }
            _ => {
                // Menu events are handled by menu item click handlers
            }
        }
    }

    /// Get current state
    pub fn get_state(&self) -> TrayIconState {
        self.current_state
    }

    /// Update tray state based on workblock status
    pub async fn refresh_state(&mut self) {
        let has_active = get_active_session(&self.app).is_ok_and(|opt| opt.is_some());

        let new_state = if has_active {
            TrayIconState::Active
        } else {
            // Only set to Idle if summary window is not open
            // Summary window state is managed separately via update_icon_state calls
            TrayIconState::Idle
        };

        // Only update if not already in SummaryReady state (which is managed by window manager)
        if self.current_state != TrayIconState::SummaryReady {
            self.update_icon_state(new_state).await;
        }
    }
}
