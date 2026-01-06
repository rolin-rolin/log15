// System tray integration for Log15

use crate::db::{get_active_workblock, get_today_date, get_workblocks_by_date};
use tauri::{
    AppHandle, Manager, tray::{TrayIconBuilder, TrayIconEvent},
    menu::{Menu, MenuItem},
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TrayIconState {
    Idle,          // No active workblock
    Active,        // Workblock in progress
    SummaryReady,  // Workblock completed, summary available
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
        let start_workblock = MenuItem::with_id(app, "start_workblock", "Start Workblock", true, None::<&str>)?;
        let view_summary = MenuItem::with_id(app, "view_summary", "View Summary", false, None::<&str>)?;
        let view_last_words = MenuItem::with_id(app, "view_last_words", "View Last Words", false, None::<&str>)?;
        let show_window = MenuItem::with_id(app, "show_window", "Show Window", true, None::<&str>)?;
        let hide_window = MenuItem::with_id(app, "hide_window", "Hide Window", false, None::<&str>)?;
        let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

        // Create menu
        let menu = Menu::with_items(app, &[
            &start_workblock,
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
            .tooltip("Log15 - Workblock Tracker")
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
            TrayIconState::Idle => "Log15 - No active workblock",
            TrayIconState::Active => "Log15 - Workblock in progress",
            TrayIconState::SummaryReady => "Log15 - Summary ready",
        };

        // Update tooltip (icon state changes would require different icon files)
        // For MVP, we'll update tooltip and menu visibility
        self.update_menu().await;
    }

    /// Update tray menu based on current state
    pub async fn update_menu(&self) {
        let _has_active_workblock = get_active_workblock(&self.app).is_ok_and(|opt| opt.is_some());
        
        // Check if there are completed or cancelled workblocks today (summary available)
        let today = get_today_date();
        let _has_summary = get_workblocks_by_date(&self.app, &today)
            .map(|wbs| wbs.iter().any(|wb| {
                let status = wb.status.as_str();
                status == "completed" || status == "cancelled"
            }))
            .unwrap_or(false);

        // Note: Menu item visibility updates would require recreating the menu
        // For MVP, we'll handle this in the event handler by checking state
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
        let has_active = get_active_workblock(&self.app).is_ok_and(|opt| opt.is_some());
        
        let today = get_today_date();
        let has_summary = get_workblocks_by_date(&self.app, &today)
            .map(|wbs| wbs.iter().any(|wb| {
                let status = wb.status.as_str();
                status == "completed" || status == "cancelled"
            }))
            .unwrap_or(false);

        let new_state = if has_active {
            TrayIconState::Active
        } else if has_summary {
            TrayIconState::SummaryReady
        } else {
            TrayIconState::Idle
        };

        self.update_icon_state(new_state).await;
    }
}
