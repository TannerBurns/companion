use crate::pipeline::PipelineManager;
use std::sync::Arc;
use tauri::{
    image::Image,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager,
};
use tokio::sync::Mutex;

const TRAY_ICON: &[u8] = include_bytes!("../../icons/tray-icon.png");

pub fn init_tray(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let icon = Image::from_bytes(TRAY_ICON)?;

    let show_window = MenuItem::with_id(app, "show", "Show Companion", true, None::<&str>)?;
    let sync_now = MenuItem::with_id(app, "sync", "Sync Now", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show_window, &sync_now, &separator, &quit])?;

    let _tray = TrayIconBuilder::with_id("main")
        .icon(icon)
        .icon_as_template(true)
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
                if let Some(window) = tray.app_handle().get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        })
        .build(app)?;

    Ok(())
}

fn handle_menu_event(app: &AppHandle, event_id: &str) {
    match event_id {
        "show" => {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }
        "sync" => {
            let _ = app.emit("tray:sync-requested", ());
        }
        "quit" => {
            app.exit(0);
        }
        _ => {}
    }
}

pub async fn update_tray_tooltip(app: &AppHandle, pipeline: &Arc<Mutex<PipelineManager>>) {
    let pipeline = pipeline.lock().await;
    let message = pipeline.get_status_message().await;
    if let Some(tray) = app.tray_by_id("main") {
        let _ = tray.set_tooltip(Some(&message));
    }
}

pub fn spawn_tray_updater(app_handle: AppHandle, pipeline: Arc<Mutex<PipelineManager>>) {
    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(2));
        loop {
            interval.tick().await;
            update_tray_tooltip(&app_handle, &pipeline).await;
        }
    });
}
