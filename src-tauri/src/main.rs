// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use companion::db::Database;
use companion::crypto::CryptoService;
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
                
                // Store state
                app.manage(Arc::new(Mutex::new(AppState { db, crypto })));
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
