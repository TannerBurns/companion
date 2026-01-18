use crate::pipeline::{PipelineManager, PipelineState, TaskStatus};
use std::sync::Arc;
use tauri::{
    image::Image,
    menu::{Menu, MenuItem, PredefinedMenuItem, Submenu},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager,
};
use tokio::sync::Mutex;

const TRAY_ICON: &[u8] = include_bytes!("../../icons/tray-icon.png");

pub fn init_tray(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let icon = Image::from_bytes(TRAY_ICON)?;
    let menu = build_tray_menu(app, None)?;

    let _tray = TrayIconBuilder::with_id("main")
        .icon(icon)
        .icon_as_template(true)
        .tooltip("Companion - Idle")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| {
            handle_menu_event(app, event.id.as_ref());
        })
        .build(app)?;

    Ok(())
}

fn build_tray_menu(
    app: &AppHandle,
    pipeline_state: Option<&PipelineState>,
) -> Result<Menu<tauri::Wry>, Box<dyn std::error::Error>> {
    let show_window = MenuItem::with_id(app, "show", "Show Companion", true, None::<&str>)?;
    let open_settings = MenuItem::with_id(app, "settings", "Open Settings", true, None::<&str>)?;
    let sync_now = MenuItem::with_id(app, "sync", "Sync Now", true, None::<&str>)?;
    let separator1 = PredefinedMenuItem::separator(app)?;
    let activity_submenu = build_activity_submenu(app, pipeline_state)?;
    
    let separator2 = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

    let menu = Menu::with_items(
        app,
        &[
            &show_window,
            &open_settings,
            &sync_now,
            &separator1,
            &activity_submenu,
            &separator2,
            &quit,
        ],
    )?;

    Ok(menu)
}

fn build_activity_submenu(
    app: &AppHandle,
    pipeline_state: Option<&PipelineState>,
) -> Result<Submenu<tauri::Wry>, Box<dyn std::error::Error>> {
    use tauri::menu::IsMenuItem;
    
    let tasks: Vec<_> = pipeline_state
        .map(|s| s.recent_history.iter().take(5).collect())
        .unwrap_or_default();

    let items: Vec<MenuItem<tauri::Wry>> = if tasks.is_empty() {
        vec![MenuItem::with_id(
            app,
            "activity_empty",
            "No recent activity",
            false,
            None::<&str>,
        )?]
    } else {
        tasks
            .iter()
            .enumerate()
            .map(|(idx, task)| {
                let status_icon = match task.status {
                    TaskStatus::Completed => "✓",
                    TaskStatus::Failed => "✗",
                    TaskStatus::Running => "◎",
                    TaskStatus::Pending => "○",
                };
                let label = format!("{} {}", status_icon, task.task_type.display_name());
                MenuItem::with_id(app, format!("activity_{}", idx), &label, false, None::<&str>)
            })
            .collect::<Result<Vec<_>, _>>()?
    };

    let item_refs: Vec<&dyn IsMenuItem<tauri::Wry>> = items
        .iter()
        .map(|item| item as &dyn IsMenuItem<tauri::Wry>)
        .collect();
    let submenu = Submenu::with_items(app, "Recent Activity", true, &item_refs)?;

    Ok(submenu)
}

fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn handle_menu_event(app: &AppHandle, event_id: &str) {
    match event_id {
        "show" => show_main_window(app),
        "settings" => {
            show_main_window(app);
            let _ = app.emit("tray:open-settings", ());
        }
        "sync" => {
            let _ = app.emit("tray:sync-requested", ());
        }
        "quit" => app.exit(0),
        _ => {}
    }
}

pub async fn update_tray(app: &AppHandle, pipeline: &Arc<Mutex<PipelineManager>>) {
    let pipeline = pipeline.lock().await;
    let message = pipeline.get_status_message().await;
    let state = pipeline.get_state().await;
    drop(pipeline); // Release lock before menu operations
    
    if let Some(tray) = app.tray_by_id("main") {
        let _ = tray.set_tooltip(Some(&message));
        if let Ok(menu) = build_tray_menu(app, Some(&state)) {
            let _ = tray.set_menu(Some(menu));
        }
    }
}

pub fn spawn_tray_updater(app_handle: AppHandle, pipeline: Arc<Mutex<PipelineManager>>) {
    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(2));
        loop {
            interval.tick().await;
            update_tray(&app_handle, &pipeline).await;
        }
    });
}
