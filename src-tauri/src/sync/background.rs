use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tauri::{AppHandle, Emitter};

use crate::db::Database;
use crate::crypto::CryptoService;
use super::slack::{SlackClient, SlackSyncService, SlackTokens};

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
                
                Self::run_sync_cycle(&app_handle, db.clone(), crypto.clone()).await;
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
    async fn run_sync_cycle(app_handle: &AppHandle, db: Arc<Database>, crypto: Arc<CryptoService>) {
        tracing::info!("Starting sync cycle");
        let start = Instant::now();
        let _ = app_handle.emit("sync:started", ());
        
        let mut total_items = 0;
        let mut errors: Vec<String> = Vec::new();
        
        // Sync Slack if connected
        match Self::sync_slack(db.clone(), crypto.clone()).await {
            Ok(items) => {
                total_items += items;
                tracing::info!("Slack sync completed: {} items", items);
            }
            Err(e) => {
                if !e.contains("not connected") {
                    tracing::error!("Slack sync error: {}", e);
                    errors.push(format!("Slack: {}", e));
                }
            }
        }
        
        let duration_ms = start.elapsed().as_millis() as u64;
        
        let _ = app_handle.emit("sync:completed", serde_json::json!({
            "items_synced": total_items,
            "duration_ms": duration_ms,
            "errors": errors,
        }));
        
        tracing::info!("Sync cycle completed: {} items in {}ms", total_items, duration_ms);
    }
    
    /// Sync Slack data if credentials exist
    async fn sync_slack(db: Arc<Database>, crypto: Arc<CryptoService>) -> Result<i32, String> {
        sync_slack_now(db, crypto).await
    }
}

/// Public function to sync Slack data, can be called from commands
pub async fn sync_slack_now(db: Arc<Database>, crypto: Arc<CryptoService>) -> Result<i32, String> {
    // Check for Slack credentials
    let result: Option<(String,)> = sqlx::query_as(
        "SELECT encrypted_data FROM credentials WHERE id = 'slack'"
    )
    .fetch_optional(db.pool())
    .await
    .map_err(|e| e.to_string())?;
    
    let encrypted = result.ok_or("Slack not connected")?;
    
    let tokens_json = crypto
        .decrypt_string(&encrypted.0)
        .map_err(|e| e.to_string())?;
    let tokens: SlackTokens = serde_json::from_str(&tokens_json)
        .map_err(|e| e.to_string())?;
    
    tracing::info!("Starting Slack sync for team: {}", tokens.team_name);
    
    // Create Slack client and sync service with team_id for Enterprise Grid
    let client = SlackClient::new(String::new(), String::new())
        .with_token(tokens.access_token)
        .with_team_id(tokens.team_id);
    
    let sync_service = SlackSyncService::new(client, db, crypto);
    
    let result = sync_service.sync_all().await
        .map_err(|e| e.to_string())?;
    
    tracing::info!("Slack sync completed: {} items synced", result.items_synced);
    Ok(result.items_synced)
}
