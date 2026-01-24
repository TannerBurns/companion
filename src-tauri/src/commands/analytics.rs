use crate::AppState;
use super::types::AnalyticsSummary;
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;

/// Track an analytics event
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

/// Get analytics summary for a time period
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analytics_summary_creation() {
        let mut event_counts = std::collections::HashMap::new();
        event_counts.insert("page_view".to_string(), 100);
        event_counts.insert("sync_triggered".to_string(), 25);
        
        let summary = AnalyticsSummary {
            event_counts,
            days: 7,
        };
        
        assert_eq!(summary.days, 7);
        assert_eq!(summary.event_counts.get("page_view"), Some(&100));
    }

    #[test]
    fn test_timestamp_calculation() {
        let days = 7i32;
        let now = chrono::Utc::now().timestamp();
        let since = now - (days as i64 * 86400);
        
        // Should be exactly 7 days ago in seconds
        assert_eq!(now - since, 7 * 86400);
    }

    #[test]
    fn test_event_data_serialization() {
        let event_data = serde_json::json!({
            "source": "slack",
            "items": 10
        });
        
        let serialized = serde_json::to_string(&event_data).unwrap();
        assert!(serialized.contains("slack"));
        assert!(serialized.contains("10"));
    }

    #[test]
    fn test_event_data_empty() {
        let event_data = serde_json::json!({});
        let serialized = serde_json::to_string(&event_data).unwrap_or_default();
        assert_eq!(serialized, "{}");
    }

    #[test]
    fn test_days_to_seconds_conversion() {
        let days = 30i32;
        let seconds = days as i64 * 86400;
        
        // 30 days = 2,592,000 seconds
        assert_eq!(seconds, 2_592_000);
    }

    #[test]
    fn test_empty_analytics_summary() {
        let summary = AnalyticsSummary {
            event_counts: std::collections::HashMap::new(),
            days: 1,
        };
        
        assert!(summary.event_counts.is_empty());
        assert_eq!(summary.days, 1);
    }
}
