mod slack;
pub use slack::*;

use crate::AppState;
use crate::sync::{AtlassianClient, AtlassianTokens, CloudResource, sync_slack_now};
use crate::pipeline::{PipelineState, PipelineTaskType};
use crate::ai::ProcessingPipeline;
use chrono::Datelike;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;

type GroupRow = (String, String, Option<String>, Option<String>, Option<f64>, Option<String>, i64);

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
) -> Result<DigestResponse, String> {
    let date_str = date.unwrap_or_else(|| chrono::Utc::now().format("%Y-%m-%d").to_string());
    
    let db = {
        let state = state.lock().await;
        state.db.clone()
    };
    
    // Parse date and get timestamp range
    let parsed_date = chrono::NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")
        .map_err(|e| e.to_string())?;
    let start_ts = parsed_date
        .and_hms_opt(0, 0, 0)
        .ok_or("Invalid date")?
        .and_utc()
        .timestamp_millis();
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
        let title = entities.as_ref()
            .and_then(|e| serde_json::from_str::<serde_json::Value>(e).ok())
            .and_then(|v| v["topic"].as_str().map(String::from))
            .unwrap_or_else(|| "Discussion".to_string());
        
        let highlights: Option<Vec<String>> = highlights_json
            .and_then(|h| serde_json::from_str(&h).ok());
        
        let cat = category.clone().unwrap_or_else(|| "other".to_string());
        
        let item = DigestItem {
            id: id.clone(),
            title,
            summary: summary.clone(),
            highlights,
            category: cat.clone(),
            source: "slack".to_string(),
            source_url: None,
            importance_score: importance_score.unwrap_or(0.5),
            created_at: generated_at,
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
) -> Result<DigestResponse, String> {
    let db = {
        let state = state.lock().await;
        state.db.clone()
    };
    
    // Calculate week start (Monday) if not provided
    let today = chrono::Utc::now().date_naive();
    let week_start_date = if let Some(ref ws) = week_start {
        chrono::NaiveDate::parse_from_str(ws, "%Y-%m-%d")
            .map_err(|e| e.to_string())?
    } else {
        // Get Monday of current week
        let days_since_monday = today.weekday().num_days_from_monday();
        today - chrono::Duration::days(days_since_monday as i64)
    };
    
    let week_start_str = week_start_date.format("%Y-%m-%d").to_string();
    
    let start_ts = week_start_date
        .and_hms_opt(0, 0, 0)
        .ok_or("Invalid date")?
        .and_utc()
        .timestamp_millis();
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
        let title = entities.as_ref()
            .and_then(|e| serde_json::from_str::<serde_json::Value>(e).ok())
            .and_then(|v| v["topic"].as_str().map(String::from))
            .unwrap_or_else(|| "Discussion".to_string());
        
        let highlights: Option<Vec<String>> = highlights_json
            .and_then(|h| serde_json::from_str(&h).ok());
        
        let cat = category.clone().unwrap_or_else(|| "other".to_string());
        
        let item = DigestItem {
            id: id.clone(),
            title,
            summary: summary.clone(),
            highlights,
            category: cat.clone(),
            source: "slack".to_string(),
            source_url: None,
            importance_score: importance_score.unwrap_or(0.5),
            created_at: generated_at,
        };
        
        let entry = category_counts.entry(cat.clone()).or_insert((0, vec![]));
        entry.0 += 1;
        if entry.1.len() < 3 {
            entry.1.push(item.clone());
        }
        
        items.push(item);
    }
    
    let daily_summaries: Vec<(String, Option<String>)> = sqlx::query_as(
        "SELECT summary, highlights FROM ai_summaries 
         WHERE summary_type = 'daily' AND generated_at >= ? AND generated_at < ?
         ORDER BY generated_at DESC"
    )
    .bind(start_ts)
    .bind(end_ts)
    .fetch_all(db.pool())
    .await
    .map_err(|e| e.to_string())?;
    
    if !daily_summaries.is_empty() {
        let combined_summary = daily_summaries
            .iter()
            .map(|(s, _)| s.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        
        let mut all_themes: Vec<String> = Vec::new();
        for (_, highlights_json) in &daily_summaries {
            if let Some(h) = highlights_json {
                if let Ok(themes) = serde_json::from_str::<Vec<String>>(h) {
                    all_themes.extend(themes);
                }
            }
        }
        all_themes.sort();
        all_themes.dedup();
        
        items.insert(0, DigestItem {
            id: "weekly-summary".to_string(),
            title: format!("Week of {}", week_start_str),
            summary: combined_summary,
            highlights: if all_themes.is_empty() { None } else { Some(all_themes) },
            category: "overview".to_string(),
            source: "ai".to_string(),
            source_url: None,
            importance_score: 1.0,
            created_at: start_ts,
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
) -> Result<SyncResult, String> {
    tracing::info!("Sync requested for sources: {:?}", sources);
    
    let (db, crypto, pipeline) = {
        let state = state.lock().await;
        (
            state.db.clone(),
            std::sync::Arc::new(state.crypto.clone()),
            state.pipeline.clone(),
        )
    };
    
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
        
        // Check for Gemini API key
        let gemini_key: Option<(String,)> = sqlx::query_as(
            "SELECT encrypted_data FROM credentials WHERE id = 'gemini'"
        )
        .fetch_optional(db.pool())
        .await
        .ok()
        .flatten();
        
        if let Some((encrypted_key,)) = gemini_key {
            if let Ok(api_key) = crypto.decrypt_string(&encrypted_key) {
                // Start AI summarization task
                let task_id = {
                    let pipeline = pipeline.lock().await;
                    pipeline.start_task(PipelineTaskType::AiSummarize, "Analyzing and grouping content with AI...".to_string()).await
                };
                
                let ai_pipeline = ProcessingPipeline::new(api_key, db.clone(), crypto.clone());
                // Use batch processing to group related content across channels
                match ai_pipeline.process_daily_batch().await {
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
                tracing::error!("Failed to decrypt Gemini API key");
            }
        } else {
            tracing::debug!("No Gemini API key configured, skipping AI processing");
        }
    }
    
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
    _state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<SyncStatus, String> {
    Ok(SyncStatus {
        is_syncing: false,
        last_sync_at: None,
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
    let state = state.lock().await;
    let prefs_json = serde_json::to_string(&preferences).map_err(|e| e.to_string())?;
    
    sqlx::query("INSERT OR REPLACE INTO preferences (key, value) VALUES ('user_preferences', ?)")
        .bind(&prefs_json)
        .execute(state.db.pool())
        .await
        .map_err(|e| e.to_string())?;
    
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
