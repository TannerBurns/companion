// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use companion::db::Database;
use companion::crypto::CryptoService;
use companion::notifications::NotificationService;
use companion::analytics::AnalyticsService;
use companion::pipeline::PipelineManager;
use companion::sync::{SyncQueue, BackgroundSyncService};
use companion::tray;
use companion::AppState;
use companion::commands;
use std::sync::Arc;
use tauri::Manager;
use tokio::sync::Mutex;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("companion=info"))
        )
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            let app_handle = app.handle().clone();

            tauri::async_runtime::block_on(async {
                let db = Database::new(&app_handle)
                    .await
                    .expect("Failed to initialize database");
                let crypto = CryptoService::new().expect("Failed to initialize crypto service");

                // Use a single shared database instance to avoid SQLite concurrency issues
                let db_arc = Arc::new(db);
                let crypto_arc = Arc::new(crypto.clone());
                let notifications = NotificationService::new(app_handle.clone());
                let analytics = AnalyticsService::new(db_arc.clone());

                let mut pipeline = PipelineManager::new();
                pipeline.set_app_handle(app_handle.clone());
                let pipeline_arc = Arc::new(Mutex::new(pipeline));

                let sync_queue = SyncQueue::new();
                let sync_lock = Arc::new(tokio::sync::Mutex::new(()));

                let sync_interval = load_sync_interval(db_arc.clone()).await;
                tracing::info!("Loaded sync interval: {} minutes", sync_interval);

                let background_sync = BackgroundSyncService::new(
                    app_handle.clone(),
                    db_arc.clone(),
                    crypto_arc.clone(),
                    pipeline_arc.clone(),
                    sync_lock.clone(),
                    sync_interval,
                );
                let background_sync_arc = Arc::new(background_sync);
                let is_syncing = background_sync_arc.is_syncing_flag();
                let next_sync_at = background_sync_arc.next_sync_at_flag();

                app.manage(Arc::new(Mutex::new(AppState {
                    db: db_arc.clone(),
                    crypto,
                    notifications: Some(notifications),
                    analytics: Some(analytics),
                    pipeline: pipeline_arc.clone(),
                    sync_queue,
                    sync_lock,
                    background_sync: Some(background_sync_arc.clone()),
                    is_syncing,
                    next_sync_at,
                })));

                if let Err(e) = tray::init_tray(&app_handle) {
                    tracing::error!("Failed to initialize system tray: {}", e);
                }

                tray::spawn_tray_updater(app_handle.clone(), pipeline_arc);

                let bg_sync = background_sync_arc.clone();
                tauri::async_runtime::spawn(async move {
                    bg_sync.run_startup_sync_if_needed().await;
                    bg_sync.start();
                });
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_daily_digest,
            commands::get_weekly_digest,
            commands::start_sync,
            commands::get_sync_status,
            commands::save_api_key,
            commands::has_api_key,
            commands::get_preferences,
            commands::save_preferences,
            commands::connect_slack,
            commands::connect_atlassian,
            commands::select_atlassian_resource,
            commands::track_event,
            commands::get_analytics_summary,
            commands::get_pipeline_status,
            commands::list_slack_channels,
            commands::list_slack_users,
            commands::save_slack_channels,
            commands::get_saved_slack_channels,
            commands::remove_slack_channel,
            commands::get_slack_connection_status,
            commands::disconnect_slack,
            commands::save_gemini_credentials,
            commands::verify_gemini_connection,
            commands::get_gemini_auth_type,
            commands::get_data_stats,
            commands::clear_synced_data,
            commands::factory_reset,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

async fn load_sync_interval(db: Arc<Database>) -> u64 {
    let result: Option<(String,)> = sqlx::query_as(
        "SELECT value FROM preferences WHERE key = 'user_preferences'"
    )
    .fetch_optional(db.pool())
    .await
    .ok()
    .flatten();

    if let Some((json,)) = result {
        if let Ok(prefs) = serde_json::from_str::<serde_json::Value>(&json) {
            // Preferences are serialized with camelCase
            if let Some(interval) = prefs.get("syncIntervalMinutes").and_then(|v| v.as_i64()) {
                return interval.max(1) as u64;
            }
        }
    }

    15
}
