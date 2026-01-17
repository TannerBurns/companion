use crate::AppState;
use crate::sync::{SlackClient, SlackTokens, AtlassianClient, AtlassianTokens, CloudResource};
use crate::pipeline::PipelineState;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DigestItem {
    pub id: String,
    pub title: String,
    pub summary: String,
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
pub struct CategorySummary {
    pub name: String,
    pub count: i32,
    pub top_items: Vec<DigestItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncStatus {
    pub is_syncing: bool,
    pub last_sync_at: Option<i64>,
    pub sources: Vec<SourceStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceStatus {
    pub name: String,
    pub status: String,
    pub items_synced: i32,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preferences {
    pub sync_interval_minutes: i32,
    pub enabled_sources: Vec<String>,
    pub enabled_categories: Vec<String>,
    pub notifications_enabled: bool,
}

#[tauri::command]
pub async fn get_daily_digest(
    _state: State<'_, Arc<Mutex<AppState>>>,
    date: Option<String>,
) -> Result<DigestResponse, String> {
    Ok(DigestResponse {
        date: date.unwrap_or_else(|| chrono::Utc::now().format("%Y-%m-%d").to_string()),
        items: vec![],
        categories: vec![],
    })
}

#[tauri::command]
pub async fn get_weekly_digest(
    _state: State<'_, Arc<Mutex<AppState>>>,
    week_start: Option<String>,
) -> Result<DigestResponse, String> {
    Ok(DigestResponse {
        date: week_start.unwrap_or_else(|| chrono::Utc::now().format("%Y-%m-%d").to_string()),
        items: vec![],
        categories: vec![],
    })
}

#[tauri::command]
pub async fn start_sync(
    _state: State<'_, Arc<Mutex<AppState>>>,
    sources: Option<Vec<String>>,
) -> Result<(), String> {
    tracing::info!("Sync requested for sources: {:?}", sources);
    Ok(())
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
pub async fn get_preferences(
    _state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Preferences, String> {
    Ok(Preferences {
        sync_interval_minutes: 15,
        enabled_sources: vec!["slack".to_string(), "jira".to_string(), "confluence".to_string()],
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
pub async fn connect_slack(
    state: State<'_, Arc<Mutex<AppState>>>,
    client_id: String,
    client_secret: String,
) -> Result<SlackTokens, String> {
    let client = SlackClient::new(client_id, client_secret);
    let tokens = client.start_oauth_flow().await.map_err(|e| e.to_string())?;
    
    // Store tokens
    let state = state.lock().await;
    let encrypted = state.crypto
        .encrypt_string(&serde_json::to_string(&tokens).unwrap())
        .map_err(|e| e.to_string())?;
    
    let now = chrono::Utc::now().timestamp();
    sqlx::query(
        "INSERT INTO credentials (id, service, encrypted_data, created_at, updated_at)
         VALUES ('slack', 'slack', ?, ?, ?)
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
    
    tracing::info!("Slack connected for team: {}", tokens.team_name);
    Ok(tokens)
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
