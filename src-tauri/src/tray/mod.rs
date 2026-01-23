use crate::pipeline::{PipelineManager, PipelineState, TaskStatus};
use std::sync::Arc;
use tauri::{
    image::Image,
    menu::{Menu, MenuItem, PredefinedMenuItem, Submenu},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager,
};
use tokio::sync::Mutex;

struct TrayCache {
    is_busy: bool,
    task_count: usize,
    tooltip: String,
}

impl TrayCache {
    fn new() -> Self {
        Self {
            is_busy: false,
            task_count: 0,
            tooltip: String::new(),
        }
    }
    
    fn needs_update(&self, state: &PipelineState, tooltip: &str) -> bool {
        self.is_busy != state.is_busy 
            || self.task_count != state.active_tasks.len()
            || self.tooltip != tooltip
    }
    
    fn update(&mut self, state: &PipelineState, tooltip: &str) {
        self.is_busy = state.is_busy;
        self.task_count = state.active_tasks.len();
        self.tooltip = tooltip.to_string();
    }
}

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
    
    let is_syncing = pipeline_state.map(|s| s.is_busy).unwrap_or(false);
    let sync_label = if is_syncing { "⟳ Syncing..." } else { "Sync Now" };
    let sync_now = MenuItem::with_id(app, "sync", sync_label, !is_syncing, None::<&str>)?;
    let check_updates = MenuItem::with_id(app, "check_updates", "Check for Updates", true, None::<&str>)?;
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
            &check_updates,
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
        "check_updates" => {
            show_main_window(app);
            let _ = app.emit("tray:check-for-updates", ());
        }
        "quit" => app.exit(0),
        _ => {}
    }
}

pub fn spawn_tray_updater(app_handle: AppHandle, pipeline: Arc<Mutex<PipelineManager>>) {
    tauri::async_runtime::spawn(async move {
        let mut cache = TrayCache::new();
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(2));
        
        loop {
            interval.tick().await;
            
            let pipeline = pipeline.lock().await;
            let message = pipeline.get_status_message().await;
            let state = pipeline.get_state().await;
            drop(pipeline);
            
            if let Some(tray) = app_handle.tray_by_id("main") {
                if cache.needs_update(&state, &message) {
                    tracing::debug!("Tray update: is_busy={}, tasks={}, message={}", 
                        state.is_busy, state.active_tasks.len(), message);
                    cache.update(&state, &message);
                    let _ = tray.set_tooltip(Some(&message));
                    if let Ok(menu) = build_tray_menu(&app_handle, Some(&state)) {
                        let _ = tray.set_menu(Some(menu));
                    }
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_state(is_busy: bool, active_task_count: usize) -> PipelineState {
        use crate::pipeline::{PipelineTask, PipelineTaskType, TaskStatus};
        
        let active: Vec<PipelineTask> = (0..active_task_count)
            .map(|i| PipelineTask {
                id: format!("task-{}", i),
                task_type: PipelineTaskType::SyncSlack,
                status: TaskStatus::Running,
                message: format!("Task {}", i),
                progress: None,
                started_at: 0,
                completed_at: None,
                error: None,
            })
            .collect();
        
        PipelineState {
            active_tasks: active,
            recent_history: vec![],
            is_busy,
        }
    }

    #[test]
    fn test_tray_cache_new_defaults() {
        let cache = TrayCache::new();
        assert!(!cache.is_busy);
        assert_eq!(cache.task_count, 0);
        assert!(cache.tooltip.is_empty());
    }

    #[test]
    fn test_tray_cache_needs_update_on_busy_change() {
        let cache = TrayCache::new();
        let state = make_state(true, 0);
        assert!(cache.needs_update(&state, "Companion"));
    }

    #[test]
    fn test_tray_cache_needs_update_on_task_count_change() {
        let cache = TrayCache::new();
        let state = make_state(false, 3);
        assert!(cache.needs_update(&state, "Companion"));
    }

    #[test]
    fn test_tray_cache_needs_update_on_tooltip_change() {
        let cache = TrayCache::new();
        let state = make_state(false, 0);
        assert!(cache.needs_update(&state, "New tooltip"));
    }

    #[test]
    fn test_tray_cache_no_update_when_unchanged() {
        let mut cache = TrayCache::new();
        let state = make_state(false, 0);
        cache.update(&state, "Companion");
        
        let same_state = make_state(false, 0);
        assert!(!cache.needs_update(&same_state, "Companion"));
    }

    #[test]
    fn test_tray_cache_update_stores_values() {
        let mut cache = TrayCache::new();
        let state = make_state(true, 5);
        cache.update(&state, "Syncing...");
        
        assert!(cache.is_busy);
        assert_eq!(cache.task_count, 5);
        assert_eq!(cache.tooltip, "Syncing...");
    }

    #[test]
    fn test_tray_cache_detects_busy_to_idle_transition() {
        let mut cache = TrayCache::new();
        let busy_state = make_state(true, 2);
        cache.update(&busy_state, "⟳ Syncing...");
        
        let idle_state = make_state(false, 3);
        assert!(cache.needs_update(&idle_state, "Companion"));
    }

    #[test]
    fn test_tray_cache_detects_active_task_change() {
        let mut cache = TrayCache::new();
        let state = make_state(false, 2);
        cache.update(&state, "Companion");
        
        let new_state = make_state(false, 4);
        assert!(cache.needs_update(&new_state, "Companion"));
    }
}
