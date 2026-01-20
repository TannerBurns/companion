mod slack;
pub use slack::*;

use crate::AppState;
use crate::sync::{AtlassianClient, AtlassianTokens, CloudResource, sync_slack_now};
use crate::pipeline::{PipelineState, PipelineTaskType};
use crate::ai::{ProcessingPipeline, GeminiClient, ServiceAccountCredentials};
use chrono::Datelike;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;

type GroupRow = (String, String, Option<String>, Option<String>, Option<f64>, Option<String>, i64);

/// Helper function to get Gemini API key from either service account or API key credentials
async fn get_gemini_client(
    db: std::sync::Arc<crate::db::Database>,
    crypto: std::sync::Arc<crate::crypto::CryptoService>,
) -> Option<String> {
    // Try service account first
    let service_account: Option<(String,)> = sqlx::query_as(
        "SELECT encrypted_data FROM credentials WHERE id = 'gemini_service_account'"
    )
    .fetch_optional(db.pool())
    .await
    .ok()?;
    
    if let Some((encrypted_json,)) = service_account {
        if let Ok(json_content) = crypto.decrypt_string(&encrypted_json) {
            if let Ok(_credentials) = serde_json::from_str::<ServiceAccountCredentials>(&json_content) {
                // Return with prefix so ProcessingPipeline can identify auth type
                return Some(format!("SERVICE_ACCOUNT:{}", json_content));
            }
        }
    }
    
    // Try API key
    let api_key: Option<(String,)> = sqlx::query_as(
        "SELECT encrypted_data FROM credentials WHERE id = 'gemini'"
    )
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DigestItem {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub highlights: Option<Vec<String>>,
    pub category: String,
    pub source: String,
    pub source_url: Option<String>,
    pub importance_score: f64,
    pub created_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channels: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub people: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_count: Option<i32>,
}

/// Parsed entity metadata from AI summaries
struct ParsedEntities {
    title: String,
    channels: Option<Vec<String>>,
    people: Option<Vec<String>>,
    message_count: Option<i32>,
}

impl ParsedEntities {
    fn from_json(entities: &Option<String>) -> Self {
        let value: serde_json::Value = entities.as_ref()
            .and_then(|e| serde_json::from_str(e).ok())
            .unwrap_or(serde_json::json!({}));
        
        let title = value["topic"].as_str()
            .map(String::from)
            .unwrap_or_else(|| "Discussion".to_string());
        
        let channels: Option<Vec<String>> = value.get("channels")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .filter(|v: &Vec<String>| !v.is_empty());
        
        let people: Option<Vec<String>> = value.get("people")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .filter(|v: &Vec<String>| !v.is_empty());
        
        let message_count: Option<i32> = value.get("message_ids")
            .and_then(|v| v.as_array())
            .map(|arr| arr.len() as i32);
        
        Self { title, channels, people, message_count }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DigestResponse {
    pub date: String,
    pub items: Vec<DigestItem>,
    pub categories: Vec<CategorySummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CategorySummary {
    pub name: String,
    pub count: i32,
    pub top_items: Vec<DigestItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncStatus {
    pub is_syncing: bool,
    pub last_sync_at: Option<i64>,
    pub next_sync_at: Option<i64>,
    pub sources: Vec<SourceStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceStatus {
    pub name: String,
    pub status: String,
    pub items_synced: i32,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Preferences {
    pub sync_interval_minutes: i32,
    pub enabled_sources: Vec<String>,
    pub enabled_categories: Vec<String>,
    pub notifications_enabled: bool,
}

#[tauri::command]
pub async fn get_daily_digest(
    state: State<'_, Arc<Mutex<AppState>>>,
    date: Option<String>,
    timezone_offset: Option<i32>,
) -> Result<DigestResponse, String> {
    // JS getTimezoneOffset() returns minutes, positive for west of UTC
    let offset_minutes = timezone_offset.unwrap_or(0);
    
    let date_str = date.unwrap_or_else(|| {
        let now_utc = chrono::Utc::now();
        let offset = chrono::FixedOffset::west_opt(offset_minutes * 60).unwrap_or(chrono::FixedOffset::east_opt(0).unwrap());
        now_utc.with_timezone(&offset).format("%Y-%m-%d").to_string()
    });
    
    let db = {
        let state = state.lock().await;
        state.db.clone()
    };
    
    let parsed_date = chrono::NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")
        .map_err(|e| e.to_string())?;
    
    // Convert local midnight to UTC timestamp
    let local_offset = chrono::FixedOffset::west_opt(offset_minutes * 60)
        .unwrap_or(chrono::FixedOffset::east_opt(0).unwrap());
    
    let local_midnight = parsed_date
        .and_hms_opt(0, 0, 0)
        .ok_or("Invalid date")?
        .and_local_timezone(local_offset)
        .single()
        .ok_or("Ambiguous or invalid local time")?;
    
    let start_ts = local_midnight.with_timezone(&chrono::Utc).timestamp_millis();
    let end_ts = start_ts + 86400 * 1000; // 24 hours
    
    // Fetch group summaries (cross-channel grouped content)
    let groups: Vec<GroupRow> = sqlx::query_as(
        "SELECT id, summary, highlights, category, importance_score, entities, generated_at
         FROM ai_summaries 
         WHERE summary_type = 'group' AND generated_at >= ? AND generated_at < ?
         ORDER BY importance_score DESC"
    )
    .bind(start_ts)
    .bind(end_ts)
    .fetch_all(db.pool())
    .await
    .map_err(|e| e.to_string())?;
    
    let mut items: Vec<DigestItem> = Vec::new();
    let mut category_counts: std::collections::HashMap<String, (i32, Vec<DigestItem>)> = std::collections::HashMap::new();
    
    for (id, summary, highlights_json, category, importance_score, entities, generated_at) in groups {
        let parsed = ParsedEntities::from_json(&entities);
        let highlights: Option<Vec<String>> = highlights_json
            .and_then(|h| serde_json::from_str(&h).ok());
        let cat = category.clone().unwrap_or_else(|| "other".to_string());
        
        let item = DigestItem {
            id: id.clone(),
            title: parsed.title,
            summary: summary.clone(),
            highlights,
            category: cat.clone(),
            source: "slack".to_string(),
            source_url: None,
            importance_score: importance_score.unwrap_or(0.5),
            created_at: generated_at,
            channels: parsed.channels,
            people: parsed.people,
            message_count: parsed.message_count,
        };
        
        let entry = category_counts.entry(cat.clone()).or_insert((0, vec![]));
        entry.0 += 1;
        if entry.1.len() < 3 {
            entry.1.push(item.clone());
        }
        
        items.push(item);
    }
    
    let daily_summary: Option<(String, Option<String>)> = sqlx::query_as(
        "SELECT summary, highlights FROM ai_summaries 
         WHERE summary_type = 'daily' AND generated_at >= ? AND generated_at < ?
         ORDER BY generated_at DESC LIMIT 1"
    )
    .bind(start_ts)
    .bind(end_ts)
    .fetch_optional(db.pool())
    .await
    .map_err(|e| e.to_string())?;
    
    if let Some((summary, highlights_json)) = daily_summary {
        let highlights: Option<Vec<String>> = highlights_json
            .and_then(|h| serde_json::from_str(&h).ok());
        
        items.insert(0, DigestItem {
            id: "daily-summary".to_string(),
            title: "Today's Overview".to_string(),
            summary,
            highlights,
            category: "overview".to_string(),
            source: "ai".to_string(),
            source_url: None,
            importance_score: 1.0,
            created_at: start_ts,
            channels: None,
            people: None,
            message_count: None,
        });
    }
    
    let categories: Vec<CategorySummary> = category_counts
        .into_iter()
        .map(|(name, (count, top_items))| CategorySummary {
            name,
            count,
            top_items,
        })
        .collect();
    
    Ok(DigestResponse {
        date: date_str,
        items,
        categories,
    })
}

#[tauri::command]
pub async fn get_weekly_digest(
    state: State<'_, Arc<Mutex<AppState>>>,
    week_start: Option<String>,
    timezone_offset: Option<i32>,
) -> Result<DigestResponse, String> {
    let offset_minutes = timezone_offset.unwrap_or(0);
    let local_offset = chrono::FixedOffset::west_opt(offset_minutes * 60)
        .unwrap_or(chrono::FixedOffset::east_opt(0).unwrap());
    
    let db = {
        let state = state.lock().await;
        state.db.clone()
    };
    
    // Calculate week start (Monday) if not provided
    let now_utc = chrono::Utc::now();
    let today_local = now_utc.with_timezone(&local_offset).date_naive();
    
    let week_start_date = if let Some(ref ws) = week_start {
        chrono::NaiveDate::parse_from_str(ws, "%Y-%m-%d")
            .map_err(|e| e.to_string())?
    } else {
        // Get Monday of current week in local timezone
        let days_since_monday = today_local.weekday().num_days_from_monday();
        today_local - chrono::Duration::days(days_since_monday as i64)
    };
    
    let week_start_str = week_start_date.format("%Y-%m-%d").to_string();
    
    let local_midnight = week_start_date
        .and_hms_opt(0, 0, 0)
        .ok_or("Invalid date")?
        .and_local_timezone(local_offset)
        .single()
        .ok_or("Ambiguous or invalid local time")?;
    
    let start_ts = local_midnight.with_timezone(&chrono::Utc).timestamp_millis();
    let end_ts = start_ts + 7 * 86400 * 1000;
    
    let groups: Vec<GroupRow> = sqlx::query_as(
        "SELECT id, summary, highlights, category, importance_score, entities, generated_at
         FROM ai_summaries 
         WHERE summary_type = 'group' AND generated_at >= ? AND generated_at < ?
         ORDER BY importance_score DESC"
    )
    .bind(start_ts)
    .bind(end_ts)
    .fetch_all(db.pool())
    .await
    .map_err(|e| e.to_string())?;
    
    let mut items: Vec<DigestItem> = Vec::new();
    let mut category_counts: std::collections::HashMap<String, (i32, Vec<DigestItem>)> = std::collections::HashMap::new();
    
    for (id, summary, highlights_json, category, importance_score, entities, generated_at) in groups {
        let parsed = ParsedEntities::from_json(&entities);
        let highlights: Option<Vec<String>> = highlights_json
            .and_then(|h| serde_json::from_str(&h).ok());
        let cat = category.clone().unwrap_or_else(|| "other".to_string());
        
        let item = DigestItem {
            id: id.clone(),
            title: parsed.title,
            summary: summary.clone(),
            highlights,
            category: cat.clone(),
            source: "slack".to_string(),
            source_url: None,
            importance_score: importance_score.unwrap_or(0.5),
            created_at: generated_at,
            channels: parsed.channels,
            people: parsed.people,
            message_count: parsed.message_count,
        };
        
        let entry = category_counts.entry(cat.clone()).or_insert((0, vec![]));
        entry.0 += 1;
        if entry.1.len() < 3 {
            entry.1.push(item.clone());
        }
        
        items.push(item);
    }
    
    // Fetch individual daily summaries with their timestamps
    let daily_summaries: Vec<(String, String, Option<String>, i64)> = sqlx::query_as(
        "SELECT id, summary, highlights, generated_at FROM ai_summaries 
         WHERE summary_type = 'daily' AND generated_at >= ? AND generated_at < ?
         ORDER BY generated_at DESC"
    )
    .bind(start_ts)
    .bind(end_ts)
    .fetch_all(db.pool())
    .await
    .map_err(|e| e.to_string())?;
    
    // Add individual daily overview items with their actual dates
    for (id, summary, highlights_json, generated_at) in &daily_summaries {
        let highlights: Option<Vec<String>> = highlights_json
            .as_ref()
            .and_then(|h| serde_json::from_str(h).ok());
        
        // Convert generated_at to a date string for the title
        let date_display = chrono::DateTime::from_timestamp_millis(*generated_at)
            .map(|dt| dt.with_timezone(&local_offset).format("%A, %b %d").to_string())
            .unwrap_or_else(|| "Daily".to_string());
        
        items.push(DigestItem {
            id: id.clone(),
            title: format!("{} Overview", date_display),
            summary: summary.clone(),
            highlights,
            category: "overview".to_string(),
            source: "ai".to_string(),
            source_url: None,
            importance_score: 0.95, // High but below 1.0 so they sort after group items
            created_at: *generated_at,
            channels: None,
            people: None,
            message_count: None,
        });
    }
    
    let categories: Vec<CategorySummary> = category_counts
        .into_iter()
        .map(|(name, (count, top_items))| CategorySummary {
            name,
            count,
            top_items,
        })
        .collect();
    
    Ok(DigestResponse {
        date: week_start_str,
        items,
        categories,
    })
}

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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncResult {
    pub items_synced: i32,
    pub channels_processed: i32,
    pub errors: Vec<String>,
}

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

#[tauri::command]
pub async fn save_api_key(
    state: State<'_, Arc<Mutex<AppState>>>,
    service: String,
    api_key: String,
) -> Result<(), String> {
    let state = state.lock().await;
    
    let encrypted = state.crypto
        .encrypt_string(&api_key)
        .map_err(|e| e.to_string())?;
    
    let now = chrono::Utc::now().timestamp();
    sqlx::query(
        "INSERT INTO credentials (id, service, encrypted_data, created_at, updated_at) 
         VALUES (?, ?, ?, ?, ?)
         ON CONFLICT(id) DO UPDATE SET encrypted_data = ?, updated_at = ?"
    )
    .bind(&service)
    .bind(&service)
    .bind(&encrypted)
    .bind(now)
    .bind(now)
    .bind(&encrypted)
    .bind(now)
    .execute(state.db.pool())
    .await
    .map_err(|e| e.to_string())?;
    
    // When saving a Gemini API key, delete any existing service account credentials
    // so the API key takes priority (get_gemini_client checks service account first)
    if service == "gemini" {
        sqlx::query("DELETE FROM credentials WHERE id = 'gemini_service_account'")
            .execute(state.db.pool())
            .await
            .map_err(|e| e.to_string())?;
    }
    
    tracing::info!("Saved encrypted API key for service: {}", service);
    Ok(())
}

#[tauri::command]
pub async fn has_api_key(
    state: State<'_, Arc<Mutex<AppState>>>,
    service: String,
) -> Result<bool, String> {
    let state = state.lock().await;
    
    let result: Option<(String,)> = sqlx::query_as(
        "SELECT encrypted_data FROM credentials WHERE id = ?"
    )
    .bind(&service)
    .fetch_optional(state.db.pool())
    .await
    .map_err(|e| e.to_string())?;
    
    Ok(result.is_some())
}

#[tauri::command]
pub async fn save_gemini_credentials(
    state: State<'_, Arc<Mutex<AppState>>>,
    json_content: String,
    region: Option<String>,
) -> Result<(), String> {
    let mut credentials: ServiceAccountCredentials = serde_json::from_str(&json_content)
        .map_err(|e| format!("Invalid service account JSON: {}", e))?;
    
    if let Some(r) = region {
        if !r.is_empty() {
            credentials.vertex_region = Some(r);
        }
    }
    
    let json_with_region = serde_json::to_string(&credentials)
        .map_err(|e| format!("Failed to serialize credentials: {}", e))?;
    
    let state = state.lock().await;
    
    let encrypted = state.crypto
        .encrypt_string(&json_with_region)
        .map_err(|e| e.to_string())?;
    
    let now = chrono::Utc::now().timestamp();
    
    sqlx::query(
        "INSERT INTO credentials (id, service, encrypted_data, created_at, updated_at) 
         VALUES ('gemini_service_account', 'gemini', ?, ?, ?)
         ON CONFLICT(id) DO UPDATE SET encrypted_data = ?, updated_at = ?"
    )
    .bind(&encrypted)
    .bind(now)
    .bind(now)
    .bind(&encrypted)
    .bind(now)
    .execute(state.db.pool())
    .await
    .map_err(|e| e.to_string())?;
    
    sqlx::query("DELETE FROM credentials WHERE id = 'gemini'")
        .execute(state.db.pool())
        .await
        .map_err(|e| e.to_string())?;
    
    tracing::info!("Saved encrypted Gemini service account credentials (region: {})", 
        credentials.region());
    Ok(())
}

/// Verify Gemini connection works with current credentials
#[tauri::command]
pub async fn verify_gemini_connection(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<(), String> {
    let (db, crypto) = {
        let state = state.lock().await;
        (state.db.clone(), state.crypto.clone())
    };
    
    // Try service account first
    let service_account: Option<(String,)> = sqlx::query_as(
        "SELECT encrypted_data FROM credentials WHERE id = 'gemini_service_account'"
    )
    .fetch_optional(db.pool())
    .await
    .map_err(|e| e.to_string())?;
    
    if let Some((encrypted_json,)) = service_account {
        tracing::info!("Verifying Gemini connection with service account...");
        
        let json_content = crypto.decrypt_string(&encrypted_json)
            .map_err(|e| format!("Failed to decrypt credentials: {}", e))?;
        
        let credentials: ServiceAccountCredentials = serde_json::from_str(&json_content)
            .map_err(|e| format!("Invalid service account JSON: {}", e))?;
        
        tracing::info!("Using service account: {}", credentials.client_email);
        
        let client = GeminiClient::new_with_service_account(credentials);
        client.verify_connection().await
            .map_err(|e| e.to_string())?;
        
        return Ok(());
    }
    
    // Try API key
    let api_key: Option<(String,)> = sqlx::query_as(
        "SELECT encrypted_data FROM credentials WHERE id = 'gemini'"
    )
    .fetch_optional(db.pool())
    .await
    .map_err(|e| e.to_string())?;
    
    if let Some((encrypted_key,)) = api_key {
        tracing::info!("Verifying Gemini connection with API key...");
        
        let key = crypto.decrypt_string(&encrypted_key)
            .map_err(|e| format!("Failed to decrypt API key: {}", e))?;
        
        let client = GeminiClient::new(key);
        client.verify_connection().await
            .map_err(|e| e.to_string())?;
        
        return Ok(());
    }
    
    Err("No Gemini credentials configured".to_string())
}

/// Get the current Gemini authentication type
#[tauri::command]
pub async fn get_gemini_auth_type(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<String, String> {
    let state = state.lock().await;
    
    // Check for service account first
    let service_account: Option<(String,)> = sqlx::query_as(
        "SELECT encrypted_data FROM credentials WHERE id = 'gemini_service_account'"
    )
    .fetch_optional(state.db.pool())
    .await
    .map_err(|e| e.to_string())?;
    
    if service_account.is_some() {
        return Ok("service_account".to_string());
    }
    
    // Check for API key
    let api_key: Option<(String,)> = sqlx::query_as(
        "SELECT encrypted_data FROM credentials WHERE id = 'gemini'"
    )
    .fetch_optional(state.db.pool())
    .await
    .map_err(|e| e.to_string())?;
    
    if api_key.is_some() {
        return Ok("api_key".to_string());
    }
    
    Ok("none".to_string())
}

#[tauri::command]
pub async fn get_preferences(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Preferences, String> {
    let state = state.lock().await;
    
    let result: Option<(String,)> = sqlx::query_as(
        "SELECT value FROM preferences WHERE key = 'user_preferences'"
    )
    .fetch_optional(state.db.pool())
    .await
    .map_err(|e| e.to_string())?;
    
    match result {
        Some((json,)) => {
            serde_json::from_str(&json).map_err(|e| e.to_string())
        }
        None => {
            // Return defaults if no preferences saved yet
            Ok(Preferences {
                sync_interval_minutes: 15,
                enabled_sources: vec![],
                enabled_categories: vec![
                    "sales".to_string(),
                    "marketing".to_string(),
                    "product".to_string(),
                    "engineering".to_string(),
                    "research".to_string(),
                ],
                notifications_enabled: true,
            })
        }
    }
}

#[tauri::command]
pub async fn save_preferences(
    state: State<'_, Arc<Mutex<AppState>>>,
    preferences: Preferences,
) -> Result<(), String> {
    let (db, background_sync) = {
        let state = state.lock().await;
        (state.db.clone(), state.background_sync.clone())
    };
    
    let prefs_json = serde_json::to_string(&preferences).map_err(|e| e.to_string())?;
    
    sqlx::query("INSERT OR REPLACE INTO preferences (key, value) VALUES ('user_preferences', ?)")
        .bind(&prefs_json)
        .execute(db.pool())
        .await
        .map_err(|e| e.to_string())?;
    
    if let Some(bg_sync) = background_sync {
        bg_sync.set_interval(preferences.sync_interval_minutes as u64).await;
    }
    
    Ok(())
}

#[tauri::command]
pub async fn connect_atlassian(
    state: State<'_, Arc<Mutex<AppState>>>,
    client_id: String,
    client_secret: String,
) -> Result<(AtlassianTokens, Vec<CloudResource>), String> {
    let client = AtlassianClient::new(client_id, client_secret);
    let (tokens, resources) = client.start_oauth_flow().await.map_err(|e| e.to_string())?;
    
    // Store tokens
    let state = state.lock().await;
    let encrypted = state.crypto
        .encrypt_string(&serde_json::to_string(&tokens).unwrap())
        .map_err(|e| e.to_string())?;
    
    let now = chrono::Utc::now().timestamp();
    sqlx::query(
        "INSERT INTO credentials (id, service, encrypted_data, created_at, updated_at)
         VALUES ('atlassian', 'atlassian', ?, ?, ?)
         ON CONFLICT(id) DO UPDATE SET encrypted_data = ?, updated_at = ?"
    )
    .bind(&encrypted)
    .bind(now)
    .bind(now)
    .bind(&encrypted)
    .bind(now)
    .execute(state.db.pool())
    .await
    .map_err(|e| e.to_string())?;
    
    tracing::info!("Atlassian connected with {} cloud resources", resources.len());
    Ok((tokens, resources))
}

#[tauri::command]
pub async fn select_atlassian_resource(
    state: State<'_, Arc<Mutex<AppState>>>,
    cloud_id: String,
) -> Result<(), String> {
    let state = state.lock().await;
    
    sqlx::query(
        "INSERT INTO preferences (key, value) VALUES ('atlassian_cloud_id', ?)
         ON CONFLICT(key) DO UPDATE SET value = ?"
    )
    .bind(&cloud_id)
    .bind(&cloud_id)
    .execute(state.db.pool())
    .await
    .map_err(|e| e.to_string())?;
    
    tracing::info!("Selected Atlassian cloud resource: {}", cloud_id);
    Ok(())
}

#[tauri::command]
pub async fn track_event(
    state: State<'_, Arc<Mutex<AppState>>>,
    event_type: String,
    event_data: serde_json::Value,
) -> Result<(), String> {
    let state = state.lock().await;
    
    let now = chrono::Utc::now().timestamp();
    sqlx::query(
        "INSERT INTO analytics (event_type, event_data, created_at) VALUES (?, ?, ?)"
    )
    .bind(&event_type)
    .bind(serde_json::to_string(&event_data).unwrap_or_default())
    .bind(now)
    .execute(state.db.pool())
    .await
    .map_err(|e| e.to_string())?;

    tracing::debug!("Tracked event: {}", event_type);
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
pub struct AnalyticsSummary {
    pub event_counts: std::collections::HashMap<String, i64>,
    pub days: i32,
}

#[tauri::command]
pub async fn get_analytics_summary(
    state: State<'_, Arc<Mutex<AppState>>>,
    days: i32,
) -> Result<AnalyticsSummary, String> {
    let state = state.lock().await;
    let since = chrono::Utc::now().timestamp() - (days as i64 * 86400);

    let counts: Vec<(String, i64)> = sqlx::query_as(
        "SELECT event_type, COUNT(*) FROM analytics WHERE created_at >= ? GROUP BY event_type"
    )
    .bind(since)
    .fetch_all(state.db.pool())
    .await
    .map_err(|e| e.to_string())?;

    let event_counts: std::collections::HashMap<_, _> = counts.into_iter().collect();
    Ok(AnalyticsSummary { event_counts, days })
}

#[tauri::command]
pub async fn get_pipeline_status(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<PipelineState, String> {
    let state = state.lock().await;
    let pipeline = state.pipeline.lock().await;
    Ok(pipeline.get_state().await)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataStats {
    pub content_items: i64,
    pub ai_summaries: i64,
    pub slack_users: i64,
    pub sync_states: i64,
}

#[tauri::command]
pub async fn get_data_stats(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<DataStats, String> {
    let db = {
        let state = state.lock().await;
        state.db.clone()
    };
    
    let content_items: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM content_items")
        .fetch_one(db.pool())
        .await
        .map_err(|e| e.to_string())?;
    
    let ai_summaries: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM ai_summaries")
        .fetch_one(db.pool())
        .await
        .map_err(|e| e.to_string())?;
    
    let slack_users: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM slack_users")
        .fetch_one(db.pool())
        .await
        .unwrap_or((0,)); // Table might not exist yet
    
    let sync_states: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM sync_state")
        .fetch_one(db.pool())
        .await
        .map_err(|e| e.to_string())?;
    
    Ok(DataStats {
        content_items: content_items.0,
        ai_summaries: ai_summaries.0,
        slack_users: slack_users.0,
        sync_states: sync_states.0,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClearDataResult {
    pub items_deleted: i64,
}

#[tauri::command]
pub async fn clear_synced_data(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<ClearDataResult, String> {
    let db = {
        let state = state.lock().await;
        state.db.clone()
    };
    
    let content_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM content_items")
        .fetch_one(db.pool())
        .await
        .map_err(|e| e.to_string())?;
    
    let summary_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM ai_summaries")
        .fetch_one(db.pool())
        .await
        .map_err(|e| e.to_string())?;
    
    sqlx::query("DELETE FROM content_items")
        .execute(db.pool())
        .await
        .map_err(|e| e.to_string())?;
    
    sqlx::query("DELETE FROM ai_summaries")
        .execute(db.pool())
        .await
        .map_err(|e| e.to_string())?;
    
    sqlx::query("DELETE FROM sync_state")
        .execute(db.pool())
        .await
        .map_err(|e| e.to_string())?;
    
    sqlx::query("DELETE FROM slack_users")
        .execute(db.pool())
        .await
        .ok();
    
    sqlx::query("DELETE FROM preferences WHERE key = 'last_sync_at'")
        .execute(db.pool())
        .await
        .map_err(|e| e.to_string())?;
    
    let total_deleted = content_count.0 + summary_count.0;
    tracing::info!("Cleared synced data: {} content items, {} summaries", content_count.0, summary_count.0);
    
    Ok(ClearDataResult {
        items_deleted: total_deleted,
    })
}

#[tauri::command]
pub async fn factory_reset(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<(), String> {
    let db = {
        let state = state.lock().await;
        state.db.clone()
    };
    
    sqlx::query("DELETE FROM content_items")
        .execute(db.pool())
        .await
        .map_err(|e| e.to_string())?;
    
    sqlx::query("DELETE FROM ai_summaries")
        .execute(db.pool())
        .await
        .map_err(|e| e.to_string())?;
    
    sqlx::query("DELETE FROM sync_state")
        .execute(db.pool())
        .await
        .map_err(|e| e.to_string())?;
    
    sqlx::query("DELETE FROM slack_users")
        .execute(db.pool())
        .await
        .ok();
    
    sqlx::query("DELETE FROM credentials")
        .execute(db.pool())
        .await
        .map_err(|e| e.to_string())?;
    
    sqlx::query("DELETE FROM slack_selected_channels")
        .execute(db.pool())
        .await
        .map_err(|e| e.to_string())?;
    
    sqlx::query("DELETE FROM preferences")
        .execute(db.pool())
        .await
        .map_err(|e| e.to_string())?;
    
    sqlx::query("DELETE FROM analytics")
        .execute(db.pool())
        .await
        .map_err(|e| e.to_string())?;
    
    tracing::info!("Factory reset complete - all data cleared");
    
    Ok(())
}
