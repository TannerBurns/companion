use crate::AppState;
use crate::sync::{sync_slack_now, sync_slack_historical_day};
use crate::pipeline::PipelineTaskType;
use crate::ai::ProcessingPipeline;
use super::types::{SyncResult, SyncStatus};
use super::credentials::get_gemini_client;
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;

/// Trigger a sync operation for the specified sources
#[tauri::command]
pub async fn start_sync(
    state: State<'_, Arc<Mutex<AppState>>>,
    sources: Option<Vec<String>>,
    timezone_offset: Option<i32>,
) -> Result<SyncResult, String> {
    tracing::info!("Sync requested for sources: {:?}, timezone_offset: {:?}", sources, timezone_offset);
    
    let (db, crypto, pipeline, sync_lock, is_syncing) = {
        let state = state.lock().await;
        (
            state.db.clone(),
            std::sync::Arc::new(state.crypto.clone()),
            state.pipeline.clone(),
            state.sync_lock.clone(),
            state.is_syncing.clone(),
        )
    };
    
    let _sync_guard = match sync_lock.try_lock() {
        Ok(guard) => guard,
        Err(_) => {
            tracing::warn!("Sync already in progress, skipping duplicate request");
            return Ok(SyncResult {
                items_synced: 0,
                channels_processed: 0,
                errors: vec!["Sync already in progress".to_string()],
            });
        }
    };
    
    is_syncing.store(true, std::sync::atomic::Ordering::SeqCst);
    
    let mut total_items = 0;
    let mut channels_processed = 0;
    let mut errors: Vec<String> = Vec::new();
    
    // Determine which sources to sync
    let sync_slack = sources.as_ref().is_none_or(|s| s.contains(&"slack".to_string()));
    
    // Sync Slack
    if sync_slack {
        tracing::debug!("Starting Slack sync...");
        
        // Start pipeline task
        let task_id = {
            let pipeline = pipeline.lock().await;
            pipeline.start_task(PipelineTaskType::SyncSlack, "Syncing Slack messages...".to_string()).await
        };
        
        match sync_slack_now(db.clone(), crypto.clone()).await {
            Ok(items) => {
                tracing::info!("Slack sync completed: {} items", items);
                total_items += items;
                channels_processed += 1;
                
                // Complete pipeline task
                let pipeline = pipeline.lock().await;
                let message = if items > 0 {
                    format!("Synced {} messages from Slack", items)
                } else {
                    "Slack sync complete (no new messages)".to_string()
                };
                pipeline.complete_task(&task_id, Some(message)).await;
            }
            Err(e) => {
                tracing::error!("Slack sync error: {}", e);
                
                // Fail pipeline task
                let pipeline = pipeline.lock().await;
                if e.contains("not connected") {
                    pipeline.complete_task(&task_id, Some("Slack not connected".to_string())).await;
                } else {
                    pipeline.fail_task(&task_id, e.clone()).await;
                    errors.push(format!("Slack: {}", e));
                }
            }
        }
    }
    
    // Run AI batch processing to group and summarize content
    if total_items > 0 {
        tracing::info!("Running AI batch processing on {} new items...", total_items);
        
        // Try to get Gemini credentials (service account first, then API key)
        let gemini_client = get_gemini_client(db.clone(), crypto.clone()).await;
        
        if let Some(api_key_or_client) = gemini_client {
            // Start AI summarization task
            let task_id = {
                let pipeline = pipeline.lock().await;
                pipeline.start_task(PipelineTaskType::AiSummarize, "Analyzing and grouping content with AI...".to_string()).await
            };
            
            let ai_pipeline = ProcessingPipeline::new(api_key_or_client, db.clone(), crypto.clone());
            // Use batch processing to group related content across channels
            match ai_pipeline.process_daily_batch(timezone_offset).await {
                Ok(processed) => {
                    tracing::info!("AI batch processed {} groups/items", processed);
                    let pipeline = pipeline.lock().await;
                    pipeline.complete_task(&task_id, Some(format!("Grouped and summarized {} items", processed))).await;
                }
                Err(e) => {
                    tracing::error!("AI batch processing error: {}", e);
                    let pipeline = pipeline.lock().await;
                    pipeline.fail_task(&task_id, e.clone()).await;
                    errors.push(format!("AI: {}", e));
                }
            }
        } else {
            tracing::debug!("No Gemini credentials configured, skipping AI processing");
        }
    }
    
    let now = chrono::Utc::now().timestamp_millis();
    if let Err(e) = sqlx::query(
        "INSERT OR REPLACE INTO preferences (key, value) VALUES ('last_sync_at', ?)"
    )
    .bind(now.to_string())
    .execute(db.pool())
    .await
    {
        tracing::error!("Failed to save last sync timestamp: {}", e);
    }
    
    is_syncing.store(false, std::sync::atomic::Ordering::SeqCst);
    
    tracing::info!("Sync completed: items={}, errors={:?}", total_items, errors);
    Ok(SyncResult {
        items_synced: total_items,
        channels_processed,
        errors,
    })
}

/// Get the current sync status
#[tauri::command]
pub async fn get_sync_status(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<SyncStatus, String> {
    let (db, is_syncing, next_sync_at) = {
        let state = state.lock().await;
        (state.db.clone(), state.is_syncing.clone(), state.next_sync_at.clone())
    };
    
    let is_currently_syncing = is_syncing.load(std::sync::atomic::Ordering::SeqCst);
    let last_sync_at = crate::sync::get_last_sync_at(db).await;
    let next_sync = {
        let val = next_sync_at.load(std::sync::atomic::Ordering::SeqCst);
        if val > 0 { Some(val) } else { None }
    };
    
    Ok(SyncStatus {
        is_syncing: is_currently_syncing,
        last_sync_at,
        next_sync_at: next_sync,
        sources: vec![],
    })
}

/// Resync content for a specific historical date. Does not update the sync cursor.
#[tauri::command]
pub async fn resync_historical_day(
    state: State<'_, Arc<Mutex<AppState>>>,
    date: String,
    timezone_offset: i32,
) -> Result<SyncResult, String> {
    tracing::info!("Historical resync requested for date: {}", date);
    
    let (db, crypto, pipeline) = {
        let state = state.lock().await;
        (
            state.db.clone(),
            std::sync::Arc::new(state.crypto.clone()),
            state.pipeline.clone(),
        )
    };
    
    let mut total_items = 0;
    let mut errors: Vec<String> = Vec::new();
    
    let sync_task_id = {
        let pipeline = pipeline.lock().await;
        pipeline.start_task(
            PipelineTaskType::SyncSlack,
            format!("Syncing Slack messages for {}...", date),
        ).await
    };
    
    match sync_slack_historical_day(db.clone(), crypto.clone(), &date, timezone_offset).await {
        Ok(items) => {
            tracing::info!("Historical Slack sync completed: {} items for {}", items, date);
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
            tracing::error!("Historical Slack sync error for {}: {}", date, e);
            
            let pipeline = pipeline.lock().await;
            if e.contains("not connected") {
                pipeline.complete_task(&sync_task_id, Some("Slack not connected".to_string())).await;
            } else {
                pipeline.fail_task(&sync_task_id, e.clone()).await;
                errors.push(format!("Slack: {}", e));
            }
        }
    }
    
    if total_items > 0 {
        let gemini_client = get_gemini_client(db.clone(), crypto.clone()).await;
        
        if let Some(api_key_or_client) = gemini_client {
            let ai_task_id = {
                let pipeline = pipeline.lock().await;
                pipeline.start_task(
                    PipelineTaskType::AiSummarize,
                    format!("Analyzing content for {}...", date),
                ).await
            };
            
            let ai_pipeline = ProcessingPipeline::new(api_key_or_client, db.clone(), crypto.clone());
            match ai_pipeline.process_batch_for_date(&date, timezone_offset).await {
                Ok(processed) => {
                    tracing::info!("AI batch processed {} groups/items for {}", processed, date);
                    let pipeline = pipeline.lock().await;
                    pipeline.complete_task(
                        &ai_task_id,
                        Some(format!("Grouped and summarized {} items", processed)),
                    ).await;
                }
                Err(e) => {
                    tracing::error!("AI batch processing error for {}: {}", date, e);
                    let pipeline = pipeline.lock().await;
                    pipeline.fail_task(&ai_task_id, e.clone()).await;
                    errors.push(format!("AI: {}", e));
                }
            }
        } else {
            tracing::debug!("No Gemini credentials configured, skipping AI processing");
        }
    }
    
    tracing::info!("Historical resync for {} completed: items={}, errors={:?}", date, total_items, errors);
    
    Ok(SyncResult {
        items_synced: total_items,
        channels_processed: 1,
        errors,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_result_creation() {
        let result = SyncResult {
            items_synced: 10,
            channels_processed: 2,
            errors: vec![],
        };
        
        assert_eq!(result.items_synced, 10);
        assert_eq!(result.channels_processed, 2);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_sync_result_with_errors() {
        let result = SyncResult {
            items_synced: 0,
            channels_processed: 0,
            errors: vec!["Connection failed".to_string(), "Timeout".to_string()],
        };
        
        assert_eq!(result.errors.len(), 2);
        assert!(result.errors[0].contains("Connection"));
    }

    #[test]
    fn test_sync_status_defaults() {
        let status = SyncStatus {
            is_syncing: false,
            last_sync_at: None,
            next_sync_at: None,
            sources: vec![],
        };
        
        assert!(!status.is_syncing);
        assert!(status.last_sync_at.is_none());
        assert!(status.sources.is_empty());
    }

    #[test]
    fn test_source_detection() {
        // Test that source filtering works correctly
        let sources: Option<Vec<String>> = Some(vec!["slack".to_string()]);
        let sync_slack = sources.as_ref().is_none_or(|s| s.contains(&"slack".to_string()));
        assert!(sync_slack);
        
        let sources_none: Option<Vec<String>> = None;
        let sync_slack_none = sources_none.as_ref().is_none_or(|s| s.contains(&"slack".to_string()));
        assert!(sync_slack_none);
        
        let sources_other: Option<Vec<String>> = Some(vec!["atlassian".to_string()]);
        let sync_slack_other = sources_other.as_ref().is_none_or(|s| s.contains(&"slack".to_string()));
        assert!(!sync_slack_other);
    }

    #[test]
    fn test_timestamp_generation() {
        let now = chrono::Utc::now().timestamp_millis();
        assert!(now > 0);
        assert!(now > 1700000000000); // After 2023
    }
}
