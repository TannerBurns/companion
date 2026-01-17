//! System tray functionality for Companion.
//!
//! Shows current pipeline status and provides quick actions.

use crate::pipeline::PipelineManager;
use std::sync::Arc;
use tauri::{
    image::Image,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager,
};
use tokio::sync::Mutex;

const TRAY_ICON_IDLE: &[u8] = include_bytes!("../../icons/32x32.png");

/// Initialize the system tray
pub fn init_tray(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let icon = Image::from_bytes(TRAY_ICON_IDLE)?;

    // Create menu items
    let show_window = MenuItem::with_id(app, "show", "Show Companion", true, None::<&str>)?;
    let sync_now = MenuItem::with_id(app, "sync", "Sync Now", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

    // Build menu
    let menu = Menu::with_items(app, &[&show_window, &sync_now, &separator, &quit])?;

    // Build tray icon
    let _tray = TrayIconBuilder::new()
        .icon(icon)
        .tooltip("Companion - Idle")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| {
            handle_menu_event(app, event.id.as_ref());
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                // Show main window on left click
                if let Some(window) = tray.app_handle().get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        })
        .build(app)?;

    Ok(())
}

/// Handle tray menu events
fn handle_menu_event(app: &AppHandle, event_id: &str) {
    match event_id {
        "show" => {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }
        "sync" => {
            // Trigger sync via event
            let _ = app.emit("tray:sync-requested", ());
        }
        "quit" => {
            app.exit(0);
        }
        _ => {}
    }
}

/// Update tray tooltip based on pipeline state
pub async fn update_tray_tooltip(app: &AppHandle, pipeline: &Arc<Mutex<PipelineManager>>) {
    let pipeline = pipeline.lock().await;
    let message = pipeline.get_status_message().await;
    
    // Update tray tooltip
    if let Some(tray) = app.tray_by_id("main") {
        let _ = tray.set_tooltip(Some(&message));
    }
}

/// Spawn a task that periodically updates the tray tooltip
pub fn spawn_tray_updater(app_handle: AppHandle, pipeline: Arc<Mutex<PipelineManager>>) {
    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(2));
        loop {
            interval.tick().await;
            update_tray_tooltip(&app_handle, &pipeline).await;
        }
    });
}
