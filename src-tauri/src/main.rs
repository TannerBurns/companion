// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use companion::db::Database;
use companion::crypto::CryptoService;
use companion::notifications::NotificationService;
use companion::analytics::AnalyticsService;
use companion::pipeline::PipelineManager;
use companion::sync::SyncQueue;
use companion::tray;
use companion::AppState;
use companion::commands;
use std::sync::Arc;
use tauri::Manager;
use tokio::sync::Mutex;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

fn main() {
    // Initialize logging
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let app_handle = app.handle().clone();
            
            tauri::async_runtime::block_on(async {
                // Initialize database
                let db = Database::new(&app_handle).await
                    .expect("Failed to initialize database");
                
                // Initialize crypto service
                let crypto = CryptoService::new()
                    .expect("Failed to initialize crypto service");
                
                // Create Arc for database to share with services
                let db_arc = Arc::new(db);
                
                // Initialize notification service
                let notifications = NotificationService::new(app_handle.clone());
                
                // Initialize analytics service
                let analytics = AnalyticsService::new(db_arc.clone());
                
                // Initialize pipeline manager
                let mut pipeline = PipelineManager::new();
                pipeline.set_app_handle(app_handle.clone());
                let pipeline_arc = Arc::new(Mutex::new(pipeline));
                
                // Initialize sync queue for offline support
                let sync_queue = SyncQueue::new();
                
                // Create new database instance for state (since we moved it to Arc)
                let db_for_state = Database::new(&app_handle).await
                    .expect("Failed to initialize database");
                
                // Store state
                app.manage(Arc::new(Mutex::new(AppState {
                    db: db_for_state,
                    crypto,
                    notifications: Some(notifications),
                    analytics: Some(analytics),
                    pipeline: pipeline_arc.clone(),
                    sync_queue,
                })));
                
                // Initialize system tray
                if let Err(e) = tray::init_tray(&app_handle) {
                    tracing::error!("Failed to initialize system tray: {}", e);
                }
                
                // Spawn tray tooltip updater
                tray::spawn_tray_updater(app_handle.clone(), pipeline_arc);
            });
            
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_daily_digest,
            commands::get_weekly_digest,
            commands::start_sync,
            commands::get_sync_status,
            commands::save_api_key,
            commands::get_preferences,
            commands::save_preferences,
            commands::connect_slack,
            commands::connect_atlassian,
            commands::select_atlassian_resource,
            commands::track_event,
            commands::get_analytics_summary,
            commands::get_pipeline_status,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
