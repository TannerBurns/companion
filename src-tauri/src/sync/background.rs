use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tauri::{AppHandle, Emitter};

use crate::db::Database;
use crate::crypto::CryptoService;

pub struct BackgroundSyncService {
    app_handle: AppHandle,
    db: Arc<Database>,
    crypto: Arc<CryptoService>,
    interval_minutes: u64,
    is_running: Arc<Mutex<bool>>,
}

impl BackgroundSyncService {
    pub fn new(
        app_handle: AppHandle,
        db: Arc<Database>,
        crypto: Arc<CryptoService>,
        interval_minutes: u64,
    ) -> Self {
        Self {
            app_handle,
            db,
            crypto,
            interval_minutes,
            is_running: Arc::new(Mutex::new(false)),
        }
    }
    
    /// Start the background sync loop
    pub async fn start(&self) {
        let mut is_running = self.is_running.lock().await;
        if *is_running {
            return;
        }
        *is_running = true;
        drop(is_running);
        
        let app_handle = self.app_handle.clone();
        let db = self.db.clone();
        let crypto = self.crypto.clone();
        let interval = self.interval_minutes;
        let is_running = self.is_running.clone();
        
        tokio::spawn(async move {
            loop {
                let running = is_running.lock().await;
                if !*running {
                    break;
                }
                drop(running);
                
                Self::run_sync_cycle(&app_handle, &db, &crypto).await;
                tokio::time::sleep(Duration::from_secs(interval * 60)).await;
            }
        });
    }
    
    /// Stop the background sync loop
    pub async fn stop(&self) {
        let mut is_running = self.is_running.lock().await;
        *is_running = false;
    }
    
    /// Run a single sync cycle
    async fn run_sync_cycle(app_handle: &AppHandle, _db: &Database, _crypto: &CryptoService) {
        tracing::info!("Starting sync cycle");
        let _ = app_handle.emit("sync:started", ());
        
        // TODO: Load credentials and run sync services
        
        let _ = app_handle.emit("sync:completed", serde_json::json!({
            "items_synced": 0,
            "duration_ms": 0,
        }));
        
        tracing::info!("Sync cycle completed");
    }
}
