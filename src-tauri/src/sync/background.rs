use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::sync::{Mutex, Notify};

use super::queue::SyncQueue;
use super::slack::{SlackClient, SlackSyncService, SlackTokens};
use crate::crypto::CryptoService;
use crate::db::Database;
use crate::pipeline::PipelineManager;

pub struct BackgroundSyncService {
    app_handle: AppHandle,
    db: Arc<Database>,
    crypto: Arc<CryptoService>,
    pipeline: Arc<tokio::sync::Mutex<PipelineManager>>,
    sync_lock: Arc<tokio::sync::Mutex<()>>,
    sync_queue: Arc<SyncQueue>,
    interval_minutes: Arc<Mutex<u64>>,
    is_running: Arc<AtomicBool>,
    is_syncing: Arc<AtomicBool>,
    next_sync_at: Arc<AtomicI64>,
    interval_changed: Arc<Notify>,
}

impl BackgroundSyncService {
    pub fn new(
        app_handle: AppHandle,
        db: Arc<Database>,
        crypto: Arc<CryptoService>,
        pipeline: Arc<tokio::sync::Mutex<PipelineManager>>,
        sync_lock: Arc<tokio::sync::Mutex<()>>,
        sync_queue: Arc<SyncQueue>,
        interval_minutes: u64,
    ) -> Self {
        Self {
            app_handle,
            db,
            crypto,
            pipeline,
            sync_lock,
            sync_queue,
            interval_minutes: Arc::new(Mutex::new(interval_minutes)),
            is_running: Arc::new(AtomicBool::new(false)),
            is_syncing: Arc::new(AtomicBool::new(false)),
            next_sync_at: Arc::new(AtomicI64::new(0)),
            interval_changed: Arc::new(Notify::new()),
        }
    }

    pub fn is_syncing(&self) -> bool {
        self.is_syncing.load(Ordering::SeqCst)
    }

    pub fn is_syncing_flag(&self) -> Arc<AtomicBool> {
        self.is_syncing.clone()
    }

    pub fn next_sync_at(&self) -> Option<i64> {
        let val = self.next_sync_at.load(Ordering::SeqCst);
        if val > 0 {
            Some(val)
        } else {
            None
        }
    }

    pub fn next_sync_at_flag(&self) -> Arc<AtomicI64> {
        self.next_sync_at.clone()
    }

    pub async fn set_interval(&self, interval_minutes: u64) {
        {
            let mut interval = self.interval_minutes.lock().await;
            *interval = interval_minutes;
        }
        tracing::info!(
            "Updated sync interval to {} minutes, resetting timer",
            interval_minutes
        );
        self.interval_changed.notify_one();
    }

    pub async fn check_sync_needed(&self) -> bool {
        let interval_minutes = *self.interval_minutes.lock().await;
        let last_sync = get_last_sync_at(self.db.clone()).await;

        match last_sync {
            Some(last_sync_ms) => {
                let now = chrono::Utc::now().timestamp_millis();
                let elapsed_minutes = (now - last_sync_ms) / 1000 / 60;
                elapsed_minutes >= interval_minutes as i64
            }
            None => true,
        }
    }

    pub async fn run_startup_sync_if_needed(&self) {
        if self.check_sync_needed().await {
            tracing::info!("Running startup sync");
            Self::run_sync_cycle(
                &self.app_handle,
                self.db.clone(),
                self.crypto.clone(),
                self.pipeline.clone(),
                self.sync_lock.clone(),
                self.sync_queue.clone(),
                self.is_syncing.clone(),
            )
            .await;
        }
    }

    pub fn start(&self) {
        if self.is_running.swap(true, Ordering::SeqCst) {
            return;
        }

        let app_handle = self.app_handle.clone();
        let db = self.db.clone();
        let crypto = self.crypto.clone();
        let pipeline = self.pipeline.clone();
        let sync_lock = self.sync_lock.clone();
        let sync_queue = self.sync_queue.clone();
        let interval_minutes = self.interval_minutes.clone();
        let is_running = self.is_running.clone();
        let is_syncing = self.is_syncing.clone();
        let next_sync_at = self.next_sync_at.clone();
        let interval_changed = self.interval_changed.clone();

        tokio::spawn(async move {
            loop {
                if !is_running.load(Ordering::SeqCst) {
                    break;
                }

                let interval = *interval_minutes.lock().await;
                let next_sync =
                    chrono::Utc::now().timestamp_millis() + (interval as i64 * 60 * 1000);
                next_sync_at.store(next_sync, Ordering::SeqCst);

                tokio::select! {
                    _ = tokio::time::sleep(Duration::from_secs(interval * 60)) => {
                        if !is_running.load(Ordering::SeqCst) {
                            break;
                        }
                        next_sync_at.store(0, Ordering::SeqCst);
                        Self::run_sync_cycle(&app_handle, db.clone(), crypto.clone(), pipeline.clone(), sync_lock.clone(), sync_queue.clone(), is_syncing.clone()).await;
                    }
                    _ = interval_changed.notified() => {
                        tracing::info!("Sync interval changed, resetting timer");
                        continue;
                    }
                }
            }
            next_sync_at.store(0, Ordering::SeqCst);
            tracing::info!("Background sync loop stopped");
        });

        tracing::info!("Background sync loop started");
    }

    pub fn stop(&self) {
        self.is_running.store(false, Ordering::SeqCst);
    }

    async fn run_sync_cycle(
        app_handle: &AppHandle,
        db: Arc<Database>,
        crypto: Arc<CryptoService>,
        pipeline: Arc<tokio::sync::Mutex<PipelineManager>>,
        sync_lock: Arc<tokio::sync::Mutex<()>>,
        sync_queue: Arc<SyncQueue>,
        is_syncing: Arc<AtomicBool>,
    ) {
        use crate::ai::ProcessingPipeline;
        use crate::pipeline::PipelineTaskType;
        use std::time::Instant;

        let Ok(_guard) = sync_lock.try_lock() else {
            tracing::debug!("Skipping background sync, another sync is in progress");
            return;
        };

        is_syncing.store(true, Ordering::SeqCst);
        tracing::info!("Starting background sync cycle");
        let start = Instant::now();
        let _ = app_handle.emit("sync:started", ());

        let mut total_items = 0;
        let mut errors: Vec<String> = Vec::new();

        let task_id = {
            let pipeline = pipeline.lock().await;
            pipeline
                .start_task(
                    PipelineTaskType::SyncSlack,
                    "Syncing Slack messages...".to_string(),
                )
                .await
        };

        match sync_slack_now(db.clone(), crypto.clone()).await {
            Ok(items) => {
                total_items = items;
                let pipeline = pipeline.lock().await;
                let message = if items > 0 {
                    format!("Synced {} messages from Slack", items)
                } else {
                    "Slack sync complete (no new messages)".to_string()
                };
                pipeline.complete_task(&task_id, Some(message)).await;
            }
            Err(e) => {
                let pipeline = pipeline.lock().await;
                if e.contains("not connected") {
                    pipeline
                        .complete_task(&task_id, Some("Slack not connected".to_string()))
                        .await;
                } else {
                    tracing::error!("Slack sync error: {}", e);
                    pipeline.fail_task(&task_id, e.clone()).await;
                    errors.push(format!("Slack: {}", e));
                }
            }
        }

        if total_items > 0 {
            if let Some(api_key_or_client) = get_gemini_client(db.clone(), crypto.clone()).await {
                let ai_task_id = {
                    let pipeline = pipeline.lock().await;
                    pipeline
                        .start_task(
                            PipelineTaskType::AiSummarize,
                            "Analyzing content with AI...".to_string(),
                        )
                        .await
                };

                let local_offset = chrono::Local::now().offset().local_minus_utc() / 60;
                let timezone_offset = Some(-local_offset);

                let ai_pipeline =
                    ProcessingPipeline::new(api_key_or_client, db.clone(), crypto.clone());
                match ai_pipeline.process_daily_batch(timezone_offset).await {
                    Ok(processed) => {
                        tracing::info!("AI batch processed {} groups/items", processed);
                        let pipeline = pipeline.lock().await;
                        pipeline
                            .complete_task(
                                &ai_task_id,
                                Some(format!("Summarized {} items", processed)),
                            )
                            .await;
                    }
                    Err(e) => {
                        tracing::error!("AI batch processing error: {}", e);
                        let pipeline = pipeline.lock().await;
                        pipeline.fail_task(&ai_task_id, e.clone()).await;
                        errors.push(format!("AI: {}", e));
                    }
                }
            }
        }

        let duration_ms = start.elapsed().as_millis() as u64;

        if let Err(e) = save_last_sync_at(db.clone()).await {
            tracing::error!("Failed to save last sync timestamp: {}", e);
        }

        is_syncing.store(false, Ordering::SeqCst);

        let _ = app_handle.emit(
            "sync:completed",
            serde_json::json!({
                "items_synced": total_items,
                "duration_ms": duration_ms,
                "errors": errors,
            }),
        );

        tracing::info!(
            "Background sync cycle completed: {} items in {}ms",
            total_items,
            duration_ms
        );

        // Drop the sync guard before draining the queue
        drop(_guard);

        // Drain any queued historical syncs
        Self::drain_queue(
            db,
            crypto,
            pipeline,
            sync_lock,
            sync_queue,
            is_syncing,
        )
        .await;
    }

    /// Drain the sync queue, executing each queued historical resync in order.
    async fn drain_queue(
        db: Arc<Database>,
        crypto: Arc<CryptoService>,
        pipeline: Arc<tokio::sync::Mutex<PipelineManager>>,
        sync_lock: Arc<tokio::sync::Mutex<()>>,
        sync_queue: Arc<SyncQueue>,
        is_syncing: Arc<AtomicBool>,
    ) {
        use crate::ai::ProcessingPipeline;
        use crate::pipeline::PipelineTaskType;

        loop {
            let request = sync_queue.dequeue().await;

            let Some(request) = request else {
                break;
            };

            let (Some(date), Some(tz_offset)) =
                (request.date.clone(), request.timezone_offset)
            else {
                tracing::warn!(
                    "Queued sync request missing date/timezone, skipping: {:?}",
                    request.source
                );
                continue;
            };

            tracing::info!("Background: draining queued historical resync for {}", date);

            let _sync_guard = match sync_lock.try_lock() {
                Ok(guard) => guard,
                Err(_) => {
                    tracing::info!(
                        "Sync lock busy during background queue drain, re-queuing {}",
                        date
                    );
                    sync_queue.enqueue(request).await;
                    break;
                }
            };

            is_syncing.store(true, Ordering::SeqCst);

            let mut total_items = 0;

            let sync_task_id = {
                let pipeline = pipeline.lock().await;
                pipeline
                    .start_task(
                        PipelineTaskType::SyncSlack,
                        format!("Syncing Slack messages for {}...", date),
                    )
                    .await
            };

            match sync_slack_historical_day(db.clone(), crypto.clone(), &date, tz_offset).await {
                Ok(result) => {
                    let items = result.items_synced;
                    tracing::info!(
                        "Background queued historical sync completed: {} items for {}",
                        items,
                        date
                    );
                    total_items = items;

                    let pipeline = pipeline.lock().await;
                    let message = if items > 0 {
                        format!("Synced {} messages for {}", items, date)
                    } else {
                        format!("No new messages found for {}", date)
                    };
                    pipeline.complete_task(&sync_task_id, Some(message)).await;
                }
                Err(e) => {
                    tracing::error!(
                        "Background queued historical sync error for {}: {}",
                        date,
                        e
                    );
                    let pipeline = pipeline.lock().await;
                    if e.contains("not connected") {
                        pipeline
                            .complete_task(
                                &sync_task_id,
                                Some("Slack not connected".to_string()),
                            )
                            .await;
                    } else {
                        pipeline.fail_task(&sync_task_id, e).await;
                    }
                }
            }

            if total_items > 0 {
                if let Some(api_key_or_client) =
                    get_gemini_client(db.clone(), crypto.clone()).await
                {
                    let ai_task_id = {
                        let pipeline = pipeline.lock().await;
                        pipeline
                            .start_task(
                                PipelineTaskType::AiSummarize,
                                format!("Analyzing content for {}...", date),
                            )
                            .await
                    };

                    let ai_pipeline = ProcessingPipeline::new(
                        api_key_or_client,
                        db.clone(),
                        crypto.clone(),
                    );
                    match ai_pipeline.process_batch_for_date(&date, tz_offset).await {
                        Ok(processed) => {
                            tracing::info!(
                                "AI batch processed {} groups/items for {} (background queued)",
                                processed,
                                date
                            );
                            let pipeline = pipeline.lock().await;
                            pipeline
                                .complete_task(
                                    &ai_task_id,
                                    Some(format!(
                                        "Grouped and summarized {} items",
                                        processed
                                    )),
                                )
                                .await;
                        }
                        Err(e) => {
                            tracing::error!(
                                "AI batch processing error for {} (background queued): {}",
                                date,
                                e
                            );
                            let pipeline = pipeline.lock().await;
                            pipeline.fail_task(&ai_task_id, e).await;
                        }
                    }
                }
            }

            is_syncing.store(false, Ordering::SeqCst);
            tracing::info!(
                "Background queued historical resync for {} completed",
                date
            );

            // Drop guard and yield to allow user-initiated syncs to interleave
            drop(_sync_guard);
            tokio::task::yield_now().await;
        }
    }
}

async fn get_gemini_client(db: Arc<Database>, crypto: Arc<CryptoService>) -> Option<String> {
    use crate::ai::ServiceAccountCredentials;

    let service_account: Option<(String,)> = sqlx::query_as(
        "SELECT encrypted_data FROM credentials WHERE id = 'gemini_service_account'",
    )
    .fetch_optional(db.pool())
    .await
    .ok()?;

    if let Some((encrypted_json,)) = service_account {
        if let Ok(json_content) = crypto.decrypt_string(&encrypted_json) {
            if let Ok(_credentials) =
                serde_json::from_str::<ServiceAccountCredentials>(&json_content)
            {
                return Some(format!("SERVICE_ACCOUNT:{}", json_content));
            }
        }
    }

    let api_key: Option<(String,)> =
        sqlx::query_as("SELECT encrypted_data FROM credentials WHERE id = 'gemini'")
            .fetch_optional(db.pool())
            .await
            .ok()?;

    if let Some((encrypted_key,)) = api_key {
        if let Ok(key) = crypto.decrypt_string(&encrypted_key) {
            return Some(key);
        }
    }

    None
}

pub async fn get_last_sync_at(db: Arc<Database>) -> Option<i64> {
    let result: Option<(String,)> =
        sqlx::query_as("SELECT value FROM preferences WHERE key = 'last_sync_at'")
            .fetch_optional(db.pool())
            .await
            .ok()?;

    result.and_then(|(value,)| value.parse::<i64>().ok())
}

async fn save_last_sync_at(db: Arc<Database>) -> Result<(), String> {
    let now = chrono::Utc::now().timestamp_millis();

    sqlx::query("INSERT OR REPLACE INTO preferences (key, value) VALUES ('last_sync_at', ?)")
        .bind(now.to_string())
        .execute(db.pool())
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Public function to sync Slack data, can be called from commands
pub async fn sync_slack_now(db: Arc<Database>, crypto: Arc<CryptoService>) -> Result<i32, String> {
    // Check for Slack credentials
    let result: Option<(String,)> =
        sqlx::query_as("SELECT encrypted_data FROM credentials WHERE id = 'slack'")
            .fetch_optional(db.pool())
            .await
            .map_err(|e| e.to_string())?;

    let encrypted = result.ok_or("Slack not connected")?;

    let tokens_json = crypto
        .decrypt_string(&encrypted.0)
        .map_err(|e| e.to_string())?;
    let mut tokens: SlackTokens = serde_json::from_str(&tokens_json).map_err(|e| e.to_string())?;

    tracing::info!("Starting Slack sync for team: {}", tokens.team_name);

    let client = SlackClient::new(String::new(), String::new())
        .with_token(tokens.access_token.clone())
        .with_team_id(tokens.team_id.clone());

    // Migrate tokens saved before team_domain was added
    if tokens.team_domain.is_none() {
        if let Ok(auth_info) = client.test_auth().await {
            if let Some(ref domain) = auth_info.team_domain {
                tokens.team_domain = Some(domain.clone());
                if let Ok(updated_json) = serde_json::to_string(&tokens) {
                    if let Ok(encrypted_data) = crypto.encrypt_string(&updated_json) {
                        let now = chrono::Utc::now().timestamp();
                        let _ = sqlx::query(
                            "UPDATE credentials SET encrypted_data = ?, updated_at = ? WHERE id = 'slack'"
                        )
                        .bind(&encrypted_data)
                        .bind(now)
                        .execute(db.pool())
                        .await;
                    }
                }
            }
        }
    }

    let sync_service =
        SlackSyncService::new(client, db.clone(), crypto).with_team_domain(tokens.team_domain);

    let result = sync_service.sync_all().await.map_err(|e| e.to_string())?;

    tracing::info!("Slack sync completed: {} items synced", result.items_synced);
    Ok(result.items_synced)
}

/// Sync Slack data for a specific historical date.
pub async fn sync_slack_historical_day(
    db: Arc<Database>,
    crypto: Arc<CryptoService>,
    date_str: &str,
    timezone_offset_minutes: i32,
) -> Result<crate::sync::SyncResult, String> {
    let result: Option<(String,)> =
        sqlx::query_as("SELECT encrypted_data FROM credentials WHERE id = 'slack'")
            .fetch_optional(db.pool())
            .await
            .map_err(|e| e.to_string())?;

    let encrypted = result.ok_or("Slack not connected")?;

    let tokens_json = crypto
        .decrypt_string(&encrypted.0)
        .map_err(|e| e.to_string())?;
    let tokens: SlackTokens = serde_json::from_str(&tokens_json).map_err(|e| e.to_string())?;

    tracing::info!(
        "Starting historical Slack sync for team: {}, date: {}",
        tokens.team_name,
        date_str
    );

    let client = SlackClient::new(String::new(), String::new())
        .with_token(tokens.access_token.clone())
        .with_team_id(tokens.team_id.clone());

    let sync_service =
        SlackSyncService::new(client, db.clone(), crypto).with_team_domain(tokens.team_domain);

    let result = sync_service
        .sync_historical_day(date_str, timezone_offset_minutes)
        .await
        .map_err(|e| e.to_string())?;

    tracing::info!(
        "Historical Slack sync completed for {}: {} items synced",
        date_str,
        result.items_synced
    );

    Ok(result)
}
