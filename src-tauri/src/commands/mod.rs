use crate::AppState;
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
