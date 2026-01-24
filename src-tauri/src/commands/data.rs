use crate::AppState;
use crate::pipeline::PipelineState;
use super::types::{DataStats, ClearDataResult};
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;

/// Get the current pipeline status
#[tauri::command]
pub async fn get_pipeline_status(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<PipelineState, String> {
    let state = state.lock().await;
    let pipeline = state.pipeline.lock().await;
    Ok(pipeline.get_state().await)
}

/// Get database statistics
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

/// Clear all synced data (keeps credentials and preferences)
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

/// Factory reset - clears all data including credentials
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_stats_creation() {
        let stats = DataStats {
            content_items: 100,
            ai_summaries: 50,
            slack_users: 25,
            sync_states: 10,
        };
        
        assert_eq!(stats.content_items, 100);
        assert_eq!(stats.ai_summaries, 50);
        assert_eq!(stats.slack_users, 25);
        assert_eq!(stats.sync_states, 10);
    }

    #[test]
    fn test_clear_data_result() {
        let result = ClearDataResult {
            items_deleted: 150,
        };
        
        assert_eq!(result.items_deleted, 150);
    }

    #[test]
    fn test_total_deleted_calculation() {
        let content_count = 100i64;
        let summary_count = 50i64;
        let total = content_count + summary_count;
        
        assert_eq!(total, 150);
    }

    #[test]
    fn test_tables_to_clear() {
        // Verify we know all tables that should be cleared
        let tables_for_clear_synced = vec![
            "content_items",
            "ai_summaries", 
            "sync_state",
            "slack_users",
        ];
        
        let tables_for_factory_reset = [
            "content_items",
            "ai_summaries",
            "sync_state",
            "slack_users",
            "credentials",
            "slack_selected_channels",
            "preferences",
            "analytics",
        ];
        
        // Factory reset clears more tables
        assert!(tables_for_factory_reset.len() > tables_for_clear_synced.len());
        
        // All clear_synced tables should be in factory_reset
        for table in &tables_for_clear_synced {
            assert!(tables_for_factory_reset.contains(table));
        }
    }

    #[test]
    fn test_data_stats_serialization() {
        let stats = DataStats {
            content_items: 0,
            ai_summaries: 0,
            slack_users: 0,
            sync_states: 0,
        };
        
        let json = serde_json::to_string(&stats).unwrap();
        assert!(json.contains("contentItems"));
        assert!(json.contains("aiSummaries"));
    }
}
